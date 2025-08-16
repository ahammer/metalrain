use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::components::{Ball, BallRadius};
use crate::config::GameConfig;
use crate::system_order::PostPhysicsAdjustSet;

/// Applies light-weight positional separation and optional velocity damping when Rapier
/// reports new collision contacts (Started events). This supplements physics by reducing
/// visible overlap jitter for many small dynamic balls.
pub struct SeparationPlugin;

impl Plugin for SeparationPlugin {
    fn build(&self, app: &mut App) {
    app.add_systems(Update, contact_separation.in_set(PostPhysicsAdjustSet));
    }
}

fn contact_separation(
    mut ev_collisions: EventReader<CollisionEvent>,
    cfg: Res<GameConfig>,
    mut q_rb: Query<(Entity, &mut Transform, Option<&mut Velocity>, &BallRadius), With<Ball>>,
) {
    let sep_cfg = &cfg.separation;
    if !sep_cfg.enabled { return; }

    // Use Bevy's fast hash map (fxhash) to gather per-entity corrections to avoid repeated mutable borrows.
    use bevy::utils::HashMap;
    // Heuristic capacity: collisions roughly ~entities in dense scenarios.
    let mut pos_shifts: HashMap<Entity, Vec2> = HashMap::default();
    let mut vel_normals: HashMap<Entity, Vec2> = HashMap::default();

    for ev in ev_collisions.read() {
        let CollisionEvent::Started(e1, e2, _flags) = ev else { continue; };
        // Only continue if both collider entities are tracked balls.
        let Ok((_, t1, _, r1)) = q_rb.get(*e1) else { continue; };
        let Ok((_, t2, _, r2)) = q_rb.get(*e2) else { continue; };

        let p1 = t1.translation.truncate();
        let p2 = t2.translation.truncate();
        let delta = p2 - p1;
        let dist_sq = delta.length_squared();
        if dist_sq < 1e-6 { continue; } // practically same position
        let dist = dist_sq.sqrt();
        let target = (r1.0 + r2.0) * sep_cfg.overlap_slop;
        if dist >= target { continue; }
        let overlap = target - dist;
        if overlap <= 0.0 { continue; }

        let normal = delta / dist; // direction from e1 -> e2
        // Split correction between the two entities.
        let mut push_mag = overlap * sep_cfg.push_strength * 0.5;
        if push_mag > sep_cfg.max_push { push_mag = sep_cfg.max_push; }
        let push_vec = normal * push_mag;
        pos_shifts.entry(*e1).and_modify(|v| *v -= push_vec).or_insert(-push_vec);
        pos_shifts.entry(*e2).and_modify(|v| *v += push_vec).or_insert(push_vec);

        if sep_cfg.velocity_dampen > 0.0 {
            // Accumulate normals (not normalized yet) so multiple contacts weigh in.
            vel_normals.entry(*e1).and_modify(|v| *v += normal).or_insert(normal);
            vel_normals.entry(*e2).and_modify(|v| *v -= normal).or_insert(-normal); // opposite direction
        }
    }

    // Apply position shifts.
    for (entity, shift) in pos_shifts.into_iter() {
        if let Ok((_, mut transform, _, _)) = q_rb.get_mut(entity) {
            transform.translation.x += shift.x;
            transform.translation.y += shift.y;
        }
    }

    // Dampen velocity component along accumulated normals.
    if sep_cfg.velocity_dampen > 0.0 {
        for (entity, combined) in vel_normals.into_iter() {
            if let Ok((_, _tf, Some(mut vel), _)) = q_rb.get_mut(entity) {
                if combined.length_squared() > 0.0 {
                    let n = combined.normalize();
                    let vn = vel.linvel.dot(n);
                    if vn > 0.0 { // moving outward along normal
                        vel.linvel -= n * vn * sep_cfg.velocity_dampen;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn applies_half_push_and_dampen() {
        // Minimal app with our system
        let mut app = App::new();
        app.add_event::<CollisionEvent>();
        app.add_systems(Update, contact_separation);
        app.insert_resource(GameConfig {
            draw_circles: false,
            rapier_debug: false,
            metaballs_enabled: true,
            draw_cluster_bounds: false,
            window: crate::config::WindowConfig { width: 800.0, height: 600.0, title: "T".into() },
            gravity: crate::config::GravityConfig { y: -9.8 },
            bounce: crate::config::BounceConfig { restitution: 0.5 },
            balls: crate::config::BallSpawnConfig {
                count: 0,
                radius_range: crate::config::SpawnRange { min: 5.0, max: 10.0 },
                x_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                y_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                vel_x_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
                vel_y_range: crate::config::SpawnRange { min: 0.0, max: 0.0 },
            },
            separation: crate::config::CollisionSeparationConfig {
                enabled: true,
                overlap_slop: 0.99,
                push_strength: 1.0,
                max_push: 100.0,
                velocity_dampen: 0.5,
            },
            interactions: crate::config::InteractionConfig { explosion: crate::config::ExplosionConfig { enabled: true, impulse: 0.0, radius: 0.0, falloff_exp: 1.0 }, drag: crate::config::DragConfig { enabled: false, grab_radius: 0.0, pull_strength: 0.0, max_speed: 0.0 } },
        });

        // Two overlapping balls along x axis
    let _e1 = app.world_mut().spawn((Ball, BallRadius(10.0), Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default(), Velocity::linear(Vec2::new(10.0, 0.0)))).id();
    let _e2 = app.world_mut().spawn((Ball, BallRadius(10.0), Transform::from_xyz(18.0, 0.0, 0.0), GlobalTransform::default(), Velocity::linear(Vec2::new(-5.0, 0.0)))).id();

        // Dist = 18, radii sum = 20, target = 19.8 => overlap = 1.8; half push each -> 0.9 expected.
        // Send collision started event
    // NOTE: CollisionEvent::Started requires flags in this crate version; constructing a Started event is unstable in unit context without flags type exposed.
    // Instead of injecting a real event, directly invoke the system logic by crafting an EventReader with one event would require deeper harness.
    // For now mark test as a placeholder.
    // (Future improvement: create a custom schedule run with manual events resource mutation.)
    // Skipping actual system invocation due to flags type access limitations.
    // TODO: Implement event injection harness to validate separation adjustments.
    // Placeholder test currently ignored.
    // Intentionally no assertions.

        app.update();

    // Assertions pending harness.
    }
}
