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
    use modes::propagate_metaballs_view_system;
        use crate::components::BallCircleVisual;

        fn toggle_circle_visibility(
            state: Res<modes::DebugState>,
            mut q: Query<&mut Visibility, With<BallCircleVisual>>,
        ) {
            use modes::DebugRenderMode::*;
            let show = matches!(state.mode, BallsFlat | BallsWithClusters | RapierWireframe);
            for mut vis in q.iter_mut() {
                vis.set_if_neq(if show { Visibility::Visible } else { Visibility::Hidden });
            }
        }

        app.init_resource::<modes::DebugState>()
        .init_resource::<modes::DebugStats>()
            .init_resource::<modes::DebugVisualOverrides>()
        .init_resource::<modes::LastAppliedMetaballsView>()
            .configure_sets(Update, DebugPreRenderSet.after(PostPhysicsAdjustSet))
            .add_systems(Startup, debug_overlay_spawn)
            .add_systems(
                Update,
                (
                    debug_key_input_system,
                    debug_stats_collect_system,
                    apply_mode_visual_overrides_system,
            propagate_metaballs_view_system,
                    toggle_circle_visibility,
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
