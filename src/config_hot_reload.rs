use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::GameConfig;

/// Resource storing the path to the game config file (RON) to watch.
#[derive(Resource, Clone)]
pub struct GameConfigPath(pub String);

/// Event fired after a config file reload has been applied. Carries the new config.
#[derive(Event, Debug, Clone)]
pub struct GameConfigReloaded(pub GameConfig);

/// Internal resource tracking last modification timestamp so we can poll cheaply.
#[derive(Resource, Default)]
struct ConfigFileMeta {
    last_modified_unix: u64,
}

/// Plugin enabling polling-based hot reload of the external RON config file.
/// Simpler than integrating with AssetServer; adequate for a single small file.
pub struct ConfigHotReloadPlugin;

impl Plugin for ConfigHotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConfigFileMeta>()
            .add_event::<GameConfigReloaded>()
            .add_systems(Update, (poll_config_file, apply_reloaded_config));
    }
}

/// System: poll the config file periodically (every N seconds) and if its
/// modification timestamp changes, attempt to reload and overwrite the GameConfig resource.
fn poll_config_file(
    path: Option<Res<GameConfigPath>>,
    mut meta: ResMut<ConfigFileMeta>,
    mut cfg_res: ResMut<GameConfig>,
    mut ev_writer: EventWriter<GameConfigReloaded>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    // Poll at most 4 times per second.
    *timer += time.delta_seconds();
    if *timer < 0.25 { return; }
    *timer = 0.0;

    let Some(path) = path else { return; };
    // Get metadata
    let Ok(metadata) = fs::metadata(&path.0) else { return; };
    let modified = metadata.modified().ok();
    let mod_unix = modified.and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    if mod_unix == 0 || mod_unix == meta.last_modified_unix { return; }

    // Attempt to read and parse new config.
    match fs::read_to_string(&path.0) {
        Ok(data) => match ron::from_str::<GameConfig>(&data) {
            Ok(new_cfg) => {
                if *cfg_res != new_cfg { // only update if something actually changed
                    info!("Config hot-reload applied from {}", path.0);
                    *cfg_res = new_cfg.clone();
                    meta.last_modified_unix = mod_unix;
                    ev_writer.send(GameConfigReloaded(new_cfg));
                } else {
                    // Even if timestamp changed but content equal, just update meta.
                    meta.last_modified_unix = mod_unix;
                }
            }
            Err(e) => warn!("Failed to parse updated config {}: {e}", path.0),
        },
        Err(e) => warn!("Failed to read config file {}: {e}", path.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sample_config(gravity_y: f32, restitution: f32, title: &str) -> String {
        format!("(window:(width:100.0,height:100.0,title:\"{title}\"),gravity:(y:{gravity_y}),bounce:(restitution:{restitution}),balls:(count:0,radius_range:(min:1.0,max:2.0),x_range:(min:-1.0,max:1.0),y_range:(min:-1.0,max:1.0),vel_x_range:(min:0.0,max:0.0),vel_y_range:(min:0.0,max:0.0)),separation:(enabled:false,overlap_slop:0.98,push_strength:0.5,max_push:10.0,velocity_dampen:0.0),)" )
    }

    #[test]
    fn reloads_on_file_change() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let initial = sample_config(-10.0, 0.3, "T1");
        tmp.write_all(initial.as_bytes()).unwrap();
        tmp.as_file().sync_all().unwrap();

        let path_str = tmp.path().to_string_lossy().to_string();
        let cfg = GameConfig::load_from_file(&path_str).unwrap();

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(cfg)
            .insert_resource(GameConfigPath(path_str.clone()))
            .add_event::<GameConfigReloaded>()
            .init_resource::<ConfigFileMeta>()
            .add_systems(Update, poll_config_file);

        // First update seeds metadata but no change
        app.update();

        // Modify file with different gravity and restitution
        std::thread::sleep(std::time::Duration::from_millis(1100)); // ensure mod time change on some filesystems
    let updated = sample_config(-20.0, 0.9, "T2");
    std::fs::write(tmp.path(), updated).unwrap();

        // Advance time resource to trigger polling (>=0.25s)
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.3));
        }
        app.update();

        let new_cfg = app.world().resource::<GameConfig>();
        assert_eq!(new_cfg.gravity.y, -20.0);
        assert_eq!(new_cfg.bounce.restitution, 0.9);
        assert_eq!(new_cfg.window.title, "T2");
    }
}

/// System: react to config reload events and update runtime state that is not
/// automatically picked up via systems reading the GameConfig resource.
fn apply_reloaded_config(
    mut ev_reader: EventReader<GameConfigReloaded>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>, // update title/resolution
    mut rapier_cfg: Option<ResMut<RapierConfiguration>>,   // update gravity
    mut q_restitution: Query<&mut Restitution>,           // update restitution on existing colliders
) {
    for ev in ev_reader.read() {
        // Update window properties
        if let Ok(mut win) = windows.get_single_mut() {
            win.title = ev.0.window.title.clone();
            // NOTE: Changing resolution here will resize; physics walls handle resize events.
            win.resolution.set(ev.0.window.width, ev.0.window.height);
        }
        // Update rapier gravity if plugin active
    if let Some(rapier) = rapier_cfg.as_mut() {
            rapier.gravity = Vect::new(0.0, ev.0.gravity.y);
        }
        // Update restitution of existing physics materials where we used a uniform coefficient.
        // For now we just set any Restitution component's coefficient to bounce.restitution if it differs.
        for mut rest in &mut q_restitution {
            if (rest.coefficient - ev.0.bounce.restitution).abs() > f32::EPSILON {
                *rest = Restitution::coefficient(ev.0.bounce.restitution);
            }
        }
    }
}
