// Phase 2 (in progress): Physics baseline
// Adds: Rapier setup (zero global gravity), radial gravity force system, separation adjustment system,
// pure helpers (radial_gravity_delta, compute_pair_push) and unit tests.
// Systems are wired into PrePhysicsSet and PostPhysicsAdjustSet defined in bm_core.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bm_core::{
    Ball, BallRadius, GameConfigRes, PostPhysicsAdjustSet, PrePhysicsSet,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Rapier setup (match legacy: default scaling of 1.0; scaling previously 100.0 caused parity drift).
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());

        app.add_systems(Startup, configure_gravity);
        app.add_systems(
            Update,
            apply_radial_gravity.in_set(PrePhysicsSet),
        );
        app.add_systems(
            Update,
            contact_separation.in_set(PostPhysicsAdjustSet),
        );
    }
}

// ---------------- Radial Gravity ----------------

fn apply_radial_gravity(
    cfg: Res<GameConfigRes>,
    mut q: Query<(&Transform, &mut Velocity), With<Ball>>,
    time: Res<Time>,
) {
    let g = cfg.0.gravity.y.abs();
    if g <= 0.0 {
        return;
    }
    let dt = time.delta_secs();
    for (transform, mut vel) in q.iter_mut() {
        let pos = transform.translation.truncate();
        vel.linvel += radial_gravity_delta(g, dt, pos);
    }
}

/// Pure helper: velocity delta toward origin for one integration step.
pub fn radial_gravity_delta(g: f32, dt: f32, pos: Vec2) -> Vec2 {
    if g <= 0.0 || dt <= 0.0 {
        return Vec2::ZERO;
    }
    if pos.length_squared() < 1e-6 {
        return Vec2::ZERO;
    }
    let dir_to_center = -pos.normalize();
    dir_to_center * g * dt
}

// ---------------- Gravity Configuration (Rapier) ----------------

fn configure_gravity(mut q_cfg: Query<&mut RapierConfiguration>) {
    if let Ok(mut cfg) = q_cfg.single_mut() {
        // Use custom radial gravity system; zero out global gravity.
        cfg.gravity = Vect::new(0.0, 0.0);
    }
}

// ---------------- Separation (Lightweight Post-Physics Adjustment) ----------------

fn contact_separation(
    mut ev_collisions: EventReader<CollisionEvent>,
    cfg_res: Res<GameConfigRes>,
    mut q_rb: Query<(Entity, &mut Transform, Option<&mut Velocity>, &BallRadius), With<Ball>>,
) {
    let sep_cfg = &cfg_res.0.separation;
    if !sep_cfg.enabled {
        return;
    }

    use std::collections::HashMap;
    let mut pos_shifts: HashMap<Entity, Vec2> = HashMap::new();
    let mut vel_normals: HashMap<Entity, Vec2> = HashMap::new();

    for ev in ev_collisions.read() {
        let CollisionEvent::Started(e1, e2, _flags) = ev else {
            continue;
        };
        let Ok((_, t1, _, r1)) = q_rb.get(*e1) else {
            continue;
        };
        let Ok((_, t2, _, r2)) = q_rb.get(*e2) else {
            continue;
        };

        let p1 = t1.translation.truncate();
        let p2 = t2.translation.truncate();
        let delta = p2 - p1;
        let dist_sq = delta.length_squared();
        if dist_sq < 1e-6 {
            continue;
        }
        let dist = dist_sq.sqrt();
        if let Some((push_vec, normal)) =
            compute_pair_push(dist, r1.0, r2.0, sep_cfg, delta)
        {
            pos_shifts
                .entry(*e1)
                .and_modify(|v| *v -= push_vec)
                .or_insert(-push_vec);
            pos_shifts
                .entry(*e2)
                .and_modify(|v| *v += push_vec)
                .or_insert(push_vec);

            if sep_cfg.velocity_dampen > 0.0 {
                vel_normals
                    .entry(*e1)
                    .and_modify(|v| *v += normal)
                    .or_insert(normal);
                vel_normals
                    .entry(*e2)
                    .and_modify(|v| *v -= normal)
                    .or_insert(-normal);
            }
        }
    }

    for (entity, shift) in pos_shifts.into_iter() {
        if let Ok((_, mut transform, _, _)) = q_rb.get_mut(entity) {
            transform.translation.x += shift.x;
            transform.translation.y += shift.y;
        }
    }

    if sep_cfg.velocity_dampen > 0.0 {
        for (entity, combined) in vel_normals.into_iter() {
            if let Ok((_, _tf, Some(mut vel), _)) = q_rb.get_mut(entity) {
                if combined.length_squared() > 0.0 {
                    let n = combined.normalize();
                    let vn = vel.linvel.dot(n);
                    if vn > 0.0 {
                        vel.linvel -= n * vn * sep_cfg.velocity_dampen;
                    }
                }
            }
        }
    }
}

/// Computes per-pair push (half overlap * strength capped by max_push) and collision normal.
pub fn compute_pair_push(
    dist: f32,
    r1: f32,
    r2: f32,
    cfg: &bm_config::CollisionSeparationConfig,
    delta: Vec2,
) -> Option<(Vec2, Vec2)> {
    if dist <= 0.0 {
        return None;
    }
    let target = (r1 + r2) * cfg.overlap_slop;
    if dist >= target {
        return None;
    }
    let overlap = target - dist;
    if overlap <= 0.0 {
        return None;
    }
    let normal = delta / dist;
    let mut push_mag = overlap * cfg.push_strength * 0.5;
    if push_mag > cfg.max_push {
        push_mag = cfg.max_push;
    }
    let push_vec = normal * push_mag;
    Some((push_vec, normal))
}

// ---------------- Tests ----------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_gravity_delta_points_inward() {
        let g = 100.0;
        let dt = 0.016;
        let pos = Vec2::new(100.0, 0.0);
        let delta = radial_gravity_delta(g, dt, pos);
        assert!(delta.x < 0.0, "expected inward x delta, got {delta:?}");
        assert!(delta.length() > 0.0);
    }

    #[test]
    fn radial_gravity_zero_cases() {
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

    #[test]
    fn compute_pair_push_basic() {
        let cfg = bm_config::CollisionSeparationConfig {
            enabled: true,
            overlap_slop: 0.99,
            push_strength: 1.0,
            max_push: 100.0,
            velocity_dampen: 0.5,
        };
        let r = 10.0;
        let dist = 18.0; // radii sum 20 -> target 19.8 -> overlap 1.8 -> half push 0.9
        let delta = Vec2::new(18.0, 0.0);
        let (push_vec, normal) =
            compute_pair_push(dist, r, r, &cfg, delta).expect("should overlap");
        assert!(
            (push_vec.length() - 0.9).abs() < 1e-3,
            "expected ~0.9 push each, got {push_vec:?}"
        );
        assert_eq!(normal, Vec2::X);
    }
}
