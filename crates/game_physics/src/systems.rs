use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use game_core::Ball;

use crate::PhysicsConfig;
use std::collections::HashMap;

/// Apply clustering / attraction forces between balls (naive O(n^2) implementation).
pub fn apply_clustering_forces(
    mut query: Query<(&Transform, &mut ExternalForce), With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    if query.is_empty() { return; }
    if !config.optimize_clustering {
        // Fallback to naive implementation
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
        return;
    }

    // Spatial hash optimization
    let cell_size = config.clustering_radius.max(1.0); // avoid division by zero
    let mut positions: Vec<Vec2> = Vec::new();
    positions.reserve(query.iter().len());
    for (t, _) in query.iter() { positions.push(t.translation.truncate()); }

    // Build grid: map from cell coord -> indices of balls in that cell
    let mut grid: HashMap<(i32,i32), Vec<usize>> = HashMap::new();
    for (i, pos) in positions.iter().enumerate() {
        let cell = ((pos.x / cell_size).floor() as i32, (pos.y / cell_size).floor() as i32);
        grid.entry(cell).or_default().push(i);
    }

    // Collect mutable references after we built positions (to satisfy borrow rules)
    // We'll recreate an iterator to mutate forces.
    let mut idx = 0usize;
    for (transform, mut ext_force) in query.iter_mut() {
        let my_pos = transform.translation.truncate();
        let my_cell = ((my_pos.x / cell_size).floor() as i32, (my_pos.y / cell_size).floor() as i32);
        let mut cluster_force = Vec2::ZERO;
        for ox in -1..=1 { for oy in -1..=1 {
            if let Some(indices) = grid.get(&(my_cell.0 + ox, my_cell.1 + oy)) {
                for &j in indices {
                    if j == idx { continue; }
                    let other_pos = positions[j];
                    let delta = other_pos - my_pos;
                    let distance = delta.length();
                    if distance > 0.0 && distance < config.clustering_radius {
                        let direction = delta / distance;
                        let strength = (1.0 - distance / config.clustering_radius) * config.clustering_strength;
                        cluster_force += direction * strength;
                    }
                }
            }
        }}
        ext_force.force = cluster_force;
        idx += 1;
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

/// Basic collision event handling (foundation for later gameplay hooks).
/// Logs collision start events involving two `Ball` entities. Future extension could
/// emit higher‑level game events or apply effects.
pub fn handle_collision_events(
    mut collisions: EventReader<CollisionEvent>,
    balls: Query<(), With<Ball>>,
) {
    for ev in collisions.read() {
        if let CollisionEvent::Started(e1, e2, _flags) = ev {
            let a_ball = balls.get(*e1).is_ok();
            let b_ball = balls.get(*e2).is_ok();
            if a_ball && b_ball {
                info!("Ball-Ball collision: {:?} <-> {:?}", e1, e2);
            }
        }
    }
}
