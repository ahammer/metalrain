#[cfg(feature = "debug")]
use super::modes::{DebugState, DebugStats};
#[cfg(feature = "debug")]
use crate::core::components::Ball;
#[cfg(feature = "debug")]
use crate::physics::clustering::cluster::Clusters;
#[cfg(feature = "debug")]
use crate::rendering::metaballs::MAX_BALLS;
#[cfg(feature = "debug")]
use bevy::prelude::*;

#[cfg(feature = "debug")]
pub fn debug_stats_collect_system(
    time: Res<Time>,
    mut state: ResMut<DebugState>,
    mut stats: ResMut<DebugStats>,
    q_balls: Query<&Ball>,
    clusters: Res<Clusters>,
) {
    state.frame_counter += 1;
    let dt = time.delta_secs().max(1e-6);
    let inst_fps = 1.0 / dt;
    if stats.fps == 0.0 {
        stats.fps = inst_fps;
    } else {
        stats.fps = stats.fps * 0.9 + inst_fps * 0.1;
    }
    let inst_ms = dt * 1000.0;
    if stats.frame_time_ms == 0.0 {
        stats.frame_time_ms = inst_ms;
    } else {
        stats.frame_time_ms = stats.frame_time_ms * 0.9 + inst_ms * 0.1;
    }
    let ball_count = q_balls.iter().count();
    let cluster_count = clusters.0.len();
    stats.ball_count = ball_count;
    stats.cluster_count = cluster_count;
    stats.truncated_balls = ball_count > MAX_BALLS;
    stats.metaballs_encoded = ball_count.min(MAX_BALLS);
}
