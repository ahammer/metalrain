#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use super::modes::{DebugRenderMode, DebugState};

#[cfg(feature = "debug")]
pub fn debug_key_input_system(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<DebugState>) {
    let mut new_mode = None;
    if keys.just_pressed(KeyCode::Digit1) { new_mode = Some(DebugRenderMode::Metaballs); }
    if keys.just_pressed(KeyCode::Digit2) { new_mode = Some(DebugRenderMode::BallsFlat); }
    if keys.just_pressed(KeyCode::Digit3) { new_mode = Some(DebugRenderMode::BallsWithClusters); }
    if keys.just_pressed(KeyCode::Digit4) { new_mode = Some(DebugRenderMode::RapierWireframe); }
    if keys.just_pressed(KeyCode::Digit5) { new_mode = Some(DebugRenderMode::MetaballHeightfield); }
    if keys.just_pressed(KeyCode::Digit6) { new_mode = Some(DebugRenderMode::MetaballColorInfo); }
    if let Some(m) = new_mode { if m != state.mode { state.last_mode = state.mode; state.mode = m; info!("MODE_CHANGE from={:?} to={:?} frame={} ", state.last_mode, state.mode, state.frame_counter); } }
    if keys.just_pressed(KeyCode::F1) { state.overlay_visible = !state.overlay_visible; }
}
