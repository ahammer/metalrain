//! Debug module: feature gated runtime visualization & stats/logging.
//! Built only when compiled with `--features debug`.

#[cfg(feature = "debug")]
pub mod keys; // pub for testing
#[cfg(feature = "debug")]
mod logging;
#[cfg(feature = "debug")]
mod modes;
#[cfg(feature = "debug")]
mod overlay;
#[cfg(feature = "debug")]
mod stats;

#[cfg(feature = "debug")]
pub use modes::*;

#[cfg(feature = "debug")]
use crate::core::system::system_order::PostPhysicsAdjustSet;
#[cfg(feature = "debug")]
use bevy::prelude::*;
// Legacy spawn & spawn-related materials imports removed.

#[cfg(feature = "debug")]
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DebugPreRenderSet;

#[cfg(feature = "debug")]
pub struct DebugPlugin;
#[cfg(feature = "debug")]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "debug")]
        use keys::debug_key_input_system;
        use logging::debug_logging_system;
        use modes::apply_mode_visual_overrides_system;
        use modes::propagate_metaballs_view_system;
        #[cfg(not(test))]
        use overlay::{debug_config_overlay_update, debug_overlay_spawn, debug_overlay_update};
        use stats::debug_stats_collect_system;


        // Removed debug_input_gizmos system (touch circle + drag line) per user request.

        app.init_resource::<modes::DebugState>()
            .init_resource::<modes::DebugStats>()
            .init_resource::<modes::DebugVisualOverrides>()
            .init_resource::<modes::LastAppliedMetaballsView>()
            .configure_sets(Update, DebugPreRenderSet.after(PostPhysicsAdjustSet));
        // In tests, skip overlay spawn (AssetServer not present with MinimalPlugins)
        #[cfg(not(test))]
        app.add_systems(Startup, debug_overlay_spawn);
        app.add_systems(
            Update,
            (
                debug_key_input_system,
                debug_stats_collect_system,
                apply_mode_visual_overrides_system,
                propagate_metaballs_view_system,
                debug_logging_system,
                #[cfg(not(test))]
                debug_config_overlay_update,
                #[cfg(not(test))]
                debug_overlay_update,
                // spawn button removed with legacy spawn system
            )
                .in_set(DebugPreRenderSet),
        );
    }
}

#[cfg(not(feature = "debug"))]
pub struct DebugPlugin;
#[cfg(not(feature = "debug"))]
impl bevy::prelude::Plugin for DebugPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}
