//! game_physics: Rapier2D integration + foundational physics systems for balls.
//!
//! Responsibilities:
//! * Provide `PhysicsConfig` tunable at runtime.
//! * Install Rapier physics + gravity config.
//! * Clustering forces (naive implementation; optimize later with spatial partitioning).
//! * Velocity clamping to keep motion readable.
//! * Sync Rapier velocity -> `Ball` component for other systems.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

mod config;
mod systems;
// (UI panel moved into demo to avoid cross-version plugin duplication issues.)

pub use config::PhysicsConfig;
use systems::*;

pub struct GamePhysicsPlugin;
impl Plugin for GamePhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Resource
        app.init_resource::<PhysicsConfig>();

        // Rapier configuration
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0));

        // Systems (order roughly follows Sprint 2 pipeline draft)
        app.add_systems(
            Update,
            (
                apply_clustering_forces,
                apply_config_gravity,
                sync_physics_to_balls,
                clamp_velocities,
                handle_collision_events,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_sane() {
        let cfg = PhysicsConfig::default();
        assert!(cfg.max_ball_speed > cfg.min_ball_speed);
    }
}
