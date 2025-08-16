use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::components::{Ball, BallRadius};
use crate::config::GameConfig;

// Separation that reacts only to actual Rapier contact events and applies a half positional correction
// plus optional velocity damping along the normal. Tunables are read from GameConfig.separation.
pub struct SeparationPlugin;

impl Plugin for SeparationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, contact_separation);
    }
}

fn contact_separation(
    mut ev_collisions: EventReader<CollisionEvent>,
    cfg: Res<GameConfig>,
    mut q_rb: Query<(Entity, &mut Transform, Option<&mut Velocity>, &BallRadius), With<Ball>>,
) {
    if !cfg.separation.enabled { return; }

    // We'll gather corrections per entity to avoid multiple mutable borrows.
    use std::collections::HashMap;
    let mut pos_shifts: HashMap<Entity, Vec2> = HashMap::new();
    let mut vel_dampen_normals: HashMap<Entity, Vec<Vec2>> = HashMap::new();

    for ev in ev_collisions.read() {
        if let CollisionEvent::Started(e1, e2, _flags) = ev {
            // Only proceed if both are balls
            let Ok((_, t1, _, r1)) = q_rb.get(*e1) else { continue; };
            let Ok((_, t2, _, r2)) = q_rb.get(*e2) else { continue; };

            let p1 = t1.translation.truncate();
            let p2 = t2.translation.truncate();
            let delta = p2 - p1;
            let dist_sq = delta.length_squared();
            if dist_sq < 0.0001 { continue; }
            let dist = dist_sq.sqrt();
            let sum = r1.0 + r2.0;
            let target = sum * cfg.separation.overlap_slop;
            if dist >= target { continue; }
            let overlap = target - dist;
            if overlap <= 0.0 { continue; }
            let normal = delta / dist; // from 1 -> 2
            // Half push each side
            let mut push_mag = overlap * cfg.separation.push_strength * 0.5;
            if push_mag > cfg.separation.max_push { push_mag = cfg.separation.max_push; }
            let push_vec = normal * push_mag;
            pos_shifts.entry(*e1).and_modify(|v| *v -= push_vec).or_insert(-push_vec);
            pos_shifts.entry(*e2).and_modify(|v| *v += push_vec).or_insert(push_vec);

            if cfg.separation.velocity_dampen > 0.0 {
                vel_dampen_normals.entry(*e1).or_default().push(normal);
                vel_dampen_normals.entry(*e2).or_default().push(-normal); // opposite direction for other body
            }
        }
    }

    // Apply position corrections
    for (entity, shift) in pos_shifts.into_iter() {
        if let Ok((_, mut transform, _, _)) = q_rb.get_mut(entity) {
            transform.translation.x += shift.x;
            transform.translation.y += shift.y;
        }
    }

    // Apply velocity damping along normals (project & reduce component)
    if cfg.separation.velocity_dampen > 0.0 {
        for (entity, normals) in vel_dampen_normals.into_iter() {
            if let Ok((_, _transform, vel_opt, _)) = q_rb.get_mut(entity) {
                if let Some(mut vel) = vel_opt {
                    let mut combined = Vec2::ZERO;
                    for n in normals { combined += n; }
                    if combined.length_squared() > 0.0 {
                        let n = combined.normalize();
                        let vn = vel.linvel.dot(n);
                        if vn > 0.0 { // moving along normal; damp it
                            vel.linvel -= n * vn * cfg.separation.velocity_dampen;
                        }
                    }
                }
            }
        }
    }
}
