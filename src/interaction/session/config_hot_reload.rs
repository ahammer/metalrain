use bevy::prelude::*;
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use crate::core::config::GameConfig;
use crate::rendering::metaballs::metaballs::{MetaballsParams, MetaballsToggle};

#[derive(Resource, Debug, Clone)]
pub struct ConfigReloadSettings { pub paths: Vec<PathBuf>, pub interval_secs: f32 }
impl Default for ConfigReloadSettings { fn default() -> Self { Self { paths: vec![ PathBuf::from("assets/config/game.ron"), PathBuf::from("assets/config/game.local.ron"), ], interval_secs: 0.5 } } }
#[derive(Resource, Debug)]
struct ConfigReloadState { last_mod: HashMap<PathBuf, SystemTime>, timer: Timer }
impl FromWorld for ConfigReloadState { fn from_world(_world: &mut World) -> Self { Self { last_mod: HashMap::new(), timer: Timer::from_seconds(0.5, TimerMode::Repeating) } } }

pub struct ConfigHotReloadPlugin;
impl Plugin for ConfigHotReloadPlugin { fn build(&self, app: &mut App) { #[cfg(not(target_arch = "wasm32"))] { app.init_resource::<ConfigReloadSettings>().init_resource::<ConfigReloadState>().add_systems(Update, poll_and_reload_config); } } }

fn poll_and_reload_config(
    time: Res<Time>,
    settings: Res<ConfigReloadSettings>,
    mut state: ResMut<ConfigReloadState>,
    mut cfg_res: ResMut<GameConfig>,
    mut windows: Query<&mut Window>,
    mut metaballs_toggle: Option<ResMut<MetaballsToggle>>,
    mut metaballs_params: Option<ResMut<MetaballsParams>>,
) {
    if (state.timer.duration().as_secs_f32() - settings.interval_secs).abs() > f32::EPSILON { state.timer.set_duration(std::time::Duration::from_secs_f32(settings.interval_secs.max(0.05))); }
    if !state.timer.tick(time.delta()).finished() { return; }
    use std::fs; use std::time::UNIX_EPOCH; let mut dirty = false; for path in &settings.paths { if let Ok(meta) = fs::metadata(path) { if let Ok(mod_time) = meta.modified() { let entry = state.last_mod.entry(path.clone()).or_insert(UNIX_EPOCH); if mod_time > *entry { *entry = mod_time; dirty = true; } } } }
    if !dirty { return; }
    let (new_cfg, _used, errors) = GameConfig::load_layered(settings.paths.iter()); if !errors.is_empty() { for e in errors { warn!("CONFIG HOT-RELOAD issue: {e}"); } }
    if *cfg_res != new_cfg { info!("Config hot-reload applied"); *cfg_res = new_cfg.clone(); if let Ok(mut window) = windows.single_mut() { if window.width() != new_cfg.window.width || window.height() != new_cfg.window.height { window.resolution.set(new_cfg.window.width, new_cfg.window.height); } if window.title != new_cfg.window.title { window.title = new_cfg.window.title.clone(); } } if let Some(t) = metaballs_toggle.as_deref_mut() { t.0 = new_cfg.metaballs_enabled; } if let Some(p) = metaballs_params.as_deref_mut() { p.iso = new_cfg.metaballs.iso; p.normal_z_scale = new_cfg.metaballs.normal_z_scale; p.radius_multiplier = new_cfg.metaballs.radius_multiplier.max(0.0001); } }
}
