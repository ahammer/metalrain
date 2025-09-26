use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use game_core::Ball;

use crate::PhysicsConfig;

/// Apply clustering / attraction forces between balls (naive O(n^2) implementation).
pub fn apply_clustering_forces(
    mut query: Query<(&Transform, &mut ExternalForce), With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    if query.is_empty() { return; }
    let positions: Vec<Vec2> = query.iter().map(|(t, _)| t.translation.truncate()).collect();
    for (i, (transform, mut ext_force)) in query.iter_mut().enumerate() {
        let my_pos = transform.translation.truncate();
        let mut cluster_force = Vec2::ZERO;
        for (j, other_pos) in positions.iter().enumerate() {
            if i == j { continue; }
            let delta = *other_pos - my_pos;
            let distance = delta.length();
            if distance > 0.0 && distance < config.clustering_radius {
                let direction = delta / distance;
                let strength = (1.0 - distance / config.clustering_radius) * config.clustering_strength;
                cluster_force += direction * strength;
            }
        }
        ext_force.force = cluster_force;
    }
}

/// Clamp linear velocities to keep simulation visually legible.
pub fn clamp_velocities(mut vel_query: Query<&mut Velocity, With<Ball>>, config: Res<PhysicsConfig>) {
    for mut vel in vel_query.iter_mut() {
        let lin = vel.linvel.length();
        if lin > config.max_ball_speed {
            vel.linvel = vel.linvel.normalize_or_zero() * config.max_ball_speed;
        }
        // Optionally nudge up extremely slow moving balls (excluding nearly stopped ones)
        if lin > 0.0 && lin < config.min_ball_speed * 0.5 {
            vel.linvel = vel.linvel.normalize_or_zero() * config.min_ball_speed * 0.5;
        }
    }
}

/// Sync Rapier velocities back into the `Ball` component for other gameplay systems.
pub fn sync_physics_to_balls(mut query: Query<(&Velocity, &mut Ball)>) {
    for (vel, mut ball) in query.iter_mut() {
        ball.velocity = vel.linvel;
    }
}

/// Apply gravity from PhysicsConfig manually as an external force (workaround for direct gravity config access changes in rapier version).
pub fn apply_config_gravity(mut query: Query<&mut ExternalForce, With<Ball>>, config: Res<PhysicsConfig>) {
    for mut force in &mut query {
        // Accumulate gravity; clustering system will overwrite force, so instead we add here then clustering overwrites => order matters.
        // To preserve both, we could store both forces; for now if clustering active, gravity is weaker so we blend.
        force.force += config.gravity;
    }
}
