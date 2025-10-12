//! Physics playground demo for testing ball physics and metaball rendering.
//!
//! This demo provides an interactive environment to experiment with:
//! - Ball spawning and physics simulation
//! - Gravity and clustering force adjustments
//! - Metaball rendering with various parameters
//! - Real-time performance monitoring

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

use background_renderer::{BackgroundConfig, BackgroundMode, BackgroundRendererPlugin};
use event_core::EventCorePlugin;
use game_core::GameCorePlugin;
use game_physics::GamePhysicsPlugin;
use game_rendering::{GameRenderingPlugin, RenderLayer};
use metaball_renderer::{MetaballRenderSettings, MetaballRendererPlugin};
use widget_renderer::WidgetRendererPlugin;

mod components;
mod constants;
mod input;
mod resources;
mod scene_setup;
mod ui;

pub use constants::DEMO_NAME;
use constants::*;
use input::*;
use resources::*;
use scene_setup::*;
use ui::*;

/// Main entry point for the physics playground demo.
pub fn run_physics_playground() {
    App::new()
        .insert_resource(game_rendering::RenderSurfaceSettings {
            base_resolution: bevy::math::UVec2::new(1280, 720),
            ..Default::default()
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: "../../assets".into(),
            ..Default::default()
        }))
        .add_plugins(game_assets::GameAssetsPlugin::default())
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings {
            texture_size: bevy::math::UVec2::new(1024, 1024),
            world_bounds: bevy::math::Rect::from_center_size(
                Vec2::ZERO,
                Vec2::splat(ARENA_HALF_EXTENT * 2.0 + 100.0),
            ),
            enable_clustering: true,
            present_via_quad: true,
            presentation_layer: Some(RenderLayer::Metaballs.order() as u8),
        }))
        .add_plugins(GameRenderingPlugin)
        .add_plugins(BackgroundRendererPlugin)
        .insert_resource(BackgroundConfig {
            mode: BackgroundMode::LinearGradient,
            primary_color: bevy::color::LinearRgba::rgb(0.05, 0.05, 0.15),
            secondary_color: bevy::color::LinearRgba::rgb(0.1, 0.1, 0.2),
            angle: 0.25 * std::f32::consts::PI,
            animation_speed: 0.5,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.75,
        })
        .add_plugins(WidgetRendererPlugin)
        .add_plugins(EventCorePlugin::default())
        .init_resource::<PlaygroundState>()
        .add_systems(
            Startup,
            (setup_camera, setup_arena, setup_ui, spawn_test_balls),
        )
        .add_systems(
            Update,
            (
                exit_on_escape,
                spawn_ball_on_click,
                reset_on_key,
                pause_on_key,
                adjust_physics_with_keys,
                update_stats_text,
                update_mouse_position_text,
                enable_ccd_for_balls,
            ),
        )
        .run();
}
