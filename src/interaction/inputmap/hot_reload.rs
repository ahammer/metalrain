#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use std::{time::SystemTime, path::PathBuf};
#[cfg(feature = "debug")]
use crate::interaction::inputmap::parse::parse_input_toml;
#[cfg(feature = "debug")]
use crate::interaction::inputmap::types::InputMap;

#[cfg(feature = "debug")]
#[derive(Resource, Debug)]
struct InputReloadState { last_modified: Option<SystemTime>, timer: Timer, path: PathBuf }
#[cfg(feature = "debug")]
impl FromWorld for InputReloadState { fn from_world(_: &mut World) -> Self { Self { last_modified: None, timer: Timer::from_seconds(0.5, TimerMode::Repeating), path: PathBuf::from(std::env::var("INPUT_CONFIG_PATH").unwrap_or_else(|_| "assets/config/input.toml".into())) } } }

#[cfg(feature = "debug")]
pub struct InputMapHotReloadPlugin;
#[cfg(feature = "debug")]
impl Plugin for InputMapHotReloadPlugin { fn build(&self, app: &mut App) { #[cfg(not(target_arch = "wasm32"))] app.init_resource::<InputReloadState>().add_systems(Update, poll_input_map_reload); } }

#[cfg(feature = "debug")]
fn poll_input_map_reload(time: Res<Time>, mut state: ResMut<InputReloadState>, mut input_map: ResMut<InputMap>) {
    #[cfg(target_arch = "wasm32")] { return; }
    if !state.timer.tick(time.delta()).finished() { return; }
    use std::fs; if let Ok(meta) = fs::metadata(&state.path) { if let Ok(mod_time) = meta.modified() { let need_reload = match state.last_modified { Some(prev) => mod_time > prev, None => true }; if need_reload { state.last_modified = Some(mod_time); if let Ok(raw) = fs::read_to_string(&state.path) { let parsed = parse_input_toml(&raw, cfg!(feature="debug")); if !parsed.errors.is_empty() { for e in parsed.errors { warn!("INPUT HOT-RELOAD parse error: {e}"); } } else { *input_map = parsed.input_map; info!("Input map hot-reloaded"); } } } } } }
