use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::{
    extract_resource::ExtractResourcePlugin,
    render_asset::RenderAssets,
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::GpuImage,
    Render, RenderApp, RenderSet,
};
use crate::constants::*;
use super::types::*;

pub struct MetaballComputePlugin;

#[derive(Resource)]
pub struct GpuMetaballPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MetaballPassLabel;

impl Plugin for MetaballComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<MetaballTarget>::default(),
            ExtractResourcePlugin::<BallBuffer>::default(),
            ExtractResourcePlugin::<TimeUniform>::default(),
            ExtractResourcePlugin::<ParamsUniform>::default(),
        ));

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            (prepare_buffers, prepare_bind_group.after(prepare_buffers))
                .in_set(RenderSet::PrepareBindGroups),
        );

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(MetaballPassLabel, MetaballComputeNode::default());
        graph.add_node_edge(MetaballPassLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<GpuMetaballPipeline>();
    }
}

impl FromWorld for GpuMetaballPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();

        let layout = device.create_bind_group_layout(
            Some("metaballs.bind_group_layout"),
            &[
                BindGroupLayoutEntry { // storage texture
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba8Unorm, view_dimension: TextureViewDimension::D2 },
                    count: None,
                },
                BindGroupLayoutEntry { // params uniform
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: BufferSize::new(std::mem::size_of::<ParamsUniform>() as u64) },
                    count: None,
                },
                BindGroupLayoutEntry { // time uniform
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: BufferSize::new(std::mem::size_of::<TimeUniform>() as u64) },
                    count: None,
                },
                BindGroupLayoutEntry { // balls storage
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: BufferSize::new((std::mem::size_of::<Ball>() * MAX_BALLS) as u64) },
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

        Self { bind_group_layout: layout, pipeline_id }
    }
}

fn prepare_buffers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    params: Res<ParamsUniform>,
    time_uni: Res<TimeUniform>,
    balls: Res<BallBuffer>,
) {
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
    let fixed = padded_balls_slice(&balls.balls);
    let balls_buf = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.balls"),
        contents: bytemuck::cast_slice(&fixed),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });

    commands.insert_resource(GpuBuffers { params: params_buf, time: time_buf, balls: balls_buf });
}

fn prepare_bind_group(
    mut commands: Commands,
    target: Res<MetaballTarget>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    pipeline: Res<GpuMetaballPipeline>,
    gpu_buffers: Option<Res<GpuBuffers>>,
    render_device: Res<RenderDevice>,
) {
    let Some(gpu_buffers) = gpu_buffers else { return; };
    let Some(gpu_image) = gpu_images.get(&target.texture) else { return; };

    let bind_group = render_device.create_bind_group(
        Some("metaballs.bind_group"),
        &pipeline.bind_group_layout,
        &[
            BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&gpu_image.texture_view) },
            BindGroupEntry { binding: 1, resource: gpu_buffers.params.as_entire_binding() },
            BindGroupEntry { binding: 2, resource: gpu_buffers.time.as_entire_binding() },
            BindGroupEntry { binding: 3, resource: gpu_buffers.balls.as_entire_binding() },
        ],
    );
    commands.insert_resource(GpuMetaballBindGroup(bind_group));
}

#[derive(Default)]
pub struct MetaballComputeNode { state: MetaballNodeState }

enum MetaballNodeState { Loading, Ready }

impl Default for MetaballNodeState { fn default() -> Self { MetaballNodeState::Loading } }

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

    fn run(&self, _ctx: &mut render_graph::RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), render_graph::NodeRunError> {
        if !matches!(self.state, MetaballNodeState::Ready) { return Ok(()); }
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GpuMetaballPipeline>();
        let bind_group = &world.resource::<GpuMetaballBindGroup>().0;
        let gpu_pipeline = pipeline_cache.get_compute_pipeline(pipeline.pipeline_id).expect("pipeline ready");
        let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(gpu_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        let gx = (WIDTH + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let gy = (HEIGHT + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        pass.dispatch_workgroups(gx, gy, 1);
        Ok(())
    }
}
