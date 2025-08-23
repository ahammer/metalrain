//! Emitter system scaffold.
//!
//! Adds runtime spawning of Balls after the initial ring, driven by `GameConfigRes.emitter` and a
//! deterministic RNG seeded from `RngSeed` (separate stream from ring spawning randomness).
//!
//! Simplifications (Phase 4 scaffold):
//! - Frame-based spawn pacing assuming nominal 60 FPS: spawn_rate_per_frame = rate_per_sec / 60.
//! - Accumulates fractional spawn credit to achieve average rate.
//! - Spawns at random polar coordinates within the same bounding radius heuristic as the ring
//!   (0.8 * min(|x_range.max|, |y_range.max|)) with random radii sampled from balls.radius_range.
//! - Deterministic across runs given identical config + `RngSeed` + frame count.
//!
//! Future enhancements (tracked in plan):
//! - Use actual delta time instead of fixed 60 FPS assumption.
//! - Consider spatial distribution (avoid overlaps) and physics warmup integration.
//! - Streaming snapshot tests for spawn sequence parity.
//! - Adaptive rate control based on performance metrics.

use bevy::prelude::*;
use bm_core::{Ball, BallRadius, GameConfigRes, RngSeed};
use rand::{Rng, SeedableRng};

/// Internal resource: deterministic RNG dedicated to emitter (separate from any other RNG usage).
#[derive(Resource)]
pub struct EmitterRng(pub rand::rngs::StdRng);

/// Internal resource: fractional spawn credit accumulator.
#[derive(Resource, Default)]
pub struct SpawnAccumulator {
    pub credit: f32,
}

/// System: initialize emitter RNG & accumulator if emitter is enabled and not already present.
pub fn emitter_init(
    mut commands: Commands,
    cfg: Option<Res<GameConfigRes>>,
    seed: Option<Res<RngSeed>>,
    has_rng: Option<Res<EmitterRng>>,
    has_accum: Option<Res<SpawnAccumulator>>,
) {
    let Some(cfg) = cfg else { return };
    if !cfg.0.emitter.enabled {
        return;
    }
    if has_rng.is_none() {
        // Derive a domain-separated seed (add constant) so sequence differs from ring spawn usage.
        let base = seed.map(|s| s.0).unwrap_or(0);
        // Domain separation constant to ensure emitter RNG stream differs from other seeded uses.
        let rng = rand::rngs::StdRng::seed_from_u64(base.wrapping_add(0xE0E17E77));
        commands.insert_resource(EmitterRng(rng));
    }
    if has_accum.is_none() {
        commands.insert_resource(SpawnAccumulator::default());
    }
}

/// Helper: compute ring-style base radius from config (shared heuristic with spawning.rs).
fn compute_base_ring(cfg: &bm_config::GameConfig) -> f32 {
    let max_x = cfg.balls.x_range.max.abs();
    let max_y = cfg.balls.y_range.max.abs();
    let base = max_x.min(max_y);
    if base > 0.0 { base * 0.8 } else { 120.0 }
}

/// System: spawn new balls according to emitter rate until `max_live` reached.
pub fn emitter_spawn(
    mut commands: Commands,
    cfg: Option<Res<GameConfigRes>>,
    rng_res: Option<ResMut<EmitterRng>>,
    accum: Option<ResMut<SpawnAccumulator>>,
    q_existing: Query<(), With<Ball>>,
) {
    let Some(cfg) = cfg else { return };
    let emitter_cfg = &cfg.0.emitter;
    if !emitter_cfg.enabled {
        return;
    }
    // Require initialized resources (created by emitter_init).
    let (Some(mut rng_res), Some(mut accum)) = (rng_res, accum) else { return };

    let current = q_existing.iter().count();
    if current >= emitter_cfg.max_live {
        return;
    }

    let spawn_rate_per_frame = emitter_cfg.rate_per_sec / 60.0;
    accum.credit += spawn_rate_per_frame;

    let mut to_spawn = accum.credit.floor() as usize;
    accum.credit -= to_spawn as f32;

    if to_spawn == 0 {
        return;
    }

    // Clamp to remaining capacity.
    let remaining_capacity = emitter_cfg.max_live - current;
    if to_spawn > remaining_capacity {
        to_spawn = remaining_capacity;
        // No need to adjust credit further; fractional part preserved.
    }

    let gc = &cfg.0;
    let r_min = gc.balls.radius_range.min.max(0.01);
    let r_max = gc.balls.radius_range.max.max(r_min);
    let ring_r = compute_base_ring(gc);

    for _ in 0..to_spawn {
        let radius = if (r_max - r_min) > f32::EPSILON {
            rng_res.0.gen_range(r_min..=r_max)
        } else {
            r_min
        };
        // Uniform disk sampling: r = R * sqrt(u), theta = 2πv
        let u: f32 = rng_res.0.gen();
        let v: f32 = rng_res.0.gen();
        let radial = ring_r * u.sqrt();
        let angle = v * std::f32::consts::TAU;
        let x = radial * angle.cos();
        let y = radial * angle.sin();
        commands.spawn((
            Ball,
            BallRadius(radius),
            Transform::from_xyz(x, y, 0.0),
            GlobalTransform::default(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, MinimalPlugins, Transform, World};

    fn count_balls(world: &mut World) -> usize {
        let mut q = world.query::<&Ball>();
        q.iter(world).count()
    }

    #[test]
    fn emitter_respects_max_live() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut cfg = bm_config::GameConfig::default();
        cfg.emitter.enabled = true;
        cfg.emitter.rate_per_sec = 120.0; // 2 per frame under 60 fps assumption
        cfg.emitter.max_live = 20;
        cfg.balls.count = 0; // no initial ring (we will not add ring system in this isolated test)
        app.insert_resource(GameConfigRes(cfg));
        app.insert_resource(RngSeed(5));
        app.add_systems(Startup, emitter_init);
        app.add_systems(Update, emitter_spawn);

        // Advance enough frames to exceed capacity if unchecked.
        for _ in 0..50 {
            app.update();
        }
        let world = app.world_mut();
        assert_eq!(count_balls(world), 20, "emitter should cap at max_live");
    }

    #[test]
    fn deterministic_sequence_same_seed() {
        let mut cfg = bm_config::GameConfig::default();
        cfg.emitter.enabled = true;
        cfg.emitter.rate_per_sec = 60.0; // 1 per frame
        cfg.emitter.max_live = 25;
        cfg.balls.count = 0;

        let run_with_seed = |seed: u64| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.insert_resource(GameConfigRes(cfg.clone()));
            app.insert_resource(RngSeed(seed));
            app.add_systems(Startup, emitter_init);
            app.add_systems(Update, emitter_spawn);
            for _ in 0..30 {
                app.update();
            }
            let world = app.world_mut();
            let mut q = world.query::<(&Transform, &BallRadius)>();
            let mut data: Vec<(i32, i32, u32)> = q
                .iter(world)
                .map(|(t, r)| {
                    // Quantize to reduce floating noise risk across platforms
                    ((t.translation.x * 100.0) as i32, (t.translation.y * 100.0) as i32, (r.0 * 100.0) as u32)
                })
                .collect();
            data.sort();
            data
        };

        let a = run_with_seed(1234);
        let b = run_with_seed(1234);
        assert_eq!(a, b, "same seed should yield identical sequence");
        let c = run_with_seed(1235);
        assert_ne!(a, c, "different seed should differ");
    }
}
