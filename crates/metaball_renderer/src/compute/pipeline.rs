use super::types::*;
use crate::internal::{
    AlbedoTexture, BallBuffer, BallGpu, FieldTexture, NormalTexture, ParamsUniform, TimeUniform,
    WORKGROUP_SIZE,
};
use crate::settings::MetaballRenderSettings;
use bevy::asset::Assets;
use bevy::prelude::*;
use bevy::render::{
    extract_resource::ExtractResourcePlugin,
    render_asset::RenderAssets,
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
    Render, RenderApp, RenderSet,
};
use game_assets::GameAssets;
use std::borrow::Cow; // centralized shader handles (readiness inferred via load state)

#[cfg(feature = "embed_shaders")]
const COMPUTE_METABALLS_WGSL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/shaders/compute_metaballs.wgsl"
));

pub struct ComputeMetaballsPlugin;

#[derive(Resource)]
pub struct GpuMetaballPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
    pub shader_handle: Handle<Shader>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MetaballPassLabel;

impl Plugin for ComputeMetaballsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<FieldTexture>::default(),
            ExtractResourcePlugin::<BallBuffer>::default(),
            ExtractResourcePlugin::<TimeUniform>::default(),
            ExtractResourcePlugin::<ParamsUniform>::default(),
            ExtractResourcePlugin::<AlbedoTexture>::default(),
            ExtractResourcePlugin::<NormalTexture>::default(),
            ExtractResourcePlugin::<GameAssets>::default(),
        ));

        app.add_systems(Startup, setup_textures_and_uniforms);

        let render_app = app.sub_app_mut(RenderApp);

        // Gate pipeline creation strictly on AssetsReady so shader is fully loaded (avoids transient compile Err on WASM).
        render_app.add_systems(
            Render,
            create_pipeline_when_ready.in_set(RenderSet::Prepare),
        );

        // Buffers + bind group prep (tolerant if pipeline missing).
        render_app.add_systems(Render, prepare_buffers.in_set(RenderSet::PrepareBindGroups));
        render_app.add_systems(
            Render,
            prepare_bind_group
                .after(prepare_buffers)
                .in_set(RenderSet::PrepareBindGroups),
        );
        render_app.add_systems(
            Render,
            upload_metaball_buffers
                .in_set(RenderSet::Prepare)
                .after(RenderSet::PrepareBindGroups),
        );

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(MetaballPassLabel, MetaballComputeNode::default());
    }
}

fn setup_textures_and_uniforms(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<MetaballRenderSettings>,
) {
    let (w, h) = (settings.texture_size.x, settings.texture_size.y);
    let mut field = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 8],
        TextureFormat::Rgba16Float,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    field.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let field_h = images.add(field);
    let mut albedo = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    albedo.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let albedo_h = images.add(albedo);
    // Normal texture (second pass output)
    let mut normal_img = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 8],
        TextureFormat::Rgba16Float,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    normal_img.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let normal_h = images.add(normal_img);
    // Empty CPU buffer (Phase 2 placeholder)
    let balls = Vec::new();
    commands.insert_resource(FieldTexture(field_h));
    commands.insert_resource(AlbedoTexture(albedo_h));
    commands.insert_resource(BallBuffer { balls });
    commands.insert_resource(TimeUniform::default());
    commands.insert_resource(ParamsUniform {
        screen_size: [w as f32, h as f32],
        num_balls: 0,
        clustering_enabled: if settings.enable_clustering { 1 } else { 0 },
    });
    commands.insert_resource(NormalTexture(normal_h));
    info!(target: "metaballs", "created field/albedo textures {}x{}", w, h);
}

// Build the pipeline only after AssetsReady is extracted and shader is fully loaded (or embedded).
fn build_metaball_pipeline(world: &mut World) {
    let device = world.resource::<RenderDevice>();
    let layout = device.create_bind_group_layout(
        Some("metaballs.bind_group_layout"),
        &[
            BindGroupLayoutEntry {
                // field texture
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba16Float,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                // params uniform
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(std::mem::size_of::<ParamsUniform>() as u64),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                // time uniform
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(std::mem::size_of::<TimeUniform>() as u64),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                // balls storage
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                // albedo texture
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba8Unorm,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    );

    #[cfg(feature = "embed_shaders")]
    let shader_handle: Handle<Shader> = {
        let mut shaders = world.resource_mut::<Assets<Shader>>();
        shaders.add(Shader::from_wgsl(
            COMPUTE_METABALLS_WGSL,
            "embedded://compute_metaballs.wgsl",
        ))
    };
    #[cfg(not(feature = "embed_shaders"))]
    let shader_handle: Handle<Shader> = {
        let ga = world.resource::<GameAssets>();
        ga.shaders.compute_metaballs.clone()
    };

    let cache = world.resource::<PipelineCache>();
    let pipeline_id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some(Cow::Borrowed("metaballs.compute")),
        layout: vec![layout.clone()],
        push_constant_ranges: vec![],
        shader: shader_handle.clone(),
        shader_defs: vec![],
        entry_point: Cow::Borrowed("metaballs"),
        zero_initialize_workgroup_memory: false,
    });
    world.insert_resource(GpuMetaballPipeline {
        bind_group_layout: layout,
        pipeline_id,
        shader_handle,
    });
    info!(
        target = "metaballs",
        "Metaball compute pipeline created (assets ready)."
    );
}

fn create_pipeline_when_ready(world: &mut World) {
    // already built
    if world.get_resource::<GpuMetaballPipeline>().is_some() {
        return;
    }
    // need extracted assets (state machine in main world guarantees assets ready by time we reach here)
    if !world.contains_resource::<GameAssets>() {
        return;
    }

    #[cfg(not(feature = "embed_shaders"))]
    {
        use bevy::asset::LoadState;
        let ga = world.resource::<GameAssets>();
        let asset_server = world.resource::<AssetServer>();
        match asset_server.get_load_state(ga.shaders.compute_metaballs.id()) {
            Some(LoadState::Loaded) => {}
            Some(LoadState::Failed(_)) => {
                error!(
                    target = "metaballs",
                    "compute_metaballs.wgsl failed to load; cannot build pipeline"
                );
                return;
            }
            _ => {
                return;
            } // still loading (rare since state enforces readiness, but network timing on WASM may lag extraction)
        }
    }
    build_metaball_pipeline(world);
}

fn prepare_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    params: Res<ParamsUniform>,
    time_u: Res<TimeUniform>,
    _balls: Res<BallBuffer>,
    existing: Option<Res<GpuBuffers>>,
) {
    // Allocate once; subsequent frames just update via queue writes.
    if existing.is_some() {
        return;
    }
    let params_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.params"),
        contents: bytemuck::bytes_of(&*params),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let time_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.time"),
        contents: bytemuck::bytes_of(&*time_u),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    // Create zero-sized (valid) buffer; will be resized on first upload if needed.
    let balls_buf = render_device.create_buffer(&BufferDescriptor {
        label: Some("metaballs.balls"),
        size: 16,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    commands.insert_resource(GpuBuffers {
        params: params_buf,
        time: time_buf,
        balls: balls_buf,
    });
}

fn prepare_bind_group(
    mut commands: Commands,
    field: Res<FieldTexture>,
    albedo: Res<AlbedoTexture>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    pipeline: Option<Res<GpuMetaballPipeline>>,
    gpu_buffers: Option<Res<GpuBuffers>>,
    render_device: Res<RenderDevice>,
) {
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(gpu_buffers) = gpu_buffers else {
        return;
    };
    let Some(gpu_field) = gpu_images.get(&field.0) else {
        return;
    };
    let Some(gpu_albedo) = gpu_images.get(&albedo.0) else {
        return;
    };
    let bind_group = render_device.create_bind_group(
        Some("metaballs.bind_group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&gpu_field.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: gpu_buffers.params.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: gpu_buffers.time.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: gpu_buffers.balls.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: BindingResource::TextureView(&gpu_albedo.texture_view),
            },
        ],
    );
    commands.insert_resource(GpuMetaballBindGroup(bind_group));
}

fn upload_metaball_buffers(
    balls: Res<BallBuffer>,
    mut params: ResMut<ParamsUniform>,
    time_u: Res<TimeUniform>,
    gpu: Option<ResMut<GpuBuffers>>,
    queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
    pipeline: Option<Res<GpuMetaballPipeline>>,
    field: Option<Res<FieldTexture>>,
    albedo: Option<Res<AlbedoTexture>>,
    gpu_images: Option<Res<RenderAssets<GpuImage>>>,
    mut commands: Commands,
) {
    let Some(mut gpu) = gpu else {
        return;
    };
    params.num_balls = balls.balls.len() as u32;

    let required_size = (balls.balls.len() * std::mem::size_of::<BallGpu>()) as u64;
    if required_size > 0 && required_size > gpu.balls.size() {
        // Recreate storage buffer with new size.
        let new_buf = render_device.create_buffer(&BufferDescriptor {
            label: Some("metaballs.balls"),
            size: required_size.next_power_of_two(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.balls = new_buf;
        // Rebuild bind group because buffer changed.
        if let (Some(pipeline), Some(field), Some(albedo), Some(gpu_images)) =
            (pipeline, field, albedo, gpu_images)
        {
            if let (Some(gpu_field), Some(gpu_albedo)) =
                (gpu_images.get(&field.0), gpu_images.get(&albedo.0))
            {
                let bind_group = render_device.create_bind_group(
                    Some("metaballs.bind_group"),
                    &pipeline.bind_group_layout,
                    &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&gpu_field.texture_view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: gpu.params.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: gpu.time.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: gpu.balls.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(&gpu_albedo.texture_view),
                        },
                    ],
                );
                commands.insert_resource(GpuMetaballBindGroup(bind_group));
            }
        }
    }
    if required_size > 0 {
        queue.write_buffer(&gpu.balls, 0, bytemuck::cast_slice(&balls.balls));
    }
    queue.write_buffer(&gpu.params, 0, bytemuck::bytes_of(&*params));
    queue.write_buffer(&gpu.time, 0, bytemuck::bytes_of(&*time_u));
}

#[derive(Default)]
pub struct MetaballComputeNode {
    state: NodeState,
}
#[derive(Default)]
enum NodeState {
    #[default]
    Loading,
    Ready,
}
impl render_graph::Node for MetaballComputeNode {
    fn update(&mut self, world: &mut World) {
        let Some(pipeline) = world.get_resource::<GpuMetaballPipeline>() else {
            return;
        };
        let cache = world.resource::<PipelineCache>();
        if matches!(self.state, NodeState::Loading) {
            match cache.get_compute_pipeline_state(pipeline.pipeline_id) {
                CachedPipelineState::Ok(_) => self.state = NodeState::Ready,
                CachedPipelineState::Err(err) => {
                    // Because we only queue after shader fully Loaded, any Err now is a real compile failure.
                    panic!("Metaballs compute pipeline failed after assets ready:\n{err}");
                }
                _ => {}
            }
        }
    }
    fn run(
        &self,
        _ctx: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !matches!(self.state, NodeState::Ready) {
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
        // NOTE: use params uniform for dispatch size (dynamic texture size) once packed each frame; for Phase 2 rely on settings inserted
        // For now read from initial params resource
        let params = world.resource::<ParamsUniform>();
        let w = params.screen_size[0] as u32;
        let h = params.screen_size[1] as u32;
        let gx = (w + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let gy = (h + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        pass.dispatch_workgroups(gx, gy, 1);
        Ok(())
    }
}
