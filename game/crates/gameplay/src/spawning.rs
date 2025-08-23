//! Ring spawning scaffold (Phase 4 start).
//!
//! Temporary implementation to validate gameplay crate integration:
//! - Spawns a ring of Balls at startup with uniform angular distribution.
//! - BallRadius uniform placeholder (later: distribution from config).
//!
//! Future enhancements (tracked in plan.md):
//! - Load spawning parameters from GameConfigRes (counts, radius range, ring radius).
//! - Seeded RNG-driven jitter & varying radii (deterministic under tests).
//! - Separate startup ring vs runtime emitter systems (with rate control).
//! - Snapshot tests for deterministic layout given fixed seed.

use bevy::prelude::*;
use bm_core::{Ball, BallRadius, GameConfigRes, RngSeed};
use rand::{Rng, SeedableRng};

/// Placeholder: number of balls in startup ring.
pub const START_RING_COUNT: usize = 12;
/// Placeholder: ring radius (world units).
pub const START_RING_RADIUS: f32 = 120.0;
/// Placeholder: each ball logical radius.
pub const START_BALL_RADIUS: f32 = 6.0;

/// System: spawn initial ring of balls once at startup.
///
/// Dynamic behavior:
/// - If a `GameConfigRes` is present, derive parameters from it:
///     * count = cfg.balls.count
///     * ring radius = 0.8 * min(abs(x_range.max), abs(y_range.max)) (fallback to START_RING_RADIUS if ranges degenerate)
///     * ball radius = midpoint(cfg.balls.radius_range) (fallback to START_BALL_RADIUS if invalid)
/// - Otherwise fallback to placeholder constants.
///
/// Future:
/// - Seeded RNG jitter & per-ball radius variation.
/// - Separate emitter for runtime spawning.
///   Accepts optional GameConfigRes and RngSeed to drive deterministic spawning & variation.
pub fn spawn_initial_ring(
    mut commands: Commands,
    cfg: Option<Res<GameConfigRes>>,
    rng_seed: Option<Res<RngSeed>>,
) {
    // Derive base parameters from config if present.
    let (count, ring_r, r_min, r_max) = if let Some(cfg) = cfg {
        let gc = &cfg.0;
        let count = gc.balls.count.max(1);
        let max_x = gc.balls.x_range.max.abs();
        let max_y = gc.balls.y_range.max.abs();
        let base_ring_extent = max_x.min(max_y);
        let ring_r = if base_ring_extent > 0.0 {
            base_ring_extent * 0.8
        } else {
            START_RING_RADIUS
        };
        let r_min = gc.balls.radius_range.min;
        let r_max = gc.balls.radius_range.max.max(r_min);
        (count, ring_r, r_min, r_max)
    } else {
        (
            START_RING_COUNT,
            START_RING_RADIUS,
            START_BALL_RADIUS,
            START_BALL_RADIUS,
        )
    };

    // Initialize deterministic RNG if seed resource provided.
    let mut rng_opt = rng_seed
        .map(|s| rand::rngs::StdRng::seed_from_u64(s.0))
        .or(None);

    let step = std::f32::consts::TAU / count as f32;
    // Optional global angular offset (for variation) only if rng present.
    let global_offset = rng_opt
        .as_mut()
        .map(|r| r.gen::<f32>() * step)
        .unwrap_or(0.0);

    for i in 0..count {
        // Sample per-ball radius if variability exists and rng is available.
        let radius = if (r_max - r_min) > f32::EPSILON {
            if let Some(rng) = rng_opt.as_mut() {
                rng.gen_range(r_min..=r_max)
            } else {
                (r_min + r_max) * 0.5
            }
        } else {
            r_min
        };

        let angle = global_offset + step * i as f32;
        // Optional slight radial jitter (<= 5% of ring_r) if rng present.
        let radial_jitter = rng_opt
            .as_mut()
            .map(|r| r.gen_range(-0.05..=0.05) * ring_r)
            .unwrap_or(0.0);

        let r_actual = (ring_r + radial_jitter).max(1.0);
        let x = r_actual * angle.cos();
        let y = r_actual * angle.sin();

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

    fn collect_positions_and_radii(world: &mut World) -> Vec<(f32, f32, f32)> {
        let mut q = world.query::<(&Transform, &BallRadius)>();
        let mut v: Vec<(f32, f32, f32)> = q
            .iter(world)
            .map(|(t, r)| (t.translation.x, t.translation.y, r.0))
            .collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        v
    }

    #[test]
    fn ring_spawns_expected_count_from_config() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut cfg = bm_config::GameConfig::default();
        cfg.balls.count = 17;
        cfg.balls.x_range.max = 250.0;
        cfg.balls.y_range.max = 200.0;
        cfg.balls.radius_range.min = 5.0;
        cfg.balls.radius_range.max = 9.0;
        app.insert_resource(GameConfigRes(cfg));
        app.insert_resource(RngSeed(42));
        app.add_systems(Startup, spawn_initial_ring);
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&Ball>();
        assert_eq!(q.iter(world).count(), 17, "expected config-driven count");
    }

    #[test]
    fn ring_spawns_with_fallback_constants_when_no_config() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Startup, spawn_initial_ring);
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<&Ball>();
        assert_eq!(
            q.iter(world).count(),
            START_RING_COUNT,
            "fallback constant count mismatch"
        );
    }

    #[test]
    fn deterministic_with_same_seed() {
        // First run with seed 123
        let mut app1 = App::new();
        app1.add_plugins(MinimalPlugins);
        let mut cfg = bm_config::GameConfig::default();
        cfg.balls.count = 10;
        cfg.balls.x_range.max = 300.0;
        cfg.balls.y_range.max = 300.0;
        cfg.balls.radius_range.min = 4.0;
        cfg.balls.radius_range.max = 12.0;
        app1.insert_resource(GameConfigRes(cfg.clone()));
        app1.insert_resource(RngSeed(123));
        app1.add_systems(Startup, spawn_initial_ring);
        app1.update();
        let mut app2 = App::new();
        app2.add_plugins(MinimalPlugins);
        app2.insert_resource(GameConfigRes(cfg));
        app2.insert_resource(RngSeed(123));
        app2.add_systems(Startup, spawn_initial_ring);
        app2.update();

        let w1 = app1.world_mut();
        let w2 = app2.world_mut();
        let v1 = collect_positions_and_radii(w1);
        let v2 = collect_positions_and_radii(w2);
        assert_eq!(v1, v2, "expected identical spawn layout with same seed");
    }

    #[test]
    fn different_seed_changes_layout() {
        let mut cfg = bm_config::GameConfig::default();
        cfg.balls.count = 10;
        cfg.balls.x_range.max = 300.0;
        cfg.balls.y_range.max = 300.0;
        cfg.balls.radius_range.min = 4.0;
        cfg.balls.radius_range.max = 12.0;

        let layout_for_seed = |seed: u64| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.insert_resource(GameConfigRes(cfg.clone()));
            app.insert_resource(RngSeed(seed));
            app.add_systems(Startup, spawn_initial_ring);
            app.update();
            collect_positions_and_radii(app.world_mut())
        };

        let a = layout_for_seed(7);
        let b = layout_for_seed(8);
        assert_ne!(a, b, "different seeds should produce different layout");
    }
}
