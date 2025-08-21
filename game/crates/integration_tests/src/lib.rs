// Phase 0 scaffold: integration_tests crate
// Purpose: Provide a place for black-box style integration tests across published plugin APIs.
// Currently only contains a sanity test ensuring all placeholder plugins compose in a Bevy App.

use bevy::prelude::*;

pub fn build_minimal_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);

    // We only verify that adding all currently defined plugins compiles/links.
    // (Feature-gated plugins tested under their cfg below.)
    app
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Advance the app by a fixed dt for `steps` iterations.
    fn advance_fixed(app: &mut App, dt: f32, steps: u32) {
        for _ in 0..steps {
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(Duration::from_secs_f32(dt));
            }
            app.update();
        }
    }

    #[test]
    fn compose_core_plugins() {
        use bm_core::CorePlugin;
        use bm_physics::PhysicsPlugin;
        use bm_rendering::RenderingPlugin;
        use bm_gameplay::GameplayPlugin;

        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(PhysicsPlugin);
        app.add_plugins(RenderingPlugin);
        app.add_plugins(GameplayPlugin);
    }

    #[cfg(feature = "bm_metaballs")]
    #[test]
    fn compose_with_metaballs() {
        use bm_core::CorePlugin;
        use bm_rendering::RenderingPlugin;
        use bm_metaballs::MetaballsPlugin;
        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(RenderingPlugin);
        app.add_plugins(MetaballsPlugin);
    }

    #[cfg(feature = "bm_debug_tools")]
    #[test]
    fn compose_with_debug() {
        use bm_core::CorePlugin;
        use bm_debug_tools::DebugToolsPlugin;
        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(DebugToolsPlugin);
    }

    #[cfg(feature = "bm_hot_reload")]
    #[test]
    fn compose_with_hot_reload() {
        use bm_core::CorePlugin;
        use bm_hot_reload::HotReloadPlugin;
        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(HotReloadPlugin);
    }

    // Phase 2 skeleton: headless physics snapshot harness (placeholder).
    // Future: advance fixed timesteps with deterministic seed, record positions, compute drift metrics vs legacy snapshot.
    // For now: ensure PhysicsPlugin systems run without panicking when a Ball entity with Velocity exists.
    #[test]
    fn headless_physics_baseline_runs() {
        use bm_core::{CorePlugin, GameConfigRes, Ball, BallRadius};
        use bm_physics::PhysicsPlugin;
        use bevy_rapier2d::prelude::Velocity;

        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(PhysicsPlugin);
        app.insert_resource(GameConfigRes::default());

        // Spawn a simple ball entity with Transform + Velocity so radial gravity system can act.
        let e = app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            Transform::from_xyz(100.0, 0.0, 0.0),
            GlobalTransform::default(),
            Velocity::default(),
        )).id();

        // Advance several fixed steps so radial gravity applies cumulative inward velocity (position still static
        // without a dynamic rigid body in this smoke test).
        advance_fixed(&mut app, 0.016, 5);

        // Verify velocity gained an inward (negative x) component with non-zero magnitude.
        let vel = app.world().entity(e).get::<Velocity>().expect("velocity exists");
        assert!(vel.linvel.x < 0.0, "expected inward radial gravity velocity, got {:?}", vel.linvel);
        assert!(vel.linvel.length() > 0.0);
    }

    /// Extended drift check: with a dynamic rigid body + collider, position should move inward after several steps.
    /// (Still a smoke test; full Phase 2 exit will serialize and compare snapshot series.)
    #[test]
    fn headless_physics_drift_moves_inward() {
        use bm_core::{CorePlugin, GameConfigRes, Ball, BallRadius};
        use bm_physics::PhysicsPlugin;
        use bevy_rapier2d::prelude::{Velocity, RigidBody, Collider};

        let mut app = build_minimal_app();
        app.add_plugins(CorePlugin);
        app.add_plugins(PhysicsPlugin);
        app.insert_resource(GameConfigRes::default());

        let initial_x = 150.0f32;

        let e = app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            Transform::from_xyz(initial_x, 0.0, 0.0),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::ball(10.0),
            Velocity::default(),
        )).id();

        // Advance fixed steps; expect x position to decrease due to inward (negative x) velocity.
        advance_fixed(&mut app, 0.016, 30);

        let vel = app.world().entity(e).get::<Velocity>().expect("velocity exists");
        assert!(vel.linvel.x < 0.0, "expected negative x velocity, got {:?}", vel.linvel);

        let tf = app.world().entity(e).get::<Transform>().expect("transform exists");
        assert!(tf.translation.x < initial_x - 0.1, "expected inward motion (< {}), got {}", initial_x - 0.1, tf.translation.x);
    }
}
