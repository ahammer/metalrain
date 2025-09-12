//! Deprecated monolithic module kept temporarily for transition.
//! The implementation has been split across:
//! - gpu.rs (GPU data & uniforms)
//! - material.rs (Material definition)
//! - resources.rs (ECS resources & enums)
//! - startup.rs (startup systems)
//! - systems.rs (runtime update systems)
//! This file now only defines the plugin using the refactored modules.

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;

use crate::core::system::system_order::PostPhysicsAdjustSet;
use super::{material::MetaballsUnifiedMaterial, resources::*, startup::*, systems::*};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetaballsUpdateSet; // Public so other plugins (spawners) can order before this.

pub struct MetaballsPlugin;

impl Plugin for MetaballsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        { super::material::init_wasm_shader(app.world_mut()); }
    #[cfg(target_arch = "wasm32")]
    { super::gradient_compute::init_wasm_gradient_shader(app.world_mut()); }
        app
            .init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .init_resource::<MetaballsShadowParams>()
            .init_resource::<MetaballForeground>()
            .init_resource::<MetaballBackground>()
            .init_resource::<BallTilingConfig>()
            .init_resource::<BallTilesMeta>()
            .init_resource::<BallCpuShadow>()
            .init_resource::<MetaballsGroupDebugTimer>()
            .init_resource::<crate::rendering::metaballs::palette::ClusterPaletteStorage>()
            .add_plugins((Material2dPlugin::<MetaballsUnifiedMaterial>::default(),))
            .add_systems(Startup, (
                initialize_toggle_from_config,
                apply_config_to_params,
                apply_shader_modes_from_config,
                apply_shadow_from_config,
                setup_metaballs,
                log_initial_modes,
            ))
            .configure_sets(Update, MetaballsUpdateSet.after(PostPhysicsAdjustSet))
            .add_systems(Update, (
                update_metaballs_unified_material,
                build_metaball_tiles.after(update_metaballs_unified_material),
                cycle_foreground_mode,
                cycle_background_mode,
                resize_fullscreen_quad,
                tweak_metaballs_params,
            ).in_set(MetaballsUpdateSet));
        // Ensure main-world gradient image resource + resize system registered before entering render sub-app
        app.init_resource::<super::gradient_compute::MetaballsGradientImages>()
            .add_systems(Update, super::gradient_compute::prepare_gradient_target_main);

        // Insert gradient compute prepass into render sub-app & graph ordering
        use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
        use bevy::render::RenderApp;
        use bevy::render::render_graph::RenderGraph;
        // Use Render schedule constant from bevy::render::Render
        use bevy::render::Render;
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<super::gradient_compute::MetaballsGradientPipeline>()
            .init_resource::<super::gradient_compute::MetaballsGradientToggle>()
            .init_resource::<super::gradient_compute::MetaballsGradientStats>()
            .add_systems(Render, super::gradient_compute::prepare_gradient_pipeline)
            .add_systems(bevy::render::ExtractSchedule, super::gradient_compute::extract_gradient_images)
            .add_systems(Render, (
                super::gradient_compute::assemble_gradient_bind_group,
                super::gradient_compute::accumulate_gradient_stats.after(super::gradient_compute::assemble_gradient_bind_group),
            ))
            .add_systems(Render, super::gradient_compute::log_gradient_once.after(super::gradient_compute::prepare_gradient_pipeline));
        // (Allocation handled in main world above; extraction copies into render world each frame.)
        use super::gradient_compute::{MetaballsGradientComputeNodeLabel, MetaballsGradientComputeNode};
        let mut rg = render_app.world_mut().resource_mut::<RenderGraph>();
        let sub = rg.get_sub_graph_mut(Core2d).expect("Core2d graph exists");
        sub.add_node(MetaballsGradientComputeNodeLabel, MetaballsGradientComputeNode::default());
        let _ = sub.add_node_edge(Node2d::StartMainPass, MetaballsGradientComputeNodeLabel);
        let _ = sub.add_node_edge(MetaballsGradientComputeNodeLabel, Node2d::MainOpaquePass);
    }
}
