//! Compositor test demo showcasing layered rendering.

use bevy::prelude::*;
use scaffold::ScaffoldIntegrationPlugin;

mod constants;
mod forces;
mod resources;
mod scene_setup;
mod ui;

pub use constants::DEMO_NAME;
use forces::*;
use resources::*;
use scene_setup::*;
use ui::*;

/// Main entry point for the compositor test demo.
pub fn run_compositor_test() {
    App::new()
        .add_plugins(ScaffoldIntegrationPlugin::with_demo_name(DEMO_NAME))
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .init_resource::<CompositorState>()
    .add_systems(Startup, (setup_scene, spawn_balls, setup_ui))
        .add_systems(PostStartup, configure_metaball_presentation)
        .add_systems(
            PreUpdate,
            (update_burst_force_state, apply_burst_forces).chain(),
        )
        .add_systems(
            Update,
            (
                handle_keyboard_shortcuts,
                update_fps_counter,
                update_wall_pulse_state,
                apply_wall_pulse_forces,
                update_ui_displays,
                handle_manual_effect_triggers,
            ),
        )
        .run();
}
