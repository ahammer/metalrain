// Two-phase metaballs post-process inversion pass (PoC)
// Phase 1: existing metaballs unified material draw
// Phase 2: this post-process fullscreen inversion (optional toggle)
//
// TODO: extend post-process chain with additional effects (palette, bloom, metadata composite)
// PERF: evaluate batching multiple simple color ops into single shader before adding >3 passes

use bevy::prelude::*;
use bevy::render::{
    extract_component::ExtractComponent,
    extract_resource::ExtractResource,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
    render_resource::*,
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, ExtractSchedule,
};
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::render::renderer::RenderDevice;

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;

#[cfg(target_arch = "wasm32")]
static INVERT_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

/// Runtime toggle resource (app world)
#[derive(Resource, Debug, Clone, Copy)]
pub struct PostProcessToggle {
    pub invert: bool,
}

/// Extracted (render world) copy
#[derive(Resource, Debug, Clone, Copy, ExtractResource)]
pub struct PostProcessToggleExtracted {
    pub invert: bool,
}

#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct InversionPostProcess;

/// Label for our custom render graph node
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct MetaballsInversionNodeLabel;

/// Plugin wiring the inversion pass.
pub struct MetaballsPostProcessPlugin;

impl Plugin for MetaballsPostProcessPlugin {
    fn build(&self, app: &mut App) {
        // Insert toggle from GameConfig at startup
        app.add_systems(Startup, init_post_toggle);

        // Log activation & tag cameras in app world (so tests can observe marker)
        app.add_systems(Startup, (log_activation, tag_camera_on_startup).after(init_post_toggle));

        // WASM shader embedding (app world – asset server domain)
        #[cfg(target_arch = "wasm32")]
        {
            use bevy::asset::Assets;
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let handle = shaders.add(Shader::from_wgsl(
                include_str!("../../../assets/shaders/post_invert.wgsl"),
                "post_invert_embedded.wgsl",
            ));
            INVERT_SHADER_HANDLE.get_or_init(|| handle.clone());
        }

        // Render app setup
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app
            .add_systems(ExtractSchedule, extract_toggle)
            .add_systems(ExtractSchedule, extract_camera_marker.after(extract_toggle))
            .init_resource::<InversionPipeline>()
            .add_systems(Render, prepare_inversion_pipeline);

        // Add render graph node into the Core2d sub-graph after tonemapping and before end post-processing
        {
            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            let graph_2d = render_graph.get_sub_graph_mut(Core2d).expect("Core2d graph exists");
            graph_2d.add_node(MetaballsInversionNodeLabel, InversionNode::default());
            let _ = graph_2d.add_node_edge(Node2d::Tonemapping, MetaballsInversionNodeLabel);
            let _ = graph_2d.add_node_edge(
                MetaballsInversionNodeLabel,
                Node2d::EndMainPassPostProcessing,
            );
        }
    }
}

fn init_post_toggle(mut commands: Commands, cfg: Res<crate::core::config::config::GameConfig>) {
    commands.insert_resource(PostProcessToggle {
        invert: cfg.metaballs_post.invert_enabled,
    });
}

fn log_activation(toggle: Res<PostProcessToggle>) {
    if toggle.invert {
        info!(target: "postprocess", "Metaballs inversion post-process enabled");
    }
}

// Extract resource copy
fn extract_toggle(mut commands: Commands, toggle: Res<PostProcessToggle>) {
    commands.insert_resource(PostProcessToggleExtracted { invert: toggle.invert });
}

// Attach camera marker (app world) just after camera creation if enabled.
fn tag_camera_on_startup(
    mut commands: Commands,
    q_views: Query<Entity, (With<Camera>, With<Camera2d>)>,
    toggle: Res<PostProcessToggle>,
    existing: Query<&InversionPostProcess>,
) {
    if !toggle.invert {
        return;
    }
    for e in q_views.iter() {
        if existing.get(e).is_err() {
            commands.entity(e).insert(InversionPostProcess);
        }
    }
}

// Render-world only fallback tagging (covers cameras spawned after startup)
fn extract_camera_marker(
    mut commands: Commands,
    q_views: Query<Entity, (With<Camera>, With<Camera2d>)>,
    toggle: Option<Res<PostProcessToggleExtracted>>,
    existing: Query<&InversionPostProcess>,
) {
    let Some(toggle) = toggle else { return; };
    if !toggle.invert {
        return;
    }
    for e in q_views.iter() {
        if existing.get(e).is_err() {
            commands.entity(e).insert(InversionPostProcess);
        }
    }
}

/// Pipeline resource prepared once (render world)
#[derive(Resource)]
struct InversionPipeline {
    layout: Option<BindGroupLayout>,
    sampler: Option<Sampler>,
    pipeline_id: Option<CachedRenderPipelineId>,
    shader: Option<Handle<Shader>>,
}
impl Default for InversionPipeline {
    fn default() -> Self {
        Self {
            layout: None,
            sampler: None,
            pipeline_id: None,
            shader: None,
        }
    }
}

fn prepare_inversion_pipeline(
    pipeline_cache: ResMut<PipelineCache>,
    mut pipe: ResMut<InversionPipeline>,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    if pipe.layout.is_none() {
        let layout = render_device.create_bind_group_layout(
            Some("inversion.bind_group_layout"),
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("inversion.sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        pipe.layout = Some(layout);
        pipe.sampler = Some(sampler);
    }

    if pipe.shader.is_none() {
        #[cfg(target_arch = "wasm32")]
        {
            pipe.shader = Some(INVERT_SHADER_HANDLE.get().unwrap().clone());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            pipe.shader = Some(asset_server.load("shaders/post_invert.wgsl"));
        }
    }

    if pipe.pipeline_id.is_none() {
        let shader_handle = pipe.shader.as_ref().unwrap().clone();
        let pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("Metaballs Inversion Pipeline".into()),
            layout: vec![pipe.layout.as_ref().unwrap().clone()],
            vertex: VertexState {
                shader: shader_handle.clone(),
                entry_point: "vs".into(),
                shader_defs: vec![],
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: shader_handle,
                entry_point: "fs".into(),
                shader_defs: vec![],
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        };
        let id = pipeline_cache.queue_render_pipeline(pipeline_descriptor);
        pipe.pipeline_id = Some(id);
    }
}

/// Render graph node performing the inversion draw.
#[derive(Default)]
struct InversionNode;

impl Node for InversionNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let toggle = world.get_resource::<PostProcessToggleExtracted>().cloned();
        if !toggle.map(|t| t.invert).unwrap_or(false) {
            return Ok(());
        }

        let pipeline_res = world.resource::<InversionPipeline>();
        let Some(pipeline_id) = pipeline_res.pipeline_id else { return Ok(()); };

        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else {
            return Ok(());
        };

        let Some(layout) = &pipeline_res.layout else { return Ok(()); };
        let Some(sampler) = &pipeline_res.sampler else { return Ok(()); };

        // Iterate entities directly (avoid creating QueryState requiring &mut World)
        for entity_ref in world.iter_entities() {
            if entity_ref.get::<InversionPostProcess>().is_none() {
                continue;
            }
            if entity_ref.get::<ExtractedView>().is_none() {
                continue;
            }
            let Some(view_target) = entity_ref.get::<ViewTarget>() else {
                continue;
            };

            let post_process = view_target.post_process_write();
            let source = &post_process.source;
            let destination = &post_process.destination;

            let bind_group = render_context.render_device().create_bind_group(
                Some("inversion.bind_group"),
                layout,
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(source),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(sampler),
                    },
                ],
            );

            let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("metaballs_inversion_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: destination,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_render_pipeline(render_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}
