//! Compositor test demo showcasing the layered rendering system.
//!
//! This demo tests the game's compositor by rendering multiple layers with
//! configurable blend modes, dynamic physics, metaball rendering, and interactive controls.

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use background_renderer::{BackgroundConfig, BackgroundRendererPlugin};
use game_assets::GameAssetsPlugin;
use game_rendering::{GameRenderingPlugin, RenderLayer, RenderSurfaceSettings};
use metaball_renderer::{MetaballRenderSettings, MetaballRendererPlugin};

mod components;
mod constants;
mod effects;
mod forces;
mod hud;
mod input;
mod resources;
mod scene_setup;

pub use constants::DEMO_NAME;
use constants::*;
use effects::*;
use forces::*;
use hud::*;
use input::*;
use resources::*;
use scene_setup::*;

/// Main entry point for the compositor test demo.
pub fn run_compositor_test() {
    App::new()
        .insert_resource(RenderSurfaceSettings {
            base_resolution: UVec2::new(1280, 720),
            ..Default::default()
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: "../../assets".into(),
            ..Default::default()
        }))
        .add_plugins(GameAssetsPlugin::default())
        .add_plugins(GameRenderingPlugin)
        .add_plugins(BackgroundRendererPlugin)
        .insert_resource(BackgroundConfig::default())
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-HALF_EXTENT, -HALF_EXTENT),
                    Vec2::new(HALF_EXTENT, HALF_EXTENT),
                ))
                .clustering_enabled(true)
                .with_presentation(true)
                .with_presentation_layer(RenderLayer::Metaballs.order() as u8),
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .init_resource::<PerformanceOverlayState>()
        .init_resource::<PerformanceStats>()
        .init_resource::<LayerHudCache>()
        .init_resource::<FrameCounter>()
        .add_systems(Startup, (setup_scene, spawn_walls, spawn_balls, spawn_hud))
        .add_systems(PostStartup, configure_metaball_presentation)
        .add_systems(
            PreUpdate,
            (update_burst_force_state, apply_burst_forces).chain(),
        )
        .add_systems(
            Update,
            (
                update_wall_pulse_state,
                apply_wall_pulse_forces,
                handle_compositor_inputs,
                animate_effect_overlay,
                accumulate_performance_stats,
                update_hud,
                log_periodic_performance_snapshot,
            ),
        )
        .run();
}
