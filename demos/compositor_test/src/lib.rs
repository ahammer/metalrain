//! Compositor test demo showcasing layered rendering.

use bevy::prelude::*;
use scaffold::ScaffoldIntegrationPlugin;

mod components;
mod constants;
mod effects;
mod forces;
mod resources;
mod scene_setup;

pub use constants::DEMO_NAME;
use effects::*;
use forces::*;
use resources::*;
use scene_setup::*;

/// Main entry point for the compositor test demo.
pub fn run_compositor_test() {
    App::new()
        .add_plugins(ScaffoldIntegrationPlugin::with_demo_name(DEMO_NAME))
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .add_systems(Startup, (setup_scene, spawn_balls))
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
                animate_effect_overlay,
            ),
        )
        .run();
}
