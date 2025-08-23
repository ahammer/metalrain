#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use super::modes::{DebugState, DebugStats};

#[cfg(feature = "debug")]
pub fn debug_logging_system(time: Res<Time>, mut state: ResMut<DebugState>, stats: Res<DebugStats>) {
    // Periodic SIM line
    state.time_accum += time.delta_secs();
    if state.time_accum >= state.log_interval {
        state.time_accum = 0.0;
        info!(
            "SIM frame={} t={:.3}s fps={:.1} ft_ms={:.1} balls={} clusters={} mode={:?} encoded={}/{} trunc={}",
            state.frame_counter,
            time.elapsed_secs(),
            stats.fps,
            stats.frame_time_ms,
            stats.ball_count,
            stats.cluster_count,
            state.mode,
            stats.metaballs_encoded,
            stats.ball_count,
            stats.truncated_balls
        );
    }

    // ALERT logic: detect spikes. Simple heuristic thresholds:
    // - Frame time spike: current smoothed frame time > 1.5 * 1000 / fps (approx baseline) OR > 33ms (below 30fps) relative to previous baseline.
    // We'll use instantaneous ratio against previous collected frame_time_ms stored in DebugState last_frame_time_alert_frame to rate-limit.
    // - Ball count delta > 15% since last_ball_count.
    // - Cluster count delta > 30% since last_cluster_count.
    // Cooldown: at least 60 frames between alerts of same type.
    let cooldown = 60; // frames
    // Frame time spike: use raw instantaneous approximation from smoothed fps.
    let ft_ms = stats.frame_time_ms;
    // Baseline approximation from fps (avoid div by zero)
    let baseline_ft_ms = if stats.fps > 0.1 { 1000.0 / stats.fps } else { ft_ms };
    if state.frame_counter.saturating_sub(state.last_frame_time_alert_frame) > cooldown
        && ft_ms > baseline_ft_ms * 1.5
        && ft_ms > 20.0
    {
        // only alert if meaningful
        info!(
            "ALERT frame_time_spike frame={} ft_ms={:.2} baseline_ms={:.2} fps={:.1}",
            state.frame_counter, ft_ms, baseline_ft_ms, stats.fps
        );
        state.last_frame_time_alert_frame = state.frame_counter;
    }

    // Ball count spike
    if state.last_ball_count == 0 {
        state.last_ball_count = stats.ball_count;
    } else if state.frame_counter.saturating_sub(state.last_ball_alert_frame) > cooldown {
        let prev = state.last_ball_count as f32;
        let cur = stats.ball_count as f32;
        if prev > 0.0 {
            let delta_ratio = (cur - prev).abs() / prev.max(1.0);
            if delta_ratio > 0.15 { // 15%
                info!(
                    "ALERT ball_count_change frame={} old={} new={} delta={:+} delta_pct={:.1}%",
                    state.frame_counter,
                    state.last_ball_count,
                    stats.ball_count,
                    stats.ball_count as isize - state.last_ball_count as isize,
                    delta_ratio * 100.0
                );
                state.last_ball_alert_frame = state.frame_counter;
                state.last_ball_count = stats.ball_count;
            }
        }
    }

    // Cluster count spike
    if state.last_cluster_count == 0 {
        state.last_cluster_count = stats.cluster_count;
    } else if state.frame_counter.saturating_sub(state.last_cluster_alert_frame) > cooldown {
        let prev = state.last_cluster_count as f32;
        let cur = stats.cluster_count as f32;
        if prev > 0.0 {
            let delta_ratio = (cur - prev).abs() / prev.max(1.0);
            if delta_ratio > 0.30 { // 30%
                info!(
                    "ALERT cluster_count_change frame={} old={} new={} delta={:+} delta_pct={:.1}%",
                    state.frame_counter,
                    state.last_cluster_count,
                    stats.cluster_count,
                    stats.cluster_count as isize - state.last_cluster_count as isize,
                    delta_ratio * 100.0
                );
                state.last_cluster_alert_frame = state.frame_counter;
                state.last_cluster_count = stats.cluster_count;
            }
        }
    }
}
