#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use super::modes::{DebugStats, DebugState};

#[cfg(feature = "debug")]
pub fn debug_overlay_spawn(_commands: Commands, _asset_server: Res<AssetServer>) {
    // Placeholder: in-engine text overlay not yet implemented for Bevy 0.16 migration.
    info!("Debug overlay placeholder active");
}

#[cfg(feature = "debug")]
pub fn debug_overlay_update(state: Res<DebugState>, stats: Res<DebugStats>) {
    if !state.overlay_visible { return; }
    if state.is_changed() || stats.is_changed() {
        info!(
            "OVERLAY fps={:.1} ft_ms={:.1} balls={} enc={}/{} clusters={} mode={:?}",
            stats.fps,
            stats.frame_time_ms,
            stats.ball_count,
            stats.metaballs_encoded,
            stats.ball_count,
            stats.cluster_count,
            state.mode
        );
    }
}
