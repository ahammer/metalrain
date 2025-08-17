//! Debug module: feature gated runtime visualization & stats/logging.
//! Built only when compiled with `--features debug`.

#[cfg(feature = "debug")]
mod modes;
#[cfg(feature = "debug")]
mod keys;
#[cfg(feature = "debug")]
mod stats;
#[cfg(feature = "debug")]
mod logging;
#[cfg(feature = "debug")]
mod overlay;

#[cfg(feature = "debug")]
pub use modes::*;

#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use crate::system_order::PostPhysicsAdjustSet;

#[cfg(feature = "debug")]
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DebugPreRenderSet;

#[cfg(feature = "debug")]
pub struct DebugPlugin;
#[cfg(feature = "debug")]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        use keys::debug_key_input_system;
        use logging::debug_logging_system;
        use overlay::{debug_overlay_spawn, debug_overlay_update};
        use stats::debug_stats_collect_system;
        use modes::apply_mode_visual_overrides_system;

        app.init_resource::<modes::DebugState>()
            .init_resource::<modes::DebugStats>()
            .init_resource::<modes::DebugVisualOverrides>()
            .configure_sets(Update, DebugPreRenderSet.after(PostPhysicsAdjustSet))
            .add_systems(Startup, debug_overlay_spawn)
            .add_systems(
                Update,
                (
                    debug_key_input_system,
                    debug_stats_collect_system,
                    apply_mode_visual_overrides_system,
                    debug_logging_system,
                    debug_overlay_update,
                )
                    .in_set(DebugPreRenderSet),
            );
    }
}

#[cfg(not(feature = "debug"))]
pub struct DebugPlugin;
#[cfg(not(feature = "debug"))]
impl bevy::prelude::Plugin for DebugPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) { }
}
