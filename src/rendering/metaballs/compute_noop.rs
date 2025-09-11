use bevy::prelude::*;
use bevy::render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
    renderer::RenderContext,
    render_resource::*,
};
use std::borrow::Cow;

#[cfg(target_arch = "wasm32")] use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")] static METABALLS_NOOP_COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
pub fn init_wasm_noop_shader(world: &mut World) {
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    let handle = shaders.add(Shader::from_wgsl(
        include_str!("../../../assets/shaders/metaballs_noop_compute.wgsl"),
        "metaballs_noop_compute_embedded.wgsl",
    ));
    METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get_or_init(|| handle);
}

#[derive(Resource, Default)]
pub struct MetaballsNoopComputePipeline {
    pub pipeline_id: Option<CachedComputePipelineId>,
    pub shader: Option<Handle<Shader>>,
    pub logged: bool,
}

#[derive(Resource, Default)]
pub struct MetaballsNoopDispatchCount(pub u64);

pub fn prepare_noop_compute_pipeline(
    mut pipe: ResMut<MetaballsNoopComputePipeline>,
    mut pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    if pipe.shader.is_none() {
        #[cfg(target_arch = "wasm32")] {
            pipe.shader = Some(METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get().unwrap().clone());
        }
        #[cfg(not(target_arch = "wasm32"))] {
            pipe.shader = Some(asset_server.load("shaders/metaballs_noop_compute.wgsl"));
        }
    }
    if pipe.pipeline_id.is_none() {
        let shader = pipe.shader.as_ref().unwrap().clone();
        let desc = ComputePipelineDescriptor {
            label: Some("metaballs.noop.compute".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            shader,
            entry_point: Cow::from("cs_main"),
            shader_defs: vec![],
            zero_initialize_workgroup_memory: false,
        };
        pipe.pipeline_id = Some(pipeline_cache.queue_compute_pipeline(desc));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MetaballsNoopComputeNodeLabel;

#[derive(Default)]
pub struct MetaballsNoopComputeNode;

impl Node for MetaballsNoopComputeNode {
    fn run(&self, _graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
        let Some(res) = world.get_resource::<MetaballsNoopComputePipeline>() else { return Ok(()); };
        let Some(pid) = res.pipeline_id else { return Ok(()); };
        let cache = world.resource::<PipelineCache>();
        let Some(pipeline) = cache.get_compute_pipeline(pid) else { return Ok(()); };
        let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("metaballs_noop_precompute"), timestamp_writes: None });
        pass.set_pipeline(pipeline);
        pass.dispatch_workgroups(1, 1, 1);
        // NOTE: Skipping dispatch counter increment in render graph node (no mutable world).
        Ok(())
    }
}

pub fn log_noop_once(mut pipe: ResMut<MetaballsNoopComputePipeline>) {
    if pipe.pipeline_id.is_some() && !pipe.logged {
        info!(target="metaballs", "No-op compute prepass active (compute -> material ordering)");
        pipe.logged = true;
    }
}
