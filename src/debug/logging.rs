#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use super::modes::{DebugState, DebugStats};

#[cfg(feature = "debug")]
pub fn debug_logging_system(time: Res<Time>, mut state: ResMut<DebugState>, stats: Res<DebugStats>) {
    state.time_accum += time.delta_secs();
    if state.time_accum >= state.log_interval {
        state.time_accum = 0.0;
        info!("SIM frame={} t={:.3}s fps={:.1} ft_ms={:.1} balls={} clusters={} mode={:?} encoded={}/{} trunc={}",
            state.frame_counter,
            time.elapsed_secs(),
            stats.fps,
            stats.frame_time_ms,
            stats.ball_count,
            stats.cluster_count,
            state.mode,
            stats.metaballs_encoded,
            stats.ball_count,
            stats.truncated_balls);
    }
}
