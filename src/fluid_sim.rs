use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::render_resource::ShaderType;
use bevy::render::RenderSet;
use bevy::render::Extract; // for manual resource extraction into render world
use std::sync::{Arc, atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering}};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;
// (Phase 3 completion) Removed need for TexelCopyTextureInfo after converting dye to true ping-pong.
use crate::config::GameConfig;
use crate::background::ActiveBackground;
use crate::fluid_impulses::{GpuImpulse, MAX_GPU_IMPULSES, FluidImpulseQueue};

// Feature-gated logging macro for verbose fluid sim pass/gating details.
// Enable with cargo feature `fluid_debug_passes` to elevate to info-level.
#[cfg(feature = "fluid_debug_passes")]
macro_rules! fluid_log { ($($t:tt)*) => { info!($($t)*); }; }
#[cfg(not(feature = "fluid_debug_passes"))]
macro_rules! fluid_log { ($($t:tt)*) => { trace!($($t)*); }; }

// Public so other modules (e.g. debug overlay) can inspect current state.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidSimStatus {
    Disabled,
    NotReadyResources,
    WaitingLayout,
    WaitingPipelines { ready: usize, total: usize },
    Running,
    // ---------------------------------------------------------------------------
    // Pass Graph Scaffolding (Phase 3 - initial step)
    // ---------------------------------------------------------------------------
    // We introduce a lightweight enumeration of logical fluid simulation passes.
    // This will allow the monolithic compute driver to be refactored into a data-
    // driven loop while preserving order. Later phases (bind group cache, optional
    // conditional passes, dynamic insertion) will build on this structure. For now
    // we mirror the existing hard-coded order and represent each Jacobi iteration
    // explicitly so diagnostics can attribute work to individual iterations if
    // desired.
}
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FluidPass {
        AddForce,
        AdvectVelocity,
        ComputeDivergence,
        Jacobi(u32), // iteration index (0-based)
        ProjectVelocity,
        AdvectDye,
    }
impl Default for FluidSimStatus { fn default() -> Self { FluidSimStatus::Disabled } }
    fn build_pass_graph(jacobi_iterations: u32) -> Vec<FluidPass> {
        let mut passes = Vec::with_capacity(6 + jacobi_iterations as usize);
        passes.push(FluidPass::AddForce);
        passes.push(FluidPass::AdvectVelocity);
        passes.push(FluidPass::ComputeDivergence);
        for i in 0..jacobi_iterations.max(1) { // ensure at least one iteration
            passes.push(FluidPass::Jacobi(i));
        }
        passes.push(FluidPass::ProjectVelocity);
        passes.push(FluidPass::AdvectDye);
        passes
    }

// High-level design: GPU Stable Fluids (semi-Lagrangian + Jacobi pressure projection) on a fixed grid.
// This first iteration aims for clarity over maximal performance.
// Steps per frame (in order): force_injection -> advect_velocity -> compute_divergence -> jacobi_pressure (N iters)
// -> project (subtract gradient) -> advect_dye. Then dye texture is displayed via a Material2d.

pub struct FluidSimPlugin;

impl Plugin for FluidSimPlugin {
    fn build(&self, app: &mut App) {
        info!("Building FluidSimPlugin (registering resources & systems)");
        app.init_resource::<FluidSimSettings>()
            .init_resource::<FluidSimDiagnostics>()
            .init_resource::<FluidSimStatus>()
            .add_plugins((Material2dPlugin::<FluidDisplayMaterial>::default(),))
            .add_systems(Startup, sync_fluid_settings_from_config)
            .add_systems(Startup, setup_fluid_sim)
            .add_systems(Startup, setup_fluid_display.after(setup_fluid_sim))
            .add_systems(Update, resize_display_quad)
            .add_systems(Update, (update_sim_uniforms, input_force_position, realloc_fluid_textures_if_needed, update_display_dye_handle, log_fluid_activity));
        // Display quad & compute dispatch systems will be added when GPU pipelines are implemented.

        // Add render-world compute dispatch after pipelines compile. We operate in RenderApp so we can
        // access GPU Image views and submit command buffers prior to draw sampling of dye texture.
        {
            let render_app = app.sub_app_mut(bevy::render::RenderApp);
            render_app
                .init_resource::<FluidPipelines>()
                .init_resource::<FluidSimStatus>()
                .init_resource::<FluidPingState>()
                .add_systems(
                    bevy::render::ExtractSchedule,
                    (
                        extract_fluid_resources,
                        extract_fluid_gpu,
                        extract_fluid_settings,
                        extract_active_background,
                        extract_fluid_diagnostics,
                    ),
                )
                // Place both preparation and compute in the Prepare set so extracted resources & images exist
                // and updates land before the actual Render set draws sample the dye texture.
                .add_systems(
                    bevy::render::Render,
                    (
                        prepare_fluid_pipelines.in_set(RenderSet::Prepare),
                        run_fluid_sim_compute
                            .in_set(RenderSet::Prepare)
                            .after(prepare_fluid_pipelines),
                    ),
                );
        }
    }
}

// Tracks which texture (A=0 / B=1) currently holds the front (read) data for each field.
// Phase 3 completion: dye now participates in ping-pong; copy-back removed.
#[derive(Resource, Debug, Clone, Copy)]
struct FluidPingState {
    velocity_front_is_a: bool,
    pressure_front_is_a: bool,
    dye_front_is_a: bool,
}

impl Default for FluidPingState {
    fn default() -> Self { Self { velocity_front_is_a: true, pressure_front_is_a: true, dye_front_is_a: true } }
}

fn sync_fluid_settings_from_config(
    cfg: Option<Res<GameConfig>>,
    mut settings: ResMut<FluidSimSettings>,
) {
    let Some(cfg) = cfg else { return; };
    // Only override if enabled (future: allow disabling plugin dynamically)
    let fc = &cfg.fluid_sim;
    settings.resolution = UVec2::new(fc.width.max(1), fc.height.max(1));
    settings.jacobi_iterations = fc.jacobi_iterations.max(1);
    settings.time_step = fc.time_step;
    settings.dissipation = fc.dissipation.clamp(0.0, 1.0);
    settings.velocity_dissipation = fc.velocity_dissipation.clamp(0.0, 1.0);
    settings.force_strength = fc.force_strength.max(0.0);
    settings.enabled = fc.enabled;
}

// Extraction systems move main-world resources into render world each frame (simple clone / copy of handles)
fn extract_fluid_resources(mut commands: Commands, src: Extract<Res<FluidSimResources>>) {
    commands.insert_resource(src.clone());
}
fn extract_fluid_gpu(mut commands: Commands, src: Extract<Res<FluidSimGpu>>) {
    commands.insert_resource(src.clone());
}

fn extract_fluid_settings(mut commands: Commands, src: Extract<Res<FluidSimSettings>>) {
    commands.insert_resource(src.clone());
}

fn extract_active_background(mut commands: Commands, src: Extract<Res<ActiveBackground>>) {
    commands.insert_resource(**src); // double-deref to get enum value
}

fn extract_fluid_diagnostics(mut commands: Commands, src: Extract<Res<FluidSimDiagnostics>>) {
    // Cloning shares underlying Arc so counters are unified across worlds.
    commands.insert_resource(src.clone());
}

#[derive(Resource, Clone)]
pub struct FluidSimSettings {
    pub resolution: UVec2,
    pub jacobi_iterations: u32,
    pub time_step: f32,
    pub dissipation: f32,
    pub velocity_dissipation: f32,
    pub force_strength: f32,
    pub enabled: bool,
}

impl Default for FluidSimSettings {
    fn default() -> Self {
        Self { resolution: UVec2::new(256, 256), jacobi_iterations: 20, time_step: 1.0/60.0, dissipation: 0.995, velocity_dissipation: 0.999, force_strength: 120.0, enabled: true }
    }
}

// GPU resources for the simulation images and pipelines (to be populated later)
#[derive(Resource, Clone)]
pub struct FluidSimResources {
    pub initialized: bool,
    pub velocity_a: Handle<Image>,
    pub velocity_b: Handle<Image>,
    pub pressure_a: Handle<Image>,
    pub pressure_b: Handle<Image>,
    pub divergence: Handle<Image>,
    pub dye_a: Handle<Image>,
    pub dye_b: Handle<Image>,
    // Future: pipeline handles / bind group layouts
}

impl FluidSimResources {
    fn new(
        velocity_a: Handle<Image>, velocity_b: Handle<Image>,
        pressure_a: Handle<Image>, pressure_b: Handle<Image>,
        divergence: Handle<Image>, dye_a: Handle<Image>, dye_b: Handle<Image>) -> Self {
    Self { initialized: true, velocity_a, velocity_b, pressure_a, pressure_b, divergence, dye_a, dye_b }
    }
}

// Holds compute pipelines once queued plus the shared bind group layout.
#[derive(Resource)]
pub struct FluidPipelines {
    pub layout: Option<BindGroupLayout>,
    pub add_force: CachedComputePipelineId,
    pub advect_velocity: CachedComputePipelineId,
    pub compute_divergence: CachedComputePipelineId,
    pub jacobi_pressure: CachedComputePipelineId,
    pub project_velocity: CachedComputePipelineId,
    pub advect_dye: CachedComputePipelineId,
}

impl Default for FluidPipelines {
    fn default() -> Self {
        // Use dummy zero ids until queued; these will be replaced.
        let zero = CachedComputePipelineId::INVALID;
        Self { layout: None, add_force: zero, advect_velocity: zero, compute_divergence: zero, jacobi_pressure: zero, project_velocity: zero, advect_dye: zero }
    }
}

#[derive(Resource, Clone)]
pub struct FluidSimGpu {
    pub uniform_buffer: Buffer,
    pub sim: SimUniform,
    // Phase 4 step 2: GPU impulse storage buffer & count uniform (not yet consumed by shader).
    pub impulse_buffer: Buffer,
    pub impulse_count_buffer: Buffer,
    pub impulse_capacity: usize,
}

// Diagnostics resource (main world copy extracted to render world not required). Tracks total dispatches & last frame id.
#[derive(Debug)]
struct FluidSimDiagnosticsInner {
    frames_with_dispatch: AtomicU64,
    total_workgroups: AtomicU64,
    last_grid_x: AtomicU32,
    last_grid_y: AtomicU32,
    first_dispatch_logged: AtomicBool,
    dye_front_is_a: AtomicBool,
    removed_dye_copies: AtomicU64,
}

impl Default for FluidSimDiagnosticsInner {
    fn default() -> Self {
        Self {
            frames_with_dispatch: AtomicU64::new(0),
            total_workgroups: AtomicU64::new(0),
            last_grid_x: AtomicU32::new(0),
            last_grid_y: AtomicU32::new(0),
            first_dispatch_logged: AtomicBool::new(false),
            dye_front_is_a: AtomicBool::new(true),
            removed_dye_copies: AtomicU64::new(0),
        }
    }
}

#[derive(Resource, Clone, Default, Debug)]
pub struct FluidSimDiagnostics {
    inner: Arc<FluidSimDiagnosticsInner>,
}

impl FluidSimDiagnostics {
    fn record_dispatch(&self, grid: UVec2, added_workgroups: u64, jacobi_iters: u64) {
        self.inner.frames_with_dispatch.fetch_add(1, Ordering::Relaxed);
        self.inner.total_workgroups.fetch_add(added_workgroups, Ordering::Relaxed);
        self.inner.last_grid_x.store(grid.x, Ordering::Relaxed);
        self.inner.last_grid_y.store(grid.y, Ordering::Relaxed);
        // Log first dispatch once globally (across worlds)
        if self.inner.first_dispatch_logged.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            let wg_x = (grid.x as u64 + 7) / 8; let wg_y = (grid.y as u64 + 7) / 8;
            info!(?grid, wg_x, wg_y, jacobi_iters, "Fluid sim first frame dispatched");
        }
    }
    fn frames(&self) -> u64 { self.inner.frames_with_dispatch.load(Ordering::Relaxed) }
    fn total_wg(&self) -> u64 { self.inner.total_workgroups.load(Ordering::Relaxed) }
    fn last_grid(&self) -> UVec2 { UVec2::new(self.inner.last_grid_x.load(Ordering::Relaxed), self.inner.last_grid_y.load(Ordering::Relaxed)) }
    fn first_logged(&self) -> bool { self.inner.first_dispatch_logged.load(Ordering::Relaxed) }
    fn dye_front_is_a(&self) -> bool { self.inner.dye_front_is_a.load(Ordering::Relaxed) }
    fn set_dye_front(&self, is_a: bool) { self.inner.dye_front_is_a.store(is_a, Ordering::Relaxed); }
    fn inc_removed_dye_copy(&self) { self.inner.removed_dye_copies.fetch_add(1, Ordering::Relaxed); }
    #[allow(dead_code)]
    fn removed_dye_copies(&self) -> u64 { self.inner.removed_dye_copies.load(Ordering::Relaxed) }
}

// Display handled via a fullscreen material (to be implemented); no sprite/quad yet.

// Fullscreen display material sampling dye texture (for now just dye_a)
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct FluidDisplayMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub dye: Handle<Image>,
}

impl Default for FluidDisplayMaterial {
    fn default() -> Self { Self { dye: Handle::default() } }
}

impl Material2d for FluidDisplayMaterial {
    fn fragment_shader() -> ShaderRef { "shaders/fluid_sim.wgsl".into() }
    fn vertex_shader() -> ShaderRef { "shaders/fluid_sim.wgsl".into() }
}

#[derive(Component)]
pub struct FluidDisplayQuad;

// Matches WGSL SimUniform layout. Force 16-byte alignment so size rounds to 64 bytes (std140 style).
#[repr(C, align(16))]
#[derive(Resource, Debug, Clone, ShaderType, Copy)]
pub struct SimUniform {
    pub grid_size: UVec2,
    pub inv_grid_size: Vec2,
    pub dt: f32,
    pub dissipation: f32,
    pub vel_dissipation: f32,
    pub jacobi_alpha: f32,
    pub jacobi_beta: f32,
    pub force_pos: Vec2,
    pub force_radius: f32,
    pub force_strength: f32,
}

impl Default for SimUniform {
    fn default() -> Self {
        Self {
            grid_size: UVec2::ZERO,
            inv_grid_size: Vec2::ZERO,
            dt: 1.0/60.0,
            dissipation: 0.995,
            vel_dissipation: 0.999,
            jacobi_alpha: 0.0,
            jacobi_beta: 1.0,
            force_pos: Vec2::new(128.0,128.0),
            force_radius: 32.0,
            force_strength: 150.0,
        }
    }
}

fn setup_fluid_sim(
    mut commands: Commands,
    settings: Res<FluidSimSettings>,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let size = Extent3d { width: settings.resolution.x, height: settings.resolution.y, depth_or_array_layers: 1 };
    let mut make_tex = |format: TextureFormat, usage: TextureUsages| -> Handle<Image> {
        let pixel_size = match format {
            TextureFormat::R16Float => 2,
            TextureFormat::Rgba16Float => 8,
            TextureFormat::Rgba8Unorm => 4,
            _ => 4,
        };
        let data_size = (size.width * size.height) as usize * pixel_size;
        let data = vec![0u8; data_size];
        let mut img = Image::default();
        img.data = Some(data);
        img.texture_descriptor = TextureDescriptor {
            label: Some("fluid-sim"),
            size,
            dimension: TextureDimension::D2,
            format,
            mip_level_count: 1,
            sample_count: 1,
            usage: usage | TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
            view_formats: &[],
        };
        images.add(img)
    };
    // Basic textures (more to be added / adjusted):
    // NOTE: velocity uses RG channels but allocated as RGBA16F to match shader's storage texture declaration (rgba16float)
    let velocity_a = make_tex(TextureFormat::Rgba16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let velocity_b = make_tex(TextureFormat::Rgba16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let pressure_a = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let pressure_b = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let divergence = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let dye_a = make_tex(TextureFormat::Rgba8Unorm, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let dye_b = make_tex(TextureFormat::Rgba8Unorm, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);

    info!("Fluid sim textures allocated ({}x{})", size.width, size.height);
    // Seed initial swirling velocity so simulation has motion before user input.
    // Pattern: angular swirl around center with radial falloff.
    {
        use half::f16;
        let w = size.width as i32; let h = size.height as i32;
        let mut swirl = vec![0u8; (w as usize * h as usize) * 8];
        let cx = (w as f32) * 0.5; let cy = (h as f32) * 0.5;
        let max_r = cx.min(cy);
        let scale = 5.0f32; // base tangential speed
        for y in 0..h { for x in 0..w {
            let dx = x as f32 - cx; let dy = y as f32 - cy;
            let r = (dx*dx + dy*dy).sqrt();
            if r < 1.0 { continue; }
            let falloff = (1.0 - (r / max_r)).clamp(0.0, 1.0);
            let inv_r = 1.0 / r;
            let tx = -dy * inv_r; let ty = dx * inv_r; // tangential
            let vx = tx * scale * falloff;
            let vy = ty * scale * falloff;
            let r16 = f16::from_f32(vx).to_le_bytes();
            let g16 = f16::from_f32(vy).to_le_bytes();
            let idx = ((y as usize) * size.width as usize + x as usize) * 8;
            swirl[idx] = r16[0]; swirl[idx+1] = r16[1];
            swirl[idx+2] = g16[0]; swirl[idx+3] = g16[1];
            // B,A remain 0
        }}
        if let Some(img) = images.get_mut(&velocity_a) { if let Some(data) = &mut img.data { data.copy_from_slice(&swirl); } }
        if let Some(img) = images.get_mut(&velocity_b) { if let Some(data) = &mut img.data { data.copy_from_slice(&swirl); } }
    }
    // Seed dye_a with random color blotches so motion is visible
    if let Some(img) = images.get_mut(&dye_a) {
        if let Some(data) = &mut img.data {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let w = size.width as usize; let h = size.height as usize;
            for _ in 0..(w*h/300) { // number of blotches
                let cx = rng.gen_range(0..w);
                let cy = rng.gen_range(0..h);
                let radius = rng.gen_range(6..24) as i32;
                let color = [rng.gen_range(80..255) as u8, rng.gen_range(50..200) as u8, rng.gen_range(50..220) as u8];
                for dy in -radius..=radius { for dx in -radius..=radius {
                    let nx = cx as i32 + dx; let ny = cy as i32 + dy;
                    if nx<0 || ny<0 || nx>=w as i32 || ny>=h as i32 { continue; }
                    let d2 = dx*dx + dy*dy; if d2 > radius*radius { continue; }
                    let idx = (ny as usize * w + nx as usize) * 4;
                    data[idx] = color[0]; data[idx+1] = color[1]; data[idx+2] = color[2]; data[idx+3] = 255;
                }}
            }
        }
    }

    let sim_res = FluidSimResources::new(velocity_a, velocity_b, pressure_a, pressure_b, divergence, dye_a, dye_b);
    commands.insert_resource(sim_res);

    // Layout & pipelines now created in render world; nothing to do here for layout.

    // Initialize uniform data
    let mut sim_u = SimUniform::default();
    sim_u.grid_size = settings.resolution;
    sim_u.inv_grid_size = Vec2::new(1.0 / settings.resolution.x as f32, 1.0 / settings.resolution.y as f32);
    // Jacobi coefficients alpha/beta for Poisson solve: alpha = -h^2, beta = 0.25 (if 4 neighbors)
    sim_u.jacobi_alpha = -1.0; // assuming h=1
    sim_u.jacobi_beta = 0.25;

    use std::mem::size_of;
    let raw_size = size_of::<SimUniform>() as u64;
    debug_assert!(raw_size <= 64, "SimUniform unexpectedly large: {}", raw_size);
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid-sim-uniform"),
        size: 64, // std140 rounded size
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // SAFETY: SimUniform is plain-old-data for this prototype (only numeric types)
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts((&sim_u as *const SimUniform) as *const u8, size_of::<SimUniform>())
    };
    render_queue.write_buffer(&buffer, 0, bytes);
    // Allocate impulse GPU buffers (storage + count). Not yet used by shader.
    let storage_size = (MAX_GPU_IMPULSES * std::mem::size_of::<GpuImpulse>()) as u64;
    let impulse_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid-impulses-storage"),
        size: storage_size,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let impulse_count_buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid-impulses-count"),
        size: 16, // u32 count + padding
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    commands.insert_resource(FluidSimGpu { uniform_buffer: buffer, sim: sim_u, impulse_buffer, impulse_count_buffer, impulse_capacity: MAX_GPU_IMPULSES });
}

// Removed placeholder debug system now that real compute passes are active.

// (Removed temporary sprite-based display; will add Material2d quad later)

fn setup_fluid_display(
    mut commands: Commands,
    res: Option<Res<FluidSimResources>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FluidDisplayMaterial>>,
) {
    let Some(res) = res else { return; };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));
    let material_handle = materials.add(FluidDisplayMaterial { dye: res.dye_a.clone() });
    commands.spawn((
        Mesh2d::from(mesh_handle),
        MeshMaterial2d(material_handle),
        Transform::from_xyz(0.0, 0.0, -90.0),
        FluidDisplayQuad,
    ));
}

fn resize_display_quad(
    windows: Query<&Window> ,
    mut q: Query<&mut Transform, With<FluidDisplayQuad>>,
) {
    // Fullscreen quad uses NDC sized (-1..1) mesh so no resize required; left for future if scaling changes
    if windows.is_empty() { return; }
    if let Ok(mut tf) = q.single_mut() { tf.translation.z = -90.0; }
}

// ---------------- Render-world compute dispatch -----------------
// Strategy: Avoid complicated cross-world ping-pong state by always writing into the *_b textures
// then copying results back into the *_a textures (which are the ones sampled for display). This is
// a little less efficient (extra copy passes) but keeps main-world handles static for now.
// Later optimization: true ping-pong with a custom display node referencing the latest destination.

#[allow(clippy::too_many_arguments)]
fn run_fluid_sim_compute(
    pipelines: Res<FluidPipelines>,
    pipeline_cache: Res<PipelineCache>,
    gpu_images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    sim_res: Option<ResMut<FluidSimResources>>,
    sim_gpu: Option<Res<FluidSimGpu>>,
    settings: Option<Res<FluidSimSettings>>,
    active_bg: Option<Res<ActiveBackground>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    diag: Option<Res<FluidSimDiagnostics>>,
    mut status: ResMut<FluidSimStatus>,
    mut ping: ResMut<FluidPingState>,
    impulses: Option<Res<FluidImpulseQueue>>,
) {
    trace!("ENTER run_fluid_sim_compute");
    if sim_res.is_none() || sim_gpu.is_none() {
        *status = FluidSimStatus::NotReadyResources;
        fluid_log!(have_sim_res = sim_res.is_some(), have_sim_gpu = sim_gpu.is_some(), "Fluid sim compute early-exit: resources not yet extracted");
        return;
    }
    let (sim_res, sim_gpu) = (sim_res.unwrap(), sim_gpu.unwrap());
    // Gate on enabled flag and active background selection
    if !fluid_sim_should_run(settings.as_deref(), active_bg.as_deref()) {
        *status = FluidSimStatus::Disabled;
        fluid_log!("Fluid sim compute gated off (disabled or background not active)");
        return;
    }
    // Log once when first active dispatch occurs
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| { fluid_log!("Fluid sim compute dispatch now active (background visible)"); });
    let Some(layout) = pipelines.layout.as_ref() else {
        *status = FluidSimStatus::WaitingLayout;
        fluid_log!("Fluid sim compute waiting: no bind group layout yet");
        return
    };
    let required = [
        pipelines.add_force,
        pipelines.advect_velocity,
        pipelines.compute_divergence,
        pipelines.jacobi_pressure,
        pipelines.project_velocity,
        pipelines.advect_dye,
    ];
    if required.iter().any(|id| pipeline_cache.get_compute_pipeline(*id).is_none()) {
        let ready = required.iter().filter(|id| pipeline_cache.get_compute_pipeline(**id).is_some()).count();
        *status = FluidSimStatus::WaitingPipelines { ready, total: required.len() };
        fluid_log!(ready, total = required.len(), "Fluid sim compute waiting: pipelines not all ready yet");
        return;
    }
    *status = FluidSimStatus::Running;

    // Phase 4 step 2: write impulse queue into GPU storage + count uniform (shader still unused).
    if let Some(queue) = impulses.as_ref() {
        // Pack up to capacity
        let mut packed: Vec<GpuImpulse> = Vec::with_capacity(queue.0.len().min(sim_gpu.impulse_capacity));
        for imp in queue.0.iter().take(sim_gpu.impulse_capacity) {
            let kind_code = match imp.kind { crate::fluid_impulses::FluidImpulseKind::SwirlFromVelocity => 0u32, crate::fluid_impulses::FluidImpulseKind::DirectionalVelocity => 1u32 };
            packed.push(GpuImpulse {
                pos: [imp.position.x, imp.position.y],
                radius: imp.radius,
                strength: imp.strength,
                dir: [imp.dir.x, imp.dir.y],
                kind: kind_code,
                _pad: 0,
            });
        }
        if !packed.is_empty() {
            // Safety: Pod
            let bytes: &[u8] = bytemuck::cast_slice(&packed);
            render_queue.write_buffer(&sim_gpu.impulse_buffer, 0, bytes);
        }
        // Write count (u32 + padding)
        let count_bytes: [u8;16] = {
            let c = packed.len() as u32;
            [c as u8, (c>>8) as u8, (c>>16) as u8, (c>>24) as u8, 0,0,0,0, 0,0,0,0, 0,0,0,0]
        };
        render_queue.write_buffer(&sim_gpu.impulse_count_buffer, 0, &count_bytes);
    }

    // Phase 3 completion: true ping-pong for velocity, pressure, and dye (no copy-back for dye).
    let (vel_front, vel_back) = front_back(ping.velocity_front_is_a, &sim_res.velocity_a, &sim_res.velocity_b);
    let (pres_front, pres_back) = front_back(ping.pressure_front_is_a, &sim_res.pressure_a, &sim_res.pressure_b);
    let (dye_front_handle, dye_back_handle) = front_back(ping.dye_front_is_a, &sim_res.dye_a, &sim_res.dye_b);

    let get_view = |h: &Handle<Image>| -> Option<&TextureView> { gpu_images.get(h).map(|g| &g.texture_view) };
    let va = match get_view(vel_front) { Some(v) => v, None => return };
    let vb = match get_view(vel_back) { Some(v) => v, None => return };
    let pa = match get_view(pres_front) { Some(v) => v, None => return };
    let pb = match get_view(pres_back) { Some(v) => v, None => return };
    let div = match get_view(&sim_res.divergence) { Some(v) => v, None => return };
    let da = match get_view(dye_front_handle) { Some(v) => v, None => return };
    let db = match get_view(dye_back_handle) { Some(v) => v, None => return };

    let make_bg = |vel_in: &TextureView, vel_out: &TextureView,
                   dye_in: &TextureView, dye_out: &TextureView,
                   p_in: &TextureView, p_out: &TextureView,
                   divergence: &TextureView| {
        render_device.create_bind_group(
            Some("fluid-sim-bind-group"),
            layout,
            &[
                BindGroupEntry { binding: 0, resource: sim_gpu.uniform_buffer.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(vel_in) },
                BindGroupEntry { binding: 2, resource: BindingResource::TextureView(vel_out) },
                BindGroupEntry { binding: 3, resource: BindingResource::TextureView(dye_in) },
                BindGroupEntry { binding: 4, resource: BindingResource::TextureView(dye_out) },
                BindGroupEntry { binding: 5, resource: BindingResource::TextureView(p_in) },
                BindGroupEntry { binding: 6, resource: BindingResource::TextureView(p_out) },
                BindGroupEntry { binding: 7, resource: BindingResource::TextureView(divergence) },
                BindGroupEntry { binding: 8, resource: sim_gpu.impulse_buffer.as_entire_binding() },
                BindGroupEntry { binding: 9, resource: sim_gpu.impulse_count_buffer.as_entire_binding() },
            ],
        )
    };

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor { label: Some("fluid-sim-encoder") });

    let run_pass = |pipeline_id: CachedComputePipelineId, bg: &BindGroup, label: &str, encoder: &mut CommandEncoder, grid: UVec2| {
        let pipeline = pipeline_cache.get_compute_pipeline(pipeline_id).unwrap();
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor { label: Some(label), timestamp_writes: None });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bg, &[]);
        let wg_x = (grid.x + 7) / 8;
        let wg_y = (grid.y + 7) / 8;
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    };

    // Helper for copying back (only when the pass wrote to back buffer)
    let grid = sim_gpu.sim.grid_size;
    // (extent removed; no dye copy pass)

    // True ping-pong for velocity: alternate read/write without copying back each time (same as previous logic)
    // vel_read/vel_write reflect the logical front/back; velocity_front_is_a mirrors ping state for local swaps
    let mut vel_read = if ping.velocity_front_is_a { va } else { vb };
    let mut vel_write = if ping.velocity_front_is_a { vb } else { va };
    let mut velocity_front_is_a = ping.velocity_front_is_a;
    let jacobi_iters_val = settings.as_ref().map(|s| s.jacobi_iterations).unwrap_or(20).max(1);
    let pass_list = build_pass_graph(jacobi_iters_val);
    let mut ping_is_a = true; // for pressure jacobi internal ping-pong

    for pass in pass_list.iter() {
        match *pass {
            FluidPass::AddForce => {
                let bg = make_bg(vel_read, vel_write, da, db, pa, pb, div);
                run_pass(pipelines.add_force, &bg, "add_force", &mut encoder, grid);
                std::mem::swap(&mut vel_read, &mut vel_write); velocity_front_is_a = !velocity_front_is_a;
            }
            FluidPass::AdvectVelocity => {
                let bg = make_bg(vel_read, vel_write, da, db, pa, pb, div);
                run_pass(pipelines.advect_velocity, &bg, "advect_velocity", &mut encoder, grid);
                std::mem::swap(&mut vel_read, &mut vel_write); velocity_front_is_a = !velocity_front_is_a;
            }
            FluidPass::ComputeDivergence => {
                let bg = make_bg(vel_read, vel_write, da, db, pa, pb, div);
                run_pass(pipelines.compute_divergence, &bg, "compute_divergence", &mut encoder, grid);
            }
            FluidPass::Jacobi(_i) => {
                let (p_in, p_out) = if ping_is_a == ping.pressure_front_is_a { (pa, pb) } else { (pb, pa) };
                let jacobi_bg = make_bg(va, vb, da, db, p_in, p_out, div);
                run_pass(pipelines.jacobi_pressure, &jacobi_bg, "jacobi_pressure", &mut encoder, grid);
                ping_is_a = !ping_is_a;
            }
            FluidPass::ProjectVelocity => {
                // After Jacobi loop, flip pressure front if jacobi concluded on alternate buffer (no copy needed)
                if ping_is_a == ping.pressure_front_is_a { // last iteration swapped ping_is_a, so final front differs
                    ping.pressure_front_is_a = !ping.pressure_front_is_a;
                }
                let bg = make_bg(vel_read, vel_write, da, db, pa, pb, div);
                run_pass(pipelines.project_velocity, &bg, "project_velocity", &mut encoder, grid);
                std::mem::swap(&mut vel_read, &mut vel_write); velocity_front_is_a = !velocity_front_is_a;
                ping.velocity_front_is_a = velocity_front_is_a;
            }
            FluidPass::AdvectDye => {
                let bg = make_bg(va, vb, da, db, pa, pb, div);
                run_pass(pipelines.advect_dye, &bg, "advect_dye", &mut encoder, grid);
                ping.dye_front_is_a = !ping.dye_front_is_a; // flip dye front
                if let Some(d) = &diag { d.set_dye_front(ping.dye_front_is_a); d.inc_removed_dye_copy(); }
            }
        }
    }

    // Submit all compute + copy work
    render_queue.submit(std::iter::once(encoder.finish()));
    if let Some(d) = diag {
        let wg_x = (grid.x as u64 + 7) / 8; let wg_y = (grid.y as u64 + 7) / 8;
    let jacobi_iters_u64 = jacobi_iters_val as u64;
    // Passes counted: add_force, advect_velocity, compute_divergence, jacobi*N, project_velocity, advect_dye
    // All copy passes eliminated; approx_passes metric unchanged for now (kept simple)
    let approx_passes = 4 + jacobi_iters_u64 + 2;
        d.record_dispatch(grid, wg_x * wg_y * approx_passes, jacobi_iters_u64);
    }
}

// Render-world only: create compute pipelines once when layout available and ids still invalid
fn prepare_fluid_pipelines(
    mut pipelines: ResMut<FluidPipelines>,
    pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    trace!("ENTER prepare_fluid_pipelines");
    // Create layout if missing
    if pipelines.layout.is_none() {
        let entries = [
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: Some(std::num::NonZeroU64::new(64).unwrap()) }, count: None },
            // NOTE: We intentionally do NOT rely on the derived ShaderType::min_size (which reports 56 B before tail rounding);
            // instead we use the actual size_of::<SimUniform>() = 64 to match WGSL std140 rounding rules (struct size multiple of 16).
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::Rgba8Unorm, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 4, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba8Unorm, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 5, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 6, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 7, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadWrite, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            // New: impulses storage buffer + count uniform (padding to 16 bytes). Shader will consume later.
            BindGroupLayoutEntry { binding: 8, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            BindGroupLayoutEntry { binding: 9, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: Some(std::num::NonZeroU64::new(16).unwrap()) }, count: None },
        ];
    let layout = render_device.create_bind_group_layout(Some("fluid-sim-layout"), &entries);
    debug!("Created fluid sim bind group layout");
    pipelines.layout = Some(layout);
    }
    if pipelines.layout.is_none() { return; }
    let already_ready = [
        pipelines.add_force,
        pipelines.advect_velocity,
        pipelines.compute_divergence,
        pipelines.jacobi_pressure,
        pipelines.project_velocity,
        pipelines.advect_dye,
    ].iter().all(|id| *id != CachedComputePipelineId::INVALID);
    if already_ready { return; }
    let shader_handle: Handle<Shader> = asset_server.load("shaders/fluid_sim.wgsl");
    let layout = pipelines.layout.as_ref().unwrap().clone();
    let entries: [(&'static str, *mut CachedComputePipelineId); 6] = [
        ("add_force", &mut pipelines.add_force as *mut _),
        ("advect_velocity", &mut pipelines.advect_velocity as *mut _),
        ("compute_divergence", &mut pipelines.compute_divergence as *mut _),
        ("jacobi_pressure", &mut pipelines.jacobi_pressure as *mut _),
        ("project_velocity", &mut pipelines.project_velocity as *mut _),
        ("advect_dye", &mut pipelines.advect_dye as *mut _),
    ];
    for (name, slot_ptr) in entries {
        // SAFETY: slot_ptr points to fields of mutable pipelines struct; unique in list
        let slot = unsafe { &mut *slot_ptr };
        if *slot == CachedComputePipelineId::INVALID {
            *slot = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(format!("fluid_{}", name).into()),
                layout: vec![layout.clone()],
                push_constant_ranges: vec![],
                shader: shader_handle.clone(),
                shader_defs: vec![],
                entry_point: name.into(),
                zero_initialize_workgroup_memory: false,
            });
            info!(?name, "Queued fluid sim compute pipeline");
        }
    }
}

// Helper used for gating logic (unit-testable)
fn fluid_sim_should_run(settings: Option<&FluidSimSettings>, bg: Option<&ActiveBackground>) -> bool {
    if let Some(s) = settings { if !s.enabled { return false; } }
    if let Some(b) = bg { if *b != ActiveBackground::FluidSim { return false; } }
    true
}

// ---------------- Update stage systems (main world) -----------------
fn update_sim_uniforms(
    gpu: Option<ResMut<FluidSimGpu>>,
    settings: Res<FluidSimSettings>,
    render_queue: Res<RenderQueue>,
) {
    let Some(mut gpu) = gpu else { return };
    if !settings.enabled { return; }
    // Sync simulation parameters from settings each frame
    gpu.sim.dt = settings.time_step.min(0.033);
    gpu.sim.dissipation = settings.dissipation;
    gpu.sim.vel_dissipation = settings.velocity_dissipation;
    gpu.sim.force_strength = settings.force_strength;
    // Write entire uniform (small struct) to GPU
    unsafe {
        let bytes = std::slice::from_raw_parts((&gpu.sim as *const SimUniform) as *const u8, std::mem::size_of::<SimUniform>());
        render_queue.write_buffer(&gpu.uniform_buffer, 0, bytes);
    }
}

// Main-world logging of diagnostic counters every few frames (early during 5s auto-close window)
fn log_fluid_activity(diag: Option<Res<FluidSimDiagnostics>>, time: Res<Time>) {
    let Some(diag) = diag else { return; };
    if diag.frames() == 0 {
        if time.elapsed_secs() > 0.5 { warn!("No fluid sim dispatches in first 0.5s"); }
        return;
    }
    let t = time.elapsed_secs_f64();
    if (t < 0.6 && t > 0.5) || (t < 2.1 && t > 2.0) {
        info!(frames = diag.frames(), total_workgroups = diag.total_wg(), grid = ?diag.last_grid(), first_logged = diag.first_logged(), "Fluid sim activity snapshot");
    }
}

// Update display material dye handle to current front (set by render-world ping state)
fn update_display_dye_handle(
    diag: Option<Res<FluidSimDiagnostics>>,
    sim_res: Option<Res<FluidSimResources>>,
    mut materials: ResMut<Assets<FluidDisplayMaterial>>,
    q_display: Query<&MeshMaterial2d<FluidDisplayMaterial>, With<FluidDisplayQuad>>,
) {
    let (diag, sim_res) = match (diag, sim_res) { (Some(d), Some(r)) => (d, r), _ => return };
    let Ok(handle) = q_display.get_single() else { return; };
    let Some(mat) = materials.get_mut(&handle.0) else { return; };
    let is_a = diag.dye_front_is_a();
    let desired = if is_a { &sim_res.dye_a } else { &sim_res.dye_b };
    if mat.dye != *desired { mat.dye = desired.clone(); }
}

fn front_back<'a>(front_is_a: bool, a: &'a Handle<Image>, b: &'a Handle<Image>) -> (&'a Handle<Image>, &'a Handle<Image>) {
    if front_is_a { (a, b) } else { (b, a) }
}

fn input_force_position(
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    gpu: Option<ResMut<FluidSimGpu>>,
    settings: Res<FluidSimSettings>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    let Some(mut gpu) = gpu else { return };
    if !settings.enabled { return; }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let (camera, cam_tf) = match cam_q.iter().next() { Some(v) => v, None => return };
    if !buttons.pressed(MouseButton::Left) { return; }
    // Convert screen to world then to grid coordinate. World coordinates center at (0,0) with height ~ window.height()
    if let Ok(ray) = camera.viewport_to_world(cam_tf, cursor) {
        let origin = ray.origin.truncate();
        // Map world space (-w/2..w/2, -h/2..h/2) to grid (0..width, 0..height)
        let w = window.width();
        let h = window.height();
        let gx = (origin.x / w * settings.resolution.x as f32) + settings.resolution.x as f32 * 0.5;
        let gy = (origin.y / h * settings.resolution.y as f32) + settings.resolution.y as f32 * 0.5;
        gpu.sim.force_pos = Vec2::new(gx.clamp(0.0, settings.resolution.x as f32 - 1.0), gy.clamp(0.0, settings.resolution.y as f32 - 1.0));
    }
}

// Reallocate textures automatically if resolution changed at runtime (hot reload)
fn realloc_fluid_textures_if_needed(
    mut sim_res: Option<ResMut<FluidSimResources>>,
    settings: Res<FluidSimSettings>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FluidDisplayMaterial>>,
    q_display: Query<&MeshMaterial2d<FluidDisplayMaterial>, With<FluidDisplayQuad>>,
) {
    let Some(ref mut sim_res) = sim_res else { return; };
    // Inspect one image (velocity_a) to check current size
    let need_realloc = if let Some(img) = images.get(&sim_res.velocity_a) {
        img.texture_descriptor.size.width != settings.resolution.x || img.texture_descriptor.size.height != settings.resolution.y
    } else { true };
    if !need_realloc { return; }
    // Allocate new textures similar to setup_fluid_sim (simplified: zero-filled; skip swirl & dye seeding for now)
    let size = Extent3d { width: settings.resolution.x, height: settings.resolution.y, depth_or_array_layers: 1 };
    let mut make_tex = |format: TextureFormat, usage: TextureUsages| -> Handle<Image> {
        let pixel_size = match format { TextureFormat::R16Float => 2, TextureFormat::Rgba16Float => 8, TextureFormat::Rgba8Unorm => 4, _ => 4 };
        let data_size = (size.width * size.height) as usize * pixel_size;
        let mut img = Image::default();
        img.data = Some(vec![0u8; data_size]);
        img.texture_descriptor = TextureDescriptor {
            label: Some("fluid-sim-realloc"), size, dimension: TextureDimension::D2, format,
            mip_level_count: 1, sample_count: 1, usage: usage | TextureUsages::COPY_SRC | TextureUsages::COPY_DST, view_formats: &[] };
        images.add(img)
    };
    let velocity_a = make_tex(TextureFormat::Rgba16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let velocity_b = make_tex(TextureFormat::Rgba16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let pressure_a = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let pressure_b = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let divergence  = make_tex(TextureFormat::R16Float, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let dye_a = make_tex(TextureFormat::Rgba8Unorm, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    let dye_b = make_tex(TextureFormat::Rgba8Unorm, TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING);
    // Replace inner resource data
    **sim_res = FluidSimResources::new(velocity_a.clone(), velocity_b, pressure_a, pressure_b, divergence, dye_a.clone(), dye_b);
    // Update display material
    if let Ok(handle) = q_display.single() { if let Some(mat) = materials.get_mut(&handle.0) { mat.dye = dye_a; } }
    info!(?size, "Fluid sim textures reallocated for new resolution");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gating_logic_variants() {
        let settings_enabled = FluidSimSettings { enabled: true, ..Default::default() };
        let settings_disabled = FluidSimSettings { enabled: false, ..Default::default() };
        assert!(fluid_sim_should_run(Some(&settings_enabled), Some(&ActiveBackground::FluidSim)));
        assert!(!fluid_sim_should_run(Some(&settings_disabled), Some(&ActiveBackground::FluidSim)));
        assert!(!fluid_sim_should_run(Some(&settings_enabled), Some(&ActiveBackground::Grid)));
        assert!(fluid_sim_should_run(Some(&settings_enabled), None)); // if no background info, allow (defensive)
    }

    #[test]
    fn sim_uniform_size_is_64() {
        use std::mem::size_of;
        assert_eq!(size_of::<SimUniform>(), 64, "SimUniform must remain 64 bytes to match WGSL layout");
    }
}
