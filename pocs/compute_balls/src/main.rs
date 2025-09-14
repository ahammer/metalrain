//! Animated metaballs rendered via a compute shader into a storage texture.
//! Patterned after Bevy's `compute_shader_game_of_life` example but specialized
//! for distance-field metaballs + surface shading.

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
    sprite::{Material2d, Material2dPlugin, MeshMaterial2d},
};
use std::borrow::Cow;

// Shader asset path (relative to this crate's assets/ directory)
const SHADER_ASSET_PATH: &str = "shaders/compute_metaballs.wgsl";

// Output texture size (keep divisible by WORKGROUP_SIZE if you want exact fit)
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const DISPLAY_SCALE: f32 = 2.0;
const WORKGROUP_SIZE: u32 = 8;
const MAX_BALLS: usize = 5;

// CPU-side definition must match WGSL struct `Ball`
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Ball {
    center: [f32; 2],
    radius: f32,
    _pad: f32,
}

#[derive(Resource, Clone, ExtractResource)]
struct BallBuffer {
    balls: Vec<Ball>,
}

#[repr(C)]
#[derive(Resource, Clone, ExtractResource, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TimeUniform {
    time: f32,
    // Pad to 16 bytes for uniform buffer alignment
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Resource, Clone, ExtractResource, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ParamsUniform {
    screen_size: [f32; 2],
    num_balls: u32,
    debug_mode: u32, // 0: shaded, 1: field grayscale, 2: normals, 3: iso bands, 4: gradient dir
    iso: f32,
    ambient: f32,
    rim_power: f32,
    show_centers: u32, // 0/1 toggle
}

#[derive(Resource)]
struct GpuMetaballPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline_id: CachedComputePipelineId,
}

#[derive(Resource)]
struct GpuMetaballBindGroup(BindGroup);

#[derive(Resource, Clone, ExtractResource)]
struct MetaballTarget {
    texture: Handle<Image>,
}

struct MetaballComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct MetaballPassLabel;

impl Plugin for MetaballComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<MetaballTarget>::default())
            .add_plugins(ExtractResourcePlugin::<BallBuffer>::default())
            .add_plugins(ExtractResourcePlugin::<TimeUniform>::default())
            .add_plugins(ExtractResourcePlugin::<ParamsUniform>::default());

        let render_app = app.sub_app_mut(RenderApp);
        // Order matters: bind group creation depends on GPU buffers created earlier the same frame.
        render_app.add_systems(
            Render,
            (
                prepare_buffers,
                prepare_bind_group.after(prepare_buffers),
            )
                .in_set(RenderSet::PrepareBindGroups),
        );

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(MetaballPassLabel, MetaballComputeNode::default());
        graph.add_node_edge(MetaballPassLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GpuMetaballPipeline>();
    }
}

impl FromWorld for GpuMetaballPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        // Layout: storage texture (write), params uniform, time uniform, ball storage buffer
        let layout = device.create_bind_group_layout(
            Some("metaballs.bind_group_layout"),
            &[
                // storage texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // params uniform
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<ParamsUniform>() as u64),
                    },
                    count: None,
                },
                // time uniform
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<TimeUniform>() as u64),
                    },
                    count: None,
                },
                // balls storage buffer
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new((std::mem::size_of::<Ball>() * MAX_BALLS) as u64),
                    },
                    count: None,
                },
            ],
        );

        let shader = world.load_asset(SHADER_ASSET_PATH);
        let cache = world.resource::<PipelineCache>();
        let pipeline_id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::Borrowed("metaballs.compute")),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader,
            shader_defs: vec![],
            entry_point: Cow::Borrowed("metaballs"),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            bind_group_layout: layout,
            pipeline_id,
        }
    }
}

#[derive(Resource)]
struct GpuBuffers {
    params: Buffer,
    time: Buffer,
    balls: Buffer,
}

fn prepare_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    params: Res<ParamsUniform>,
    time_uni: Res<TimeUniform>,
    balls: Res<BallBuffer>,
) {
    // Create (or recreate) GPU buffers each frame (simple for a POC).
    let params_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.params"),
        contents: bytemuck::bytes_of(&*params),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let time_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.time"),
        contents: bytemuck::bytes_of(&*time_uni),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    // Pad / clamp balls vector to MAX_BALLS
    let mut fixed = [Ball {
        center: [0.0, 0.0],
        radius: 0.0,
        _pad: 0.0,
    }; MAX_BALLS];
    for (i, b) in balls.balls.iter().take(MAX_BALLS).enumerate() {
        fixed[i] = *b;
    }
    let balls_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.balls"),
        contents: bytemuck::cast_slice(&fixed),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    commands.insert_resource(GpuBuffers {
        params: params_buf,
        time: time_buf,
        balls: balls_buf,
    });
}

use bevy::render::render_asset::RenderAssets;
use bevy::render::texture::GpuImage;

fn prepare_bind_group(
    mut commands: Commands,
    target: Res<MetaballTarget>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    pipeline: Res<GpuMetaballPipeline>,
    gpu_buffers: Option<Res<GpuBuffers>>, // tolerate missing first frame
    render_device: Res<RenderDevice>,
) {
    let Some(gpu_buffers) = gpu_buffers else { return; };
    let Some(gpu_image) = gpu_images.get(&target.texture) else { return; };
    let view = &gpu_image.texture_view;

    let bind_group = render_device.create_bind_group(
        Some("metaballs.bind_group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry { binding: 0, resource: BindingResource::TextureView(view) },
            BindGroupEntry { binding: 1, resource: gpu_buffers.params.as_entire_binding() },
            BindGroupEntry { binding: 2, resource: gpu_buffers.time.as_entire_binding() },
            BindGroupEntry { binding: 3, resource: gpu_buffers.balls.as_entire_binding() },
        ],
    );

    commands.insert_resource(GpuMetaballBindGroup(bind_group));
}

enum MetaballNodeState {
    Loading,
    Ready,
}

struct MetaballComputeNode {
    state: MetaballNodeState,
}

impl Default for MetaballComputeNode {
    fn default() -> Self {
        Self {
            state: MetaballNodeState::Loading,
        }
    }
}

impl render_graph::Node for MetaballComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GpuMetaballPipeline>();
        let cache = world.resource::<PipelineCache>();
        if let MetaballNodeState::Loading = self.state {
            match cache.get_compute_pipeline_state(pipeline.pipeline_id) {
                CachedPipelineState::Ok(_) => self.state = MetaballNodeState::Ready,
                CachedPipelineState::Err(err) => panic!("Failed to compile metaballs compute:\n{err}"),
                _ => {}
            }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !matches!(self.state, MetaballNodeState::Ready) {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GpuMetaballPipeline>();
        let bind_group = &world.resource::<GpuMetaballBindGroup>().0;

        let gpu_pipeline = pipeline_cache
            .get_compute_pipeline(pipeline.pipeline_id)
            .expect("pipeline ready");

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(gpu_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        let gx = (WIDTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let gy = (HEIGHT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        pass.dispatch_workgroups(gx, gy, 1);

        Ok(())
    }
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Output texture
    let mut image = Image::new_fill(
        Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let handle = images.add(image);

    // The sprite-based presentation is replaced by a custom fullscreen material (see PresentMaterial).
    commands.spawn(Camera2d);

    // Initial balls arranged in an ellipse
    let balls = (0..12)
        .map(|i| {
            let t = i as f32 / 12.0 * std::f32::consts::TAU;
            Ball { center: [WIDTH as f32 * 0.5 + t.cos() * 120.0, HEIGHT as f32 * 0.5 + t.sin() * 80.0], radius: 40.0 + (i as f32 % 3.0) * 10.0, _pad: 0.0 }
        })
        .collect();

    commands.insert_resource(MetaballTarget { texture: handle });
    commands.insert_resource(BallBuffer { balls });
    commands.insert_resource(TimeUniform { time: 0.0, _pad: [0.0; 3] });
    commands.insert_resource(ParamsUniform { screen_size: [WIDTH as f32, HEIGHT as f32], num_balls: 12, debug_mode: 0, iso: 2.2, ambient: 0.25, rim_power: 2.5, show_centers: 1 });
}

fn animate_balls(time: Res<Time>, mut bufs: ResMut<BallBuffer>, params: Res<ParamsUniform>, mut time_u: ResMut<TimeUniform>) {
    time_u.time += time.delta_secs();
    let t = time_u.time;
    let n = params.num_balls.min(bufs.balls.len() as u32) as usize;
    for (i, b) in bufs.balls.iter_mut().take(n).enumerate() {
        let phase = i as f32 * 0.37;
        b.center[0] += (t * 0.9 + phase).sin() * 0.3;
        b.center[1] += (t * 0.7 + phase * 1.7).cos() * 0.25;
    }
}

fn debug_input(keys: Res<ButtonInput<KeyCode>>, mut params: ResMut<ParamsUniform>) {
    if keys.just_pressed(KeyCode::KeyF) {
        params.debug_mode = (params.debug_mode + 1) % 5; // cycle 0..4
        info!("Switched debug mode to {}", params.debug_mode);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        params.show_centers = 1 - params.show_centers;
        info!("Centers {}", if params.show_centers == 1 { "ON" } else { "OFF" });
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin { primary_window: Some(Window { title: "Compute Metaballs".into(), resolution: (WIDTH as f32 * DISPLAY_SCALE, HEIGHT as f32 * DISPLAY_SCALE).into(), ..default() }), ..default() })
                .set(ImagePlugin::default_nearest()),
        )
        // Register our fullscreen present material
        .add_plugins(Material2dPlugin::<PresentMaterial>::default())
        .add_plugins(MetaballComputePlugin)
    .add_systems(Startup, setup)
    .add_systems(PostStartup, setup_present)
    .add_systems(Update, (animate_balls, debug_input))
        .run();
}

// ---------------- Fullscreen Present Material ----------------
// We expose the compute output texture via a simple Material2d implementation.
// This gives us an explicit vertex + fragment shader pair the user requested
// instead of relying on Bevy's internal sprite material pipeline.

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct PresentMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

impl Material2d for PresentMaterial {
    fn fragment_shader() -> ShaderRef { "shaders/present_fullscreen.wgsl".into() }
}

// Extend setup to also spawn the fullscreen quad with PresentMaterial.
// We append a separate system to avoid large diff in original setup.
fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PresentMaterial>>,
    target: Res<MetaballTarget>,
    mut commands: Commands,
) {
    // Use a rectangle mesh with default generated UVs; default vertex shader will map uv.
    let quad = Mesh::from(Rectangle::new(WIDTH as f32, HEIGHT as f32));
    let quad_handle = meshes.add(quad);
    let material_handle = materials.add(PresentMaterial { texture: target.texture.clone() });
    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::splat(DISPLAY_SCALE)),
    ));
}

// Add the new setup system (after compute target is created in original setup).
// (helper removed; system registered directly in main())

