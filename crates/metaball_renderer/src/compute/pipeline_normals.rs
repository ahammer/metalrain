use super::types::GpuMetaballBindGroup; // ensure compute pass ran
use crate::compute::MetaballPassLabel;
#[allow(unused_imports)]
use crate::embedded_shaders;
use crate::internal::{FieldTexture, NormalTexture, ParamsUniform};
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssets,
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::GpuImage,
    Render, RenderApp,
};
use std::borrow::Cow;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct NormalsPassLabel;

pub struct NormalComputePlugin;

#[derive(Resource)]
pub struct GpuNormalsPipeline {
    pub layout: BindGroupLayout,
    pub pipeline_id: CachedComputePipelineId,
}
#[derive(Resource)]
pub struct GpuNormalsBindGroup(pub BindGroup);

impl Plugin for NormalComputePlugin {
    fn build(&self, app: &mut App) {
        // Shaders embedded; required resources already extracted by the primary compute plugin.
        crate::embedded_shaders::ensure_loaded(app.world_mut());
        let render_app = app.sub_app_mut(RenderApp);
        crate::embedded_shaders::ensure_loaded(render_app.world_mut());
        render_app.add_systems(
            Render,
            prepare_normals_bind_group.in_set(bevy::render::RenderSet::PrepareBindGroups),
        );
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(NormalsPassLabel, NormalsComputeNode::default());
        // Order: metaballs -> normals -> camera driver
        graph.add_node_edge(MetaballPassLabel, NormalsPassLabel);
        graph.add_node_edge(NormalsPassLabel, bevy::render::graph::CameraDriverLabel);
    }
    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<GpuNormalsPipeline>();
    }
}

impl FromWorld for GpuNormalsPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(
            Some("metaballs.normals.layout"),
            &[
                // field texture (read-only storage)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadOnly,
                        format: TextureFormat::Rgba16Float,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // params uniform (reuse to get screen_size)
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            std::mem::size_of::<ParamsUniform>() as u64
                        ),
                    },
                    count: None,
                },
                // normals output
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
            ],
        );

        #[cfg(all(feature = "shader_hot_reload", not(target_arch = "wasm32")))]
        let shader: Handle<Shader> = {
            let asset_server = world.resource::<AssetServer>();
            asset_server.load("shaders/compute_3d_normals.wgsl")
        };
        #[cfg(any(not(feature = "shader_hot_reload"), target_arch = "wasm32"))]
        let shader: Handle<Shader> = embedded_shaders::normals_handle();

        let cache = world.resource::<PipelineCache>();
        let pipeline_id = cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::Borrowed("metaballs.compute_normals")),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader,
            shader_defs: vec![],
            entry_point: Cow::Borrowed("compute_normals"),
            zero_initialize_workgroup_memory: false,
        });
        Self {
            layout,
            pipeline_id,
        }
    }
}

fn prepare_normals_bind_group(
    mut commands: Commands,
    pipeline: Res<GpuNormalsPipeline>,
    field: Option<Res<FieldTexture>>,
    normals: Option<Res<NormalTexture>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    gpu_metaball: Option<Res<GpuMetaballBindGroup>>, // ensure first pass prepared
    gpu_buffers: Option<Res<super::types::GpuBuffers>>,
) {
    let (_first_pass_ready, field, normals, gpu_buffers) =
        match (gpu_metaball, field, normals, gpu_buffers) {
            (Some(_), Some(f), Some(n), Some(bufs)) => (true, f, n, bufs),
            _ => return,
        };
    let Some(field_img) = gpu_images.get(&field.0) else {
        return;
    };
    let Some(norm_img) = gpu_images.get(&normals.0) else {
        return;
    };
    let bind_group = render_device.create_bind_group(
        Some("metaballs.normals.bind_group"),
        &pipeline.layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&field_img.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: gpu_buffers.params.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::TextureView(&norm_img.texture_view),
            },
        ],
    );
    commands.insert_resource(GpuNormalsBindGroup(bind_group));
}

#[derive(Default)]
struct NormalsComputeNode {
    state: NodeState,
}
#[derive(Default)]
enum NodeState {
    #[default]
    Loading,
    Ready,
}

impl render_graph::Node for NormalsComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GpuNormalsPipeline>();
        let cache = world.resource::<PipelineCache>();
        if matches!(self.state, NodeState::Loading) {
            match cache.get_compute_pipeline_state(pipeline.pipeline_id) {
                CachedPipelineState::Ok(_) => self.state = NodeState::Ready,
                CachedPipelineState::Err(err) => panic!("Failed to compile normals compute: {err}"),
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
        let pipeline_res = world.resource::<GpuNormalsPipeline>();
        let cache = world.resource::<PipelineCache>();
        let gpu_pipeline = cache
            .get_compute_pipeline(pipeline_res.pipeline_id)
            .expect("normals pipeline ready");
        // Gracefully skip until bind group prepared instead of panicking when first few frames
        let Some(bg_res) = world.get_resource::<GpuNormalsBindGroup>() else {
            return Ok(());
        };
        let bind_group = &bg_res.0;
        let params = world.resource::<ParamsUniform>();
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(gpu_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        let w = params.screen_size[0] as u32;
        let h = params.screen_size[1] as u32;
        let gx = (w + crate::internal::WORKGROUP_SIZE - 1) / crate::internal::WORKGROUP_SIZE;
        let gy = (h + crate::internal::WORKGROUP_SIZE - 1) / crate::internal::WORKGROUP_SIZE;
        pass.dispatch_workgroups(gx, gy, 1);
        Ok(())
    }
}
