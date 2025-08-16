use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::components::{Ball, BallRadius};

// Simple O(n^2) separation / repulsion pass. For moderate ball counts this is fine.
// Applies a velocity adjustment pushing balls apart if their distance is less than a threshold
// (sum of radii * overlap_slop). Runs after Rapier step so we read current positions.
pub struct SeparationPlugin;

impl Plugin for SeparationPlugin {
    fn build(&self, app: &mut App) {
    // Run after physics; we can approximate by ordering after the built-in FixedUpdate step.
    // Simplicity: just run each Update (Rapier already updated velocities/positions at start of frame).
    app.add_systems(Update, apply_separation);
    }
}

// Tunables (could be moved to config later)
const OVERLAP_SLOP: f32 = 0.98; // start pushing just before actual overlap (<1.0 earlier)
const PUSH_STRENGTH: f32 = 0.5; // scales impulse magnitude
const MAX_IMPULSE: f32 = 200.0; // clamp for stability

fn apply_separation(
    mut bodies: Query<(Entity, &mut Velocity, &Transform, &BallRadius), With<Ball>>,
) {
    // Snapshot positions & radii first (avoid mutable borrow conflicts)
    let snapshot: Vec<(Entity, Vec2, f32)> = bodies
        .iter()
        .map(|(e, _vel, t, r)| (e, t.translation.truncate(), r.0))
        .collect();

    // Accumulate impulses
    let mut impulses: Vec<(Entity, Vec2)> = snapshot.iter().map(|(e, _, _)| (*e, Vec2::ZERO)).collect();

    for i in 0..snapshot.len() {
        for j in (i + 1)..snapshot.len() {
            let (ei, pi, ri) = snapshot[i];
            let (ej, pj, rj) = snapshot[j];
            let delta = pj - pi;
            let dist_sq = delta.length_squared();
            if dist_sq < 0.0001 { continue; }
            let target = (ri + rj) * OVERLAP_SLOP;
            let target_sq = target * target;
            if dist_sq < target_sq {
                let dist = dist_sq.sqrt();
                let dir = delta / dist; // i -> j
                let overlap = target - dist; // positive inside threshold
                if overlap > 0.0 {
                    let mut push = dir * (overlap * PUSH_STRENGTH);
                    if push.length_squared() > MAX_IMPULSE * MAX_IMPULSE {
                        push = push.clamp_length_max(MAX_IMPULSE);
                    }
                    // apply opposite impulses
                    if let Some(a) = impulses.iter_mut().find(|(e, _)| *e == ei) { a.1 -= push; }
                    if let Some(b) = impulses.iter_mut().find(|(e, _)| *e == ej) { b.1 += push; }
                }
            }
        }
    }

    // Apply impulses by adjusting linear velocity
    for (entity, impulse) in impulses.into_iter() {
        if impulse != Vec2::ZERO {
            if let Ok((_e, mut vel, _t, _r)) = bodies.get_mut(entity) {
                vel.linvel += impulse; // direct velocity tweak; lightweight
            }
        }
    }
}
