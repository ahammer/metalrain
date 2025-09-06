#[cfg(feature = "debug")]
use super::modes::{DebugRenderMode, DebugState};
#[cfg(feature = "debug")]
use crate::interaction::inputmap::types::InputMap;
#[cfg(feature = "debug")]
use bevy::prelude::*;

#[cfg(feature = "debug")]
pub fn debug_key_input_system(input_map: Option<Res<InputMap>>, mut state: ResMut<DebugState>) {
    let Some(input_map) = input_map else {
        return;
    };
    let mut new_mode = None;
    if input_map.just_pressed("DebugMode1") {
        new_mode = Some(DebugRenderMode::Metaballs);
    }
    // Other debug modes removed; only Metaballs remains.
    if let Some(m) = new_mode {
        if m != state.mode {
            state.last_mode = state.mode;
            state.mode = m;
            info!(
                "MODE_CHANGE from={:?} to={:?} frame={} ",
                state.last_mode, state.mode, state.frame_counter
            );
        }
    }
    if input_map.just_pressed("ToggleOverlay") {
        state.overlay_visible = !state.overlay_visible;
    }
}
