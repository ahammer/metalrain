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
                attach_paddle_kinematic_physics, // ensure paddle bodies exist
                spawn_physics_for_new_balls, // must run before forces / sync
                drive_paddle_velocity,       // set linvel before clustering/forces
                apply_clustering_forces,
                apply_config_gravity,
                sync_physics_to_balls,
                clamp_velocities,
                clamp_paddle_positions,      // after velocities applied and possibly clamped
                handle_collision_events,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Ball;

    #[test]
    fn config_defaults_sane() {
        let cfg = PhysicsConfig::default();
        assert!(cfg.max_ball_speed > cfg.min_ball_speed);
    }

    #[test]
    fn spawn_physics_added() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
            .init_resource::<PhysicsConfig>()
            .add_systems(Update, spawn_physics_for_new_balls);
        let e = app.world_mut().spawn((
            Ball { velocity: Vec2::ZERO, radius: 10.0, color: game_core::GameColor::Red },
            Transform::from_translation(Vec3::ZERO),
            GlobalTransform::IDENTITY,
        )).id();
        app.update();
        assert!(app.world().get::<RigidBody>(e).is_some());
        assert!(app.world().get::<Collider>(e).is_some());
    }

    #[test]
    fn paddle_kinematic_attached_and_velocity_set() {
        use game_core::Paddle;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
            .init_resource::<PhysicsConfig>()
            .add_systems(Update, (attach_paddle_kinematic_physics, drive_paddle_velocity));
        let paddle_e = app.world_mut().spawn((
            Paddle::default(),
            Transform::from_translation(Vec3::ZERO),
            GlobalTransform::IDENTITY,
        )).id();
        // Simulate key press (A -> left)
        {
            app.world_mut().init_resource::<ButtonInput<KeyCode>>();
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.press(KeyCode::KeyA);
        }
        app.update();
        // Run a second frame to ensure velocity system observes kinematic body
        app.update();
        assert!(app.world().get::<RigidBody>(paddle_e).is_some());
        let vel = app.world().get::<Velocity>(paddle_e).unwrap();
        assert!(vel.linvel.x < 0.0, "expected negative x velocity, got {:?}", vel.linvel);
    }
}
