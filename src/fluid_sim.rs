use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::render_resource::ShaderType;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;

// High-level design: GPU Stable Fluids (semi-Lagrangian + Jacobi pressure projection) on a fixed grid.
// This first iteration aims for clarity over maximal performance.
// Steps per frame (in order): force_injection -> advect_velocity -> compute_divergence -> jacobi_pressure (N iters)
// -> project (subtract gradient) -> advect_dye. Then dye texture is displayed via a Material2d.

pub struct FluidSimPlugin;

impl Plugin for FluidSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidSimSettings>()
            .init_resource::<FluidPipelines>()
            .add_plugins((Material2dPlugin::<FluidDisplayMaterial>::default(),))
            .add_systems(Startup, setup_fluid_sim)
            .add_systems(Startup, setup_fluid_display.after(setup_fluid_sim))
            .add_systems(Update, debug_fluid_once)
            .add_systems(Update, resize_display_quad);
        // Display quad & compute dispatch systems will be added when GPU pipelines are implemented.
    }
}

#[derive(Resource, Clone)]
pub struct FluidSimSettings {
    pub resolution: UVec2,
    pub jacobi_iterations: u32,
    pub time_step: f32,
    pub dissipation: f32,
    pub velocity_dissipation: f32,
    pub force_strength: f32,
}

impl Default for FluidSimSettings {
    fn default() -> Self {
        Self { resolution: UVec2::new(256, 256), jacobi_iterations: 20, time_step: 1.0/60.0, dissipation: 0.995, velocity_dissipation: 0.999, force_strength: 120.0 }
    }
}

// GPU resources for the simulation images and pipelines (to be populated later)
#[derive(Resource)]
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
struct FluidDisplayQuad;

// Matches WGSL SimUniform layout
#[repr(C)]
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
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
    pipeline_cache: ResMut<PipelineCache>,
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

    let sim_res = FluidSimResources::new(velocity_a, velocity_b, pressure_a, pressure_b, divergence, dye_a, dye_b);
    commands.insert_resource(sim_res);

    // Create bind group layout for group(0) matching shader bindings
    let entries = [
        // binding 0: uniform buffer
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(SimUniform::min_size()),
            },
            count: None,
        },
        // velocity_in (read)
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::Rgba16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // velocity_out (write)
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // scalar_a (read dye)
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::Rgba8Unorm,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // scalar_b (write dye)
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba8Unorm,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // pressure_in (read)
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::R16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // pressure_out (write)
        BindGroupLayoutEntry {
            binding: 6,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // divergence (read_write)
        BindGroupLayoutEntry {
            binding: 7,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadWrite,
                format: TextureFormat::R16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
    ];
    let layout = render_device.create_bind_group_layout(Some("fluid-sim-layout"), &entries);

    let shader_handle: Handle<Shader> = asset_server.load("shaders/fluid_sim.wgsl");
    // Queue pipelines for each entry point
    let mut fp = FluidPipelines::default();
    fp.layout = Some(layout.clone());
    let entries = [
        ("add_force", &mut fp.add_force),
        ("advect_velocity", &mut fp.advect_velocity),
        ("compute_divergence", &mut fp.compute_divergence),
        ("jacobi_pressure", &mut fp.jacobi_pressure),
        ("project_velocity", &mut fp.project_velocity),
        ("advect_dye", &mut fp.advect_dye),
    ];
    for (entry, slot) in entries.into_iter() {
        *slot = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(format!("fluid_{}", entry).into()),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader: shader_handle.clone(),
            shader_defs: vec![],
            entry_point: entry.into(),
            zero_initialize_workgroup_memory: false,
        });
    }
    commands.insert_resource(fp);

    // Initialize uniform data
    let mut sim_u = SimUniform::default();
    sim_u.grid_size = settings.resolution;
    sim_u.inv_grid_size = Vec2::new(1.0 / settings.resolution.x as f32, 1.0 / settings.resolution.y as f32);
    // Jacobi coefficients alpha/beta for Poisson solve: alpha = -h^2, beta = 0.25 (if 4 neighbors)
    sim_u.jacobi_alpha = -1.0; // assuming h=1
    sim_u.jacobi_beta = 0.25;

    use std::mem::size_of;
    let raw_size = size_of::<SimUniform>() as u64;
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("fluid-sim-uniform"),
        size: raw_size,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // SAFETY: SimUniform is plain-old-data for this prototype (only numeric types)
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts((&sim_u as *const SimUniform) as *const u8, size_of::<SimUniform>())
    };
    render_queue.write_buffer(&buffer, 0, bytes);
    commands.insert_resource(FluidSimGpu { uniform_buffer: buffer, sim: sim_u });
}

// TODO: Build real compute pipelines & dispatch logic.
// Placeholder system to prove plugin wiring (logs once after startup when resource present)
fn debug_fluid_once(res: Option<Res<FluidSimResources>>) {
    if let Some(r) = res { if r.initialized { info!("FluidSimResources initialized (debug stub)"); } }
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
