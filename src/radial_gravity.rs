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
    let g = cfg.gravity.y.abs();
    if g <= 0.0 {
        return;
    }
    let dt = time.delta_secs();
    for (transform, mut vel) in q.iter_mut() {
        let pos = transform.translation.truncate();
        vel.linvel += radial_gravity_delta(g, dt, pos);
    }
}

/// Pure helper used by the system and unit tests: returns velocity delta for one step.
fn radial_gravity_delta(g: f32, dt: f32, pos: Vec2) -> Vec2 {
    if g <= 0.0 || dt <= 0.0 {
        return Vec2::ZERO;
    }
    if pos.length_squared() < 1e-6 {
        return Vec2::ZERO;
    }
    let dir_to_center = -pos.normalize();
    dir_to_center * g * dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_gravity_delta_points_inward() {
        let g = 100.0; // magnitude
        let dt = 0.016;
        let pos = Vec2::new(100.0, 0.0);
        let delta = radial_gravity_delta(g, dt, pos);
        assert!(
            delta.x < 0.0,
            "Expected negative x delta toward origin, got {delta:?}"
        );
        assert!(delta.length() > 0.0);
    }

    #[test]
    fn radial_gravity_zero_when_center_or_zero_dt() {
        assert_eq!(
            radial_gravity_delta(100.0, 0.0, Vec2::new(10.0, 0.0)),
            Vec2::ZERO
        );
        assert_eq!(
            radial_gravity_delta(0.0, 0.016, Vec2::new(10.0, 0.0)),
            Vec2::ZERO
        );
        assert_eq!(radial_gravity_delta(100.0, 0.016, Vec2::ZERO), Vec2::ZERO);
    }
}
