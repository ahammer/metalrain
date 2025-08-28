use bevy::prelude::*;
use crate::core::components::{Ball, BallState};
use crate::core::config::GameConfig;
use crate::core::system::system_order::PostPhysicsAdjustSet;
use crate::physics::clustering::cluster::{Clusters, compute_clusters};

/// System set label so other systems (metaball material update) can order after ball state classification.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BallStateUpdateSet;

/// Plugin managing per-ball enabled/disabled state classification and timestamps.
pub struct BallStatePlugin;

impl Plugin for BallStatePlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets(Update, BallStateUpdateSet.in_set(PostPhysicsAdjustSet).after(compute_clusters))
            .add_systems(
                Update,
                update_ball_states
                    .in_set(BallStateUpdateSet),
            )
            .init_resource::<OverflowLogged>();
    }
}

/// Tracks whether MAX_CLUSTERS overflow fallback has been logged to avoid spam.
#[derive(Resource, Default)]
pub struct OverflowLogged(pub bool);

/// Update (or lazily insert) BallState for all balls based on cluster size/area thresholds.
/// Runs after clustering so clusters & their aggregates are current.
fn update_ball_states(
    mut commands: Commands,
    time: Res<Time>,
    clusters: Res<Clusters>,
    cfg: Res<GameConfig>,
    mut q: Query<(Entity, Option<&mut BallState>), With<Ball>>,
) {
    if clusters.0.is_empty() {
        return;
    }
    let cp = &cfg.interactions.cluster_pop;
    let now = time.elapsed_secs();

    // Collect missing inserts (cannot insert while holding &mut query items in first pass).
    let mut to_insert: Vec<(Entity, bool)> = Vec::new();

    for cl in clusters.0.iter() {
        let enabled = cl.entities.len() >= cp.min_ball_count && cl.total_area >= cp.min_total_area;
        for &e in &cl.entities {
            if let Ok((_, maybe_state)) = q.get_mut(e) {
                if let Some(mut st) = maybe_state {
                    if st.enabled != enabled {
                        st.enabled = enabled;
                        st.last_change = now;
                        info!(
                            target: "ball_state",
                            "Ball {:?} -> {}",
                            e,
                            if enabled { "Enabled" } else { "Disabled" }
                        );
                    }
                } else {
                    to_insert.push((e, enabled));
                }
            }
        }
    }

    for (e, enabled) in to_insert {
        commands.entity(e).insert(BallState {
            enabled,
            last_change: now,
        });
        // Log only on disabled initial insert to reduce noise.
        if !enabled {
            info!(target: "ball_state", "Ball {:?} inserted Disabled", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_initializes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BallStatePlugin);
        assert!(app.world().contains_resource::<OverflowLogged>());
    }

    #[test]
    fn inserts_and_toggles_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Minimal resources required
        let mut cfg = GameConfig::default();
        cfg.interactions.cluster_pop.min_ball_count = 1; // Adjust for single-ball enabled test
        app.insert_resource(cfg);
        app.insert_resource(Clusters(vec![]));
        app.add_plugins(BallStatePlugin);

        // Spawn a ball & fake cluster
        let e = app.world_mut().spawn((Ball,)).id();
        {
            let mut clusters = app.world_mut().resource_mut::<Clusters>();
            clusters.0.push(crate::physics::clustering::cluster::Cluster {
                color_index: 0,
                entities: vec![e],
                min: Vec2::ZERO,
                max: Vec2::ZERO,
                centroid: Vec2::ZERO,
                total_area: 2000.0,
            });
        }
        // Run schedules
        app.update();
        // BallState inserted
        let st = app.world().get::<BallState>(e).unwrap();
        assert!(st.enabled);

        // Make cluster too small
        {
            let mut clusters = app.world_mut().resource_mut::<Clusters>();
            clusters.0[0].total_area = 0.0;
            clusters.0[0].entities = vec![e];
        }
        app.update();
        let st2 = app.world().get::<BallState>(e).unwrap();
        assert!(!st2.enabled);
    }
}
