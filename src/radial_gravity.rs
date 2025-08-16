use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::components::Ball;
use crate::config::GameConfig;
use crate::system_order::PrePhysicsSet;

/// Plugin adding a per-ball radial gravity (pull toward origin) applied before physics.
pub struct RadialGravityPlugin;

impl Plugin for RadialGravityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_radial_gravity.in_set(PrePhysicsSet));
    }
}

fn apply_radial_gravity(
    cfg: Res<GameConfig>,
    mut q: Query<(&Transform, &mut Velocity), With<Ball>>,
    time: Res<Time>,
) {
    // Use magnitude from config.gravity.y (treat absolute value) for strength (pixels/sec^2).
    let g = cfg.gravity.y.abs();
    if g <= 0.0 { return; }
    let dt = time.delta_seconds();
    for (transform, mut vel) in q.iter_mut() {
        let pos = transform.translation.truncate();
        if pos.length_squared() < 1e-6 { continue; }
        let dir_to_center = -pos.normalize();
        // Basic acceleration integration: v += a * dt
        vel.linvel += dir_to_center * g * dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn pulls_toward_center() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig {
            window: crate::config::WindowConfig { width: 800.0, height: 600.0, title: "T".into() },
            gravity: crate::config::GravityConfig { y: -100.0 }, // magnitude 100
            bounce: crate::config::BounceConfig { restitution: 0.5 },
            balls: crate::config::BallSpawnConfig {
                count: 0,
                radius_range: crate::config::SpawnRange { min: 5.0, max: 10.0 },
                x_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                y_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                vel_x_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                vel_y_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
            },
            separation: crate::config::CollisionSeparationConfig { enabled: false, overlap_slop: 1.0, push_strength: 0.0, max_push: 0.0, velocity_dampen: 0.0 },
            rapier_debug: false,
            draw_circles: false,
            metaballs_enabled: false,
            draw_cluster_bounds: false,
            interactions: crate::config::InteractionConfig { explosion: crate::config::ExplosionConfig { enabled: true, impulse: 0.0, radius: 0.0, falloff_exp: 1.0 }, drag: crate::config::DragConfig { enabled: false, grab_radius: 0.0, pull_strength: 0.0, max_speed: 0.0 } },
        });
    app.add_systems(Update, apply_radial_gravity);
    let e = app.world_mut().spawn((Ball, Transform::from_xyz(100.0, 0.0, 0.0), GlobalTransform::default(), Velocity::linear(Vec2::ZERO))).id();
    // Manually advance time so delta_seconds() > 0
    use std::time::Duration;
    if let Some(mut time) = app.world_mut().get_resource_mut::<Time>() { time.advance_by(Duration::from_secs_f32(0.016)); }
    app.update(); // run systems once
    let vel = app.world().get::<Velocity>(e).unwrap();
    assert!(vel.linvel.x < 0.0, "Velocity should point toward origin (negative x)");
    }
}
