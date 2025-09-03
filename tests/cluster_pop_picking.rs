use ball_matcher::core::components::{Ball, BallRadius};
use ball_matcher::core::config::{ClusterPopConfig, GameConfig};
use ball_matcher::interaction::cluster_pop::{pick_ball_cluster, PaddleLifecycle};
use ball_matcher::physics::clustering::cluster::{BallClusterIndex, ClusterCorePlugin, Clusters};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

fn base_cluster_pop() -> ClusterPopConfig {
    ClusterPopConfig {
        enabled: true,
        min_ball_count: 1,
        min_total_area: 0.0,
        peak_scale: 1.2,
        grow_duration: 0.05,
        hold_duration: 0.0,
        shrink_duration: 0.05,
        collider_scale_curve: 0,
        freeze_mode: 0,
        fade_alpha: false,
        fade_curve: 0,
        ball_pick_radius: 30.0,
        ball_pick_radius_scale_with_ball: true,
        prefer_larger_radius_on_tie: true,
        exclude_from_new_clusters: false,
        impulse: None,
        outward_bonus: None,
        despawn_delay: None,
        fade_duration: None,
        fade_scale_end: None,
        collider_shrink: None,
        collider_min_scale: None,
        velocity_damping: None,
        spin_jitter: None,
    }
}

fn test_app(modify: impl FnOnce(&mut ClusterPopConfig)) -> App {
    let mut cfg = GameConfig::default();
    let mut cp = base_cluster_pop();
    modify(&mut cp);
    cfg.interactions.cluster_pop = cp;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(cfg);
    // Not running full physics; velocities only
    app.add_plugins(ClusterCorePlugin);
    // Do not add full ClusterPopPlugin; we test pick_ball_cluster directly to avoid mouse input resource requirements
    app
}

fn spawn_ball(app: &mut App, pos: Vec2, radius: f32, color_index: usize) -> Entity {
    use ball_matcher::rendering::materials::materials::BallMaterialIndex;
    app.world_mut()
        .spawn((
            Ball,
            BallRadius(radius),
            BallMaterialIndex(color_index),
            Transform::from_xyz(pos.x, pos.y, 0.0),
            GlobalTransform::default(),
            Velocity::zero(),
        ))
        .id()
}

fn force_clusters(app: &mut App) {
    app.update();
}

fn perform_pick(app: &mut App, world_pos: Vec2) -> Option<usize> {
    // Build a system-like query state to pass
    let mut entities: Vec<(Entity, Transform, BallRadius, bool)> = Vec::new();
    let world = app.world_mut();
    // Manual iteration
    for (entity, tf, br, pl) in world
        .query::<(Entity, &Transform, &BallRadius, Option<&PaddleLifecycle>)>()
        .iter(world)
    {
        entities.push((entity, *tf, *br, pl.is_some()));
    }
    let clusters = world.resource::<Clusters>().clone();
    let index = world.resource::<BallClusterIndex>().clone();
    let cfg = world.resource::<GameConfig>().clone();
    // Build stable storage of fake lifecycle refs
    let lifecycle_store: Vec<Option<PaddleLifecycle>> = entities
        .iter()
        .map(|(_, _, r, has)| {
            if *has {
                Some(PaddleLifecycle {
                    elapsed: 0.0,
                    grow_duration: 0.0,
                    hold_duration: 0.0,
                    shrink_duration: 0.0,
                    peak_scale: 1.0,
                    freeze_mode:
                        ball_matcher::interaction::cluster_pop::FreezeMode::ZeroVelEachFrame,
                    base_radius: r.0,
                    fade_alpha: false,
                    fade_curve: 0,
                    collider_scale_curve: 0,
                    alpha_base: 0.0,
                })
            } else {
                None
            }
        })
        .collect();
    let iter = entities
        .iter()
        .zip(lifecycle_store.iter())
        .map(|(data, plc)| {
            let (e, t, r, _has) = data;
            (*e, t, r, plc.as_ref())
        });
    if let Some((_ball, ci, _r, _d2)) = pick_ball_cluster(
        world_pos,
        &clusters,
        &index,
        iter,
        &cfg.interactions.cluster_pop,
    ) {
        Some(ci)
    } else {
        None
    }
}

#[test]
fn hit_single_ball() {
    let mut app = test_app(|_| {});
    spawn_ball(&mut app, Vec2::ZERO, 10.0, 0);
    force_clusters(&mut app);
    let picked = perform_pick(&mut app, Vec2::new(1.0, 1.0));
    assert!(
        picked.is_some(),
        "Expected to pick the single ball's cluster"
    );
}

#[test]
fn no_hit_outside_radius() {
    let mut app = test_app(|cp| {
        cp.ball_pick_radius = 20.0;
    });
    spawn_ball(&mut app, Vec2::ZERO, 10.0, 0);
    force_clusters(&mut app);
    let picked = perform_pick(&mut app, Vec2::new(100.0, 0.0));
    assert!(picked.is_none(), "Should not pick when far away");
}

#[test]
fn overlapping_aabb_regression() {
    // Two clusters whose AABBs would overlap if legacy heuristic used; ensure local ball proximity wins irrespective of size
    let mut app = test_app(|_| {});
    // Big cluster (farther from pick point) color 0
    for i in 0..5 {
        spawn_ball(&mut app, Vec2::new(200.0 + i as f32 * 5.0, 0.0), 12.0, 0);
    }
    // Small cluster near origin color 1
    spawn_ball(&mut app, Vec2::new(5.0, 0.0), 10.0, 1);
    force_clusters(&mut app);
    let picked = perform_pick(&mut app, Vec2::new(5.5, 0.5)).expect("must pick a cluster");
    let clusters = app.world().get_resource::<Clusters>().unwrap();
    assert_eq!(
        clusters.0[picked].color_index, 1,
        "Should pick smaller nearby cluster, not larger overlapping AABB"
    );
}

#[test]
fn distance_tie_larger_radius_preferred() {
    let mut app = test_app(|cp| {
        cp.prefer_larger_radius_on_tie = true;
        cp.ball_pick_radius_scale_with_ball = false;
        cp.ball_pick_radius = 50.0;
    });
    let left_small = spawn_ball(&mut app, Vec2::new(-10.0, 0.0), 8.0, 0);
    let right_large = spawn_ball(&mut app, Vec2::new(10.0, 0.0), 12.0, 0); // same color => same cluster? place far enough to not touch
    force_clusters(&mut app);
    // Ensure they are separate clusters by using different colors instead (tie logic independent of color). Re-spawn with distinct colors if merged.
    {
        let clusters = app.world().get_resource::<Clusters>().unwrap();
        if clusters.0.len() == 1 {
            // merged -> rebuild with diff colors
            app.world_mut().despawn(left_small);
            app.world_mut().despawn(right_large);
            spawn_ball(&mut app, Vec2::new(-10.0, 0.0), 8.0, 0);
            spawn_ball(&mut app, Vec2::new(10.0, 0.0), 12.0, 1);
            force_clusters(&mut app);
        }
    }
    let picked = perform_pick(&mut app, Vec2::new(0.0, 0.0)).expect("must pick a cluster");
    let clusters = app.world().get_resource::<Clusters>().unwrap();
    let chosen = &clusters.0[picked];
    // The chosen cluster should correspond to the larger radius ball (color 1 if respawn path executed)
    assert!(
        chosen.entities.iter().any(|&e| {
            let br = app.world().get::<BallRadius>(e).unwrap().0;
            br >= 12.0 - 1e-4
        }),
        "Expected cluster of larger radius ball"
    );
}

#[test]
fn distance_tie_without_radius_pref_cluster_size_breaker() {
    let mut app = test_app(|cp| {
        cp.prefer_larger_radius_on_tie = false;
        cp.ball_pick_radius_scale_with_ball = false;
        cp.ball_pick_radius = 50.0;
    });
    // Create two clusters equidistant from origin; make one cluster have more balls
    spawn_ball(&mut app, Vec2::new(-10.0, 0.0), 10.0, 0); // cluster A (single)
    spawn_ball(&mut app, Vec2::new(10.0, 0.0), 10.0, 1); // cluster B start
    spawn_ball(&mut app, Vec2::new(13.0, 0.0), 10.0, 1); // cluster B second (touching)
    force_clusters(&mut app);
    let picked = perform_pick(&mut app, Vec2::new(0.0, 0.0)).expect("must pick a cluster");
    let clusters = app.world().get_resource::<Clusters>().unwrap();
    assert_eq!(
        clusters.0[picked].color_index, 1,
        "Expected larger cluster to win tie"
    );
}
