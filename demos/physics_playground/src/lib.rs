//! Physics playground demo for testing ball physics and metaball rendering.
//!
//! This demo provides an interactive environment to experiment with:
//! - Ball spawning and physics simulation
//! - Gravity and clustering force adjustments
//! - Metaball rendering with various parameters
//! - Real-time performance monitoring

use bevy::math::UVec2;
use bevy::prelude::*;

use background_renderer::{BackgroundConfig, BackgroundMode};
use scaffold::{ScaffoldConfig, ScaffoldIntegrationPlugin};

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
        .insert_resource(
            ScaffoldConfig::default()
                .with_base_resolution(UVec2::new(1280, 720))
                .with_metaball_texture_size(UVec2::new(1024, 1024))
                .with_world_half_extent(ARENA_HALF_EXTENT)
                .with_wall_thickness(WALL_THICKNESS),
        )
        .insert_resource(BackgroundConfig {
            mode: BackgroundMode::LinearGradient,
            primary_color: bevy::color::LinearRgba::rgb(0.05, 0.05, 0.15),
            secondary_color: bevy::color::LinearRgba::rgb(0.1, 0.1, 0.2),
            angle: 0.25 * std::f32::consts::PI,
            animation_speed: 0.5,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.75,
        })
        .init_resource::<PlaygroundState>()
        .add_plugins(ScaffoldIntegrationPlugin::with_demo_name(DEMO_NAME))
        .add_systems(Startup, (setup_ui, spawn_test_balls))
        .add_systems(
            Update,
            (
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
