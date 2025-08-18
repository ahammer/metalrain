use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::render::render_resource::*;
use bevy::render::Extract; // for Extract<Res<T>> in extraction systems
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::render_resource::ShaderType;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;
use wgpu::TexelCopyTextureInfo;

// High-level design: GPU Stable Fluids (semi-Lagrangian + Jacobi pressure projection) on a fixed grid.
// This first iteration aims for clarity over maximal performance.
// Steps per frame (in order): force_injection -> advect_velocity -> compute_divergence -> jacobi_pressure (N iters)
// -> project (subtract gradient) -> advect_dye. Then dye texture is displayed via a Material2d.

pub struct FluidSimPlugin;

impl Plugin for FluidSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidSimSettings>()
            .init_resource::<FluidBallInstances>()
            .add_plugins((Material2dPlugin::<FluidDisplayMaterial>::default(),))
            .add_systems(Startup, setup_fluid_sim)
            .add_systems(Startup, setup_fluid_display.after(setup_fluid_sim))
            .add_systems(Update, debug_fluid_once)
            .add_systems(Update, resize_display_quad)
            .add_systems(Update, (gather_ball_instances, update_sim_uniforms, input_force_position));
        // Display quad & compute dispatch systems will be added when GPU pipelines are implemented.

        // Add render-world compute dispatch after pipelines compile. We operate in RenderApp so we can
        // access GPU Image views and submit command buffers prior to draw sampling of dye texture.
        {
            let render_app = app.sub_app_mut(bevy::render::RenderApp);
            use bevy::render::RenderSet;
            render_app
                .init_resource::<FluidPipelines>()
                .add_systems(bevy::render::ExtractSchedule, (
                    extract_fluid_resources,
                    extract_fluid_gpu,
                ))
                .add_systems(bevy::render::ExtractSchedule, (
                    extract_fluid_settings,
                    extract_fluid_ball_instances,
                ))
        // Prepare pipelines & buffers; we also run compute in Prepare so the updated
        // dye texture is ready before render sampling.
                .add_systems(
                    bevy::render::Render,
                    (
                        prepare_fluid_pipelines.in_set(RenderSet::Prepare),
                        prepare_ball_gpu_buffer
                            .in_set(RenderSet::Prepare)
                            .after(prepare_fluid_pipelines),
            run_fluid_sim_compute
                .in_set(RenderSet::Prepare)
                .after(prepare_ball_gpu_buffer),
            ),
                );
        }
    }
}

// Extraction systems move main-world resources into render world each frame (simple clone / copy of handles)
// IMPORTANT: Use Extract<Res<T>> to access main-world resources during the ExtractSchedule. Using Option<Res<T>> here
// results in always missing resources because the system runs inside the render world and does not automatically pull from main world.
fn extract_fluid_resources(mut commands: Commands, src: Extract<Res<FluidSimResources>>) {
    info!("fluid: extract resources present initialized={}", src.initialized);
    commands.insert_resource(src.as_ref().clone());
}
fn extract_fluid_gpu(mut commands: Commands, src: Extract<Res<FluidSimGpu>>) {
    info!("fluid: extract gpu uniform frame={}", src.sim.frame);
    // Clone buffer handle & copy POD uniform struct
    commands.insert_resource(FluidSimGpu { uniform_buffer: src.uniform_buffer.clone(), sim: src.sim });
}

fn extract_fluid_settings(mut commands: Commands, src: Extract<Res<FluidSimSettings>>) {
    info!("fluid: extract settings resolution=({}x{})", src.resolution.x, src.resolution.y);
    commands.insert_resource(src.as_ref().clone());
}

// Extraction: copy ball injection buffer metadata & (later) staging buffer handle into render world.
fn extract_fluid_ball_instances(mut commands: Commands, src: Extract<Res<FluidBallInstances>>) {
    info!("fluid: extract ball instances count={}", src.count);
    commands.insert_resource(src.as_ref().clone());
}

/// Render-world GPU buffer for ball instances (storage buffer consumed by inject pass).
#[derive(Resource)]
pub struct FluidBallGpu {
    pub buffer: Buffer,
    pub capacity: usize, // number of instances capacity (not bytes)
}

fn prepare_ball_gpu_buffer(
    mut commands: Commands,
    mut existing: Option<ResMut<FluidBallGpu>>,
    balls: Option<Res<FluidBallInstances>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(balls) = balls else { return; };
    let needed = balls.items.len().min(FLUID_MAX_BALLS);
    let elem_size = std::mem::size_of::<FluidBallInstance>();
    let required_bytes = (needed.max(1) * elem_size) as u64; // at least 1 to avoid zero-sized buffer issues
    let mut recreate = false;
    if let Some(ref gpu) = existing { if gpu.capacity < needed { recreate = true; } }
    if existing.is_none() { recreate = true; }
    if recreate {
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("fluid-ball-buffer"),
            size: required_bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let gpu_res = FluidBallGpu { buffer, capacity: needed };
        // Write bytes immediately then insert
        if needed > 0 {
            let bytes = unsafe { std::slice::from_raw_parts(balls.items.as_ptr() as *const u8, needed * elem_size) };
            render_queue.write_buffer(&gpu_res.buffer, 0, bytes);
        }
        commands.insert_resource(gpu_res);
    } else if let Some(mut gpu) = existing { // reuse existing
        if needed > 0 {
            let bytes = unsafe { std::slice::from_raw_parts(balls.items.as_ptr() as *const u8, needed * elem_size) };
            render_queue.write_buffer(&gpu.buffer, 0, bytes);
        }
    }
}

#[derive(Resource, Clone)]
pub struct FluidSimSettings {
    pub resolution: UVec2,
    pub jacobi_iterations: u32,
    pub time_step: f32,
    pub dissipation: f32,
    pub velocity_dissipation: f32,
    pub dye_dissipation: f32,
    pub force_strength: f32,
}

// ---------------- Ball -> fluid injection data (main world) -----------------
// We gather per-frame ball state (position, velocity, radius, color) then upload to GPU as a
// storage buffer consumed by an inject compute pass. This keeps shader logic simple and avoids
// needing to sample many individual textures.
// Mapping assumptions: current world coordinate system roughly matches window pixel space.
// We approximate conversion to grid coordinates with a simple scaling (see gather_ball_instances).
// Future improvement: inject actual window size and camera transform to derive precise mapping.

/// Limit on number of balls injected into fluid (kept modest to bound buffer size).
pub const FLUID_MAX_BALLS: usize = 1024;

/// POD struct mirrored in WGSL for each ball injection instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct FluidBallInstance {
    /// World-space center (xy). We convert to grid coordinates in Rust before upload.
    pub grid_pos: Vec2,
    /// Ball velocity in world units (mapped to grid via same scale factor as position).
    pub grid_vel: Vec2,
    /// Radius in grid cells.
    pub radius: f32,
    /// Injection strength scale for velocity (could vary by ball mass / size); for now 1.0.
    pub vel_inject: f32,
    /// Packed color (linear RGB) used for dye injection; alpha unused.
    pub color: Vec4,
}

/// CPU-side collection of ball instances for current frame.
#[derive(Resource, Default, Clone)]
pub struct FluidBallInstances {
    pub count: usize,
    pub items: Vec<FluidBallInstance>,
}

impl Default for FluidSimSettings {
    fn default() -> Self {
    Self { resolution: UVec2::new(256, 256), jacobi_iterations: 20, time_step: 1.0/60.0, dissipation: 0.995, velocity_dissipation: 0.999, dye_dissipation: 0.9995, force_strength: 120.0 }
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
    pub inject_balls: CachedComputePipelineId,
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
    Self { layout: None, inject_balls: zero, add_force: zero, advect_velocity: zero, compute_divergence: zero, jacobi_pressure: zero, project_velocity: zero, advect_dye: zero }
    }
}

#[derive(Resource)]
pub struct FluidSimGpu {
    pub uniform_buffer: Buffer,
    pub sim: SimUniform,
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

// Matches WGSL SimUniform layout
#[repr(C)]
#[derive(Resource, Debug, Clone, ShaderType, Copy)]
pub struct SimUniform {
    pub grid_size: UVec2,
    pub inv_grid_size: Vec2,
    pub dt: f32,
    pub dissipation: f32,
    pub dye_dissipation: f32,
    pub vel_dissipation: f32,
    pub jacobi_alpha: f32,
    pub jacobi_beta: f32,
    pub force_pos: Vec2,
    pub force_radius: f32,
    pub force_strength: f32,
    pub ball_count: u32, // number of active FluidBallInstance entries (<= FLUID_MAX_BALLS)
    pub frame: u32,
    // Padding to 16-byte multiple (WGSL uniform structs require size multiple of 16). Original size was 60 bytes; add 4.
    pub _pad: u32,
}

impl Default for SimUniform {
    fn default() -> Self {
        Self {
            grid_size: UVec2::ZERO,
            inv_grid_size: Vec2::ZERO,
            dt: 1.0/60.0,
            dissipation: 0.995,
            dye_dissipation: 0.9995,
            vel_dissipation: 0.999,
            jacobi_alpha: 0.0,
            jacobi_beta: 1.0,
            force_pos: Vec2::new(128.0,128.0),
            force_radius: 32.0,
            force_strength: 150.0,
            ball_count: 0,
            frame: 0,
            _pad: 0,
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
    let scale = 18.0f32; // amplified tangential speed for visible motion
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
    sim_u.ball_count = 0;

    // Allocate buffer sized to SimUniform::min_size (includes required padding beyond Rust struct size)
    let raw_size = SimUniform::min_size().get();
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid-sim-uniform"),
        size: raw_size,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // SAFETY: SimUniform is plain-old-data for this prototype (only numeric types)
    let struct_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts((&sim_u as *const SimUniform) as *const u8, std::mem::size_of::<SimUniform>())
    };
    // Copy into padded vec sized to min_size
    let mut padded = vec![0u8; raw_size as usize];
    padded[..struct_bytes.len()].copy_from_slice(struct_bytes);
    render_queue.write_buffer(&buffer, 0, &padded);
    commands.insert_resource(FluidSimGpu { uniform_buffer: buffer, sim: sim_u });
}

// TODO: Build real compute pipelines & dispatch logic.
// Placeholder system to prove plugin wiring (logs once after startup when resource present)
fn debug_fluid_once(res: Option<Res<FluidSimResources>>) {
    static mut LOGGED: bool = false;
    if let Some(r) = res { if r.initialized {
        unsafe { if !LOGGED { info!("FluidSimResources initialized (debug stub)"); LOGGED = true; } }
    }}
}

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
    ball_gpu: Option<Res<FluidBallGpu>>,
    settings: Option<Res<FluidSimSettings>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    info!("fluid: compute system enter");
    if sim_res.is_none() || sim_gpu.is_none() {
        info!("fluid: early return - sim_res or sim_gpu missing (sim_res_present={} sim_gpu_present={})", sim_res.is_some(), sim_gpu.is_some());
        return;
    }
    let (mut sim_res, sim_gpu) = (sim_res.unwrap(), sim_gpu.unwrap());
    if pipelines.layout.is_none() {
        info!("fluid: early return - layout not created yet");
        return;
    }
    let layout = pipelines.layout.as_ref().unwrap();
    let required = [
        ("inject_balls", pipelines.inject_balls),
        ("add_force", pipelines.add_force),
        ("advect_velocity", pipelines.advect_velocity),
        ("compute_divergence", pipelines.compute_divergence),
        ("jacobi_pressure", pipelines.jacobi_pressure),
        ("project_velocity", pipelines.project_velocity),
        ("advect_dye", pipelines.advect_dye),
    ];
    let mut any_missing = false;
    for (name, id) in required.iter() {
        let ready = pipeline_cache.get_compute_pipeline(*id).is_some();
        if !ready { any_missing = true; }
        info!("fluid: pipeline status {} -> {}", name, if ready {"READY"} else {"LOADING"});
    }
    if any_missing {
        info!("fluid: compute skipped; pipelines not ready yet");
        return;
    }

    info!(
        "fluid: dispatch begin grid=({}x{}) balls={} dt={:.4} vel_diss={:.3} diss={:.3}",
        sim_gpu.sim.grid_size.x,
        sim_gpu.sim.grid_size.y,
        sim_gpu.sim.ball_count,
        sim_gpu.sim.dt,
        sim_gpu.sim.vel_dissipation,
        sim_gpu.sim.dissipation
    );

    // We keep the "A" textures stable for sampling by the material in the main world.
    // Each compute pass writes into the corresponding * _b texture, then we copy back into *_a.
    // This avoids needing to mutate material handles or store frame state across worlds.
    let vel_front = &sim_res.velocity_a; let vel_back = &sim_res.velocity_b;
    let pres_front = &sim_res.pressure_a; let pres_back = &sim_res.pressure_b;
    let dye_front = &sim_res.dye_a; let dye_back = &sim_res.dye_b;

    let get_view = |h: &Handle<Image>| -> Option<&TextureView> { gpu_images.get(h).map(|g| &g.texture_view) };
    let va = match get_view(vel_front) { Some(v) => v, None => { info!("fluid: missing velocity_a view"); return; } };
    let vb = match get_view(vel_back) { Some(v) => v, None => { info!("fluid: missing velocity_b view"); return; } };
    let pa = match get_view(pres_front) { Some(v) => v, None => { info!("fluid: missing pressure_a view"); return; } };
    let pb = match get_view(pres_back) { Some(v) => v, None => { info!("fluid: missing pressure_b view"); return; } };
    let div = match get_view(&sim_res.divergence) { Some(v) => v, None => { info!("fluid: missing divergence view"); return; } };
    let da = match get_view(dye_front) { Some(v) => v, None => { info!("fluid: missing dye_a view"); return; } };
    let db = match get_view(dye_back) { Some(v) => v, None => { info!("fluid: missing dye_b view"); return; } };

    let make_bg = |vel_in: &TextureView, vel_out: &TextureView,
                   dye_in: &TextureView, dye_out: &TextureView,
                   p_in: &TextureView, p_out: &TextureView,
                   divergence: &TextureView,
                   ball_buf: Option<&FluidBallGpu>| {
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
                // Storage buffer optional (can bind empty zero-sized buffer if None; here we skip binding by using a dummy if absent)
                BindGroupEntry { binding: 8, resource: if let Some(bb) = ball_buf { bb.buffer.as_entire_binding() } else { sim_gpu.uniform_buffer.as_entire_binding() } },
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
    let extent = Extent3d { width: grid.x, height: grid.y, depth_or_array_layers: 1 };

    // 0. Inject balls (velocity_a -> velocity_b & dye_a -> dye_b) then copy both back
    let ball_gpu_ref: Option<&FluidBallGpu> = ball_gpu.as_ref().map(|r| r.as_ref());
    let mut bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass inject_balls");
    run_pass(pipelines.inject_balls, &bg, "inject_balls", &mut encoder, grid);
    if let (Some(vb_tex), Some(va_tex)) = (gpu_images.get(vel_back), gpu_images.get(vel_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &vb_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &va_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }
    if let (Some(db_tex), Some(da_tex)) = (gpu_images.get(dye_back), gpu_images.get(dye_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &db_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &da_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }

    // 1. Add force (velocity_a -> velocity_b) then copy back into velocity_a
    bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass add_force");
    run_pass(pipelines.add_force, &bg, "add_force", &mut encoder, grid);
    if let (Some(vb_tex), Some(va_tex)) = (gpu_images.get(vel_back), gpu_images.get(vel_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &vb_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &va_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }
    // 2. Advect velocity -> copy back
    bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass advect_velocity");
    run_pass(pipelines.advect_velocity, &bg, "advect_velocity", &mut encoder, grid);
    if let (Some(vb_tex), Some(va_tex)) = (gpu_images.get(vel_back), gpu_images.get(vel_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &vb_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &va_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }
    // 3. Compute divergence
    bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass compute_divergence");
    run_pass(pipelines.compute_divergence, &bg, "compute_divergence", &mut encoder, grid);
    // 4. Jacobi pressure iterations with internal ping-pong
    let jacobi_iters = settings.map(|s| s.jacobi_iterations).unwrap_or(20).max(1);
    let mut ping_is_a = true; // read A write B first
    for iter in 0..jacobi_iters {
        let (p_in, p_out) = if ping_is_a { (pa, pb) } else { (pb, pa) };
        let jacobi_bg = make_bg(va, vb, da, db, p_in, p_out, div, ball_gpu_ref);
    info!("fluid: pass jacobi_pressure iter={iter}");
        run_pass(pipelines.jacobi_pressure, &jacobi_bg, "jacobi_pressure", &mut encoder, grid);
        ping_is_a = !ping_is_a;
    }
    if !ping_is_a { // final result resides in pressure_b -> copy once
        if let (Some(pb_tex), Some(pa_tex)) = (gpu_images.get(pres_back), gpu_images.get(pres_front)) {
            encoder.copy_texture_to_texture(
                TexelCopyTextureInfo { texture: &pb_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
                TexelCopyTextureInfo { texture: &pa_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
                extent,
            );
        }
    }
    // 5. Project velocity (velocity_a -> velocity_b) copy back
    bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass project_velocity");
    run_pass(pipelines.project_velocity, &bg, "project_velocity", &mut encoder, grid);
    if let (Some(vb_tex), Some(va_tex)) = (gpu_images.get(vel_back), gpu_images.get(vel_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &vb_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &va_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }
    // 6. Advect dye (dye_a -> dye_b) copy back
    bg = make_bg(va, vb, da, db, pa, pb, div, ball_gpu_ref);
    info!("fluid: pass advect_dye");
    run_pass(pipelines.advect_dye, &bg, "advect_dye", &mut encoder, grid);
    if let (Some(db_tex), Some(da_tex)) = (gpu_images.get(dye_back), gpu_images.get(dye_front)) {
        encoder.copy_texture_to_texture(
            TexelCopyTextureInfo { texture: &db_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            TexelCopyTextureInfo { texture: &da_tex.texture, mip_level: 0, origin: Origin3d::ZERO, aspect: TextureAspect::All },
            extent,
        );
    }

    // Submit all compute + copy work
    render_queue.submit(std::iter::once(encoder.finish()));
    info!("fluid: dispatch end");
}

// Render-world only: create compute pipelines once when layout available and ids still invalid
fn prepare_fluid_pipelines(
    mut pipelines: ResMut<FluidPipelines>,
    pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    // Create layout if missing
    if pipelines.layout.is_none() {
        let entries = [
            BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: Some(SimUniform::min_size()) }, count: None },
            BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 3, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::Rgba8Unorm, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 4, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba8Unorm, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 5, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadOnly, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 6, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            BindGroupLayoutEntry { binding: 7, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::ReadWrite, format: TextureFormat::R16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            // Ball injection storage buffer
            BindGroupLayoutEntry { binding: 8, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ];
        let layout = render_device.create_bind_group_layout(Some("fluid-sim-layout"), &entries);
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
    let entries: [(&'static str, *mut CachedComputePipelineId); 7] = [
        ("inject_balls", &mut pipelines.inject_balls as *mut _),
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
            info!("fluid: queued pipeline {}", name);
        }
    }
}

// ---------------- Update stage systems (main world) -----------------
fn update_sim_uniforms(
    gpu: Option<ResMut<FluidSimGpu>>,
    settings: Res<FluidSimSettings>,
    render_queue: Res<RenderQueue>,
) {
    let Some(mut gpu) = gpu else { return };
    // Sync simulation parameters from settings each frame
    gpu.sim.dt = settings.time_step.min(0.033);
    gpu.sim.dissipation = settings.dissipation;
    gpu.sim.vel_dissipation = settings.velocity_dissipation;
    gpu.sim.dye_dissipation = settings.dye_dissipation;
    gpu.sim.force_strength = settings.force_strength;
    gpu.sim.frame = gpu.sim.frame.wrapping_add(1);
    info!("fluid: uniforms updated dt={:.4} diss={:.3} dye_diss={:.4} v_diss={:.3} force_strength={:.1} frame={}", gpu.sim.dt, gpu.sim.dissipation, gpu.sim.dye_dissipation, gpu.sim.vel_dissipation, gpu.sim.force_strength, gpu.sim.frame);
    // Write entire uniform (small struct) to GPU
    unsafe {
        let struct_bytes = std::slice::from_raw_parts((&gpu.sim as *const SimUniform) as *const u8, std::mem::size_of::<SimUniform>());
        let min_size = SimUniform::min_size().get() as usize;
        let mut padded = vec![0u8; min_size];
        padded[..struct_bytes.len()].copy_from_slice(struct_bytes);
        render_queue.write_buffer(&gpu.uniform_buffer, 0, &padded);
    }
}

fn input_force_position(
    windows: Query<&Window>,
    cam_q: Query<(&Camera, &GlobalTransform)>,
    gpu: Option<ResMut<FluidSimGpu>>,
    settings: Res<FluidSimSettings>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    let Some(mut gpu) = gpu else { return };
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

// Gather current ball data into FluidBallInstances each frame (main world).
fn gather_ball_instances(
    mut inst: ResMut<FluidBallInstances>,
    gpu_uniform: Option<ResMut<FluidSimGpu>>,
    settings: Res<FluidSimSettings>,
    q_balls: Query<(&Transform, &crate::components::BallRadius, Option<&bevy_rapier2d::prelude::Velocity>, &crate::materials::BallMaterialIndex), With<crate::components::Ball>>,
) {
    inst.items.clear();
    let grid_w = settings.resolution.x as f32;
    let grid_h = settings.resolution.y as f32;
    // World coords range roughly with window size (camera default). Map world units to grid by translating origin and scaling by window to grid ratio.
    // For now assume 1 world unit ~ 1 pixel; rely on window dimensions matching world extents (-w/2..w/2). We approximate by using transform position scaled into grid.
    for (tf, radius, vel_opt, mat_idx) in q_balls.iter() {
        if inst.items.len() >= FLUID_MAX_BALLS { break; }
        let pos = tf.translation.truncate();
        // Map world position to grid (similar to cursor logic but inverse). We lack window size here; approximate by assuming world units already in pixel space.
        // TODO: pass window dimensions if mismatch appears.
        let gx = (pos.x / 800.0) * grid_w + grid_w * 0.5; // fallback 800 width assumption
        let gy = (pos.y / 600.0) * grid_h + grid_h * 0.5; // fallback 600 height assumption
        let vel = vel_opt.map(|v| v.linvel).unwrap_or(Vec2::ZERO);
        // Velocity mapping: scale similarly
        let gvx = vel.x / 800.0 * grid_w;
        let gvy = vel.y / 600.0 * grid_h;
        // Color from palette
        let color = crate::palette::color_for_index(mat_idx.0);
        let srgb = color.to_srgba();
        inst.items.push(FluidBallInstance {
            grid_pos: Vec2::new(gx, gy),
            grid_vel: Vec2::new(gvx, gvy),
            radius: radius.0, // radius in world units; treat as grid radius for now
            vel_inject: 1.0,
            color: Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0),
        });
    }
    inst.count = inst.items.len();
    if let Some(mut gpu) = gpu_uniform { gpu.sim.ball_count = inst.count as u32; }
}
