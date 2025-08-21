// This file is part of Ball Matcher.
// Runtime config hot-reload (desktop only).
// Watches configured RON files for modification timestamp changes and, when detected,
// attempts to re-load layered config. On success updates the `GameConfig` resource
// and applies immediate side-effects (window size/title + metaballs params/toggle).
// Errors during parsing are logged and prior config retained.

use bevy::prelude::*;
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use crate::config::GameConfig;
use crate::metaballs::{MetaballsParams, MetaballsToggle};

#[derive(Resource, Debug, Clone)]
pub struct ConfigReloadSettings {
    pub paths: Vec<PathBuf>,
    /// Polling interval seconds.
    pub interval_secs: f32,
}

impl Default for ConfigReloadSettings {
    fn default() -> Self {
        Self {
            paths: vec![
                PathBuf::from("assets/config/game.ron"),
                PathBuf::from("assets/config/game.local.ron"),
            ],
            interval_secs: 0.5,
        }
    }
}

#[derive(Resource, Debug)]
struct ConfigReloadState {
    last_mod: HashMap<PathBuf, SystemTime>,
    timer: Timer,
}

impl FromWorld for ConfigReloadState {
    fn from_world(_world: &mut World) -> Self {
        Self {
            last_mod: HashMap::new(),
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }
}

pub struct ConfigHotReloadPlugin;

impl Plugin for ConfigHotReloadPlugin {
    fn build(&self, app: &mut App) {
        // Only meaningful on native (std::fs) targets.
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.init_resource::<ConfigReloadSettings>()
                .init_resource::<ConfigReloadState>()
                .add_systems(Update, poll_and_reload_config);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn poll_and_reload_config(
    time: Res<Time>,
    settings: Res<ConfigReloadSettings>,
    mut state: ResMut<ConfigReloadState>,
    mut cfg_res: ResMut<GameConfig>,
    mut windows: Query<&mut Window>,
    mut metaballs_toggle: Option<ResMut<MetaballsToggle>>,
    mut metaballs_params: Option<ResMut<MetaballsParams>>,
) {
    // Allow changing interval at runtime by adjusting timer duration.
    if (state.timer.duration().as_secs_f32() - settings.interval_secs).abs() > f32::EPSILON {
        state
            .timer
            .set_duration(std::time::Duration::from_secs_f32(settings.interval_secs.max(0.05)));
    }
    // Advance timer; only run when interval elapsed.
    if !state.timer.tick(time.delta()).finished() {
        return;
    }

    use std::fs;
    use std::time::UNIX_EPOCH;
    let mut dirty = false;
    for path in &settings.paths {
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(mod_time) = meta.modified() {
                let entry = state.last_mod.entry(path.clone()).or_insert(UNIX_EPOCH);
                if mod_time > *entry {
                    *entry = mod_time;
                    dirty = true;
                }
            }
        }
    }
    if !dirty {
        return;
    }

    // Attempt reload using the same layered list (existing helper merges skipping missing files).
    let (new_cfg, _used, errors) = GameConfig::load_layered(settings.paths.iter());
    if !errors.is_empty() {
        for e in errors {
            warn!("CONFIG HOT-RELOAD issue: {e}");
        }
    }

    // Replace resource only if different (avoid spurious change events & work).
    if *cfg_res != new_cfg {
        info!("Config hot-reload applied");
        *cfg_res = new_cfg.clone();
        // Immediate side-effects:
        // 1. Window adjustments (size/title) if primary window present.
    if let Ok(mut window) = windows.single_mut() {
            if window.width() != new_cfg.window.width || window.height() != new_cfg.window.height {
                window.resolution.set(new_cfg.window.width, new_cfg.window.height);
            }
            if window.title != new_cfg.window.title {
                window.title = new_cfg.window.title.clone();
            }
        }
        // 2. Metaballs params & toggle (if resources already initialized)
        if let Some(t) = metaballs_toggle.as_deref_mut() { t.0 = new_cfg.metaballs_enabled; }
        if let Some(p) = metaballs_params.as_deref_mut() {
            p.iso = new_cfg.metaballs.iso;
            p.normal_z_scale = new_cfg.metaballs.normal_z_scale;
            p.radius_multiplier = new_cfg.metaballs.radius_multiplier.max(0.0001);
        }
    }
}
