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
    if let Some((push_vec, normal)) = compute_pair_push(dist, r1.0, r2.0, sep_cfg, delta) {
        pos_shifts.entry(*e1).and_modify(|v| *v -= push_vec).or_insert(-push_vec);
        pos_shifts.entry(*e2).and_modify(|v| *v += push_vec).or_insert(push_vec);

        if sep_cfg.velocity_dampen > 0.0 {
            // Accumulate normals (not normalized yet) so multiple contacts weigh in.
            vel_normals.entry(*e1).and_modify(|v| *v += normal).or_insert(normal);
            vel_normals.entry(*e2).and_modify(|v| *v -= normal).or_insert(-normal); // opposite direction
        }
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
    fn compute_pair_push_basic() {
        // Overlapping two equal radii balls
        let cfg = crate::config::CollisionSeparationConfig { enabled: true, overlap_slop: 0.99, push_strength: 1.0, max_push: 100.0, velocity_dampen: 0.5 };
        let r = 10.0;
        let dist = 18.0; // radii sum 20 -> target 19.8 -> overlap 1.8 -> half push 0.9
        let delta = Vec2::new(18.0, 0.0); // e1->e2
        let (push_vec, normal) = compute_pair_push(dist, r, r, &cfg, delta).expect("should overlap");
        assert!((push_vec.length() - 0.9).abs() < 1e-3, "expected ~0.9 push each, got {:?}", push_vec);
        assert_eq!(normal, Vec2::X);
    }
}

/// Computes per-pair position push vector (applied in opposite directions) and collision normal if overlapping.
fn compute_pair_push(dist: f32, r1: f32, r2: f32, cfg: &crate::config::CollisionSeparationConfig, delta: Vec2) -> Option<(Vec2, Vec2)> {
    if dist <= 0.0 { return None; }
    let target = (r1 + r2) * cfg.overlap_slop;
    if dist >= target { return None; }
    let overlap = target - dist;
    if overlap <= 0.0 { return None; }
    let normal = delta / dist; // direction e1->e2
    let mut push_mag = overlap * cfg.push_strength * 0.5;
    if push_mag > cfg.max_push { push_mag = cfg.max_push; }
    let push_vec = normal * push_mag;
    Some((push_vec, normal))
}
