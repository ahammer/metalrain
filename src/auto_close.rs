// Automatically exits the app after a configured number of seconds (if > 0)
// Reads `GameConfig.window.autoClose` (RON key) / `WindowConfig::auto_close`.
// 0.0 (default) => disabled.

use bevy::prelude::*;

use crate::config::GameConfig;

#[derive(Resource, Deref, DerefMut)]
struct AutoCloseTimer(Timer);

pub struct AutoClosePlugin;

impl Plugin for AutoClosePlugin {
    fn build(&self, app: &mut App) {
        // At startup, examine config and if auto_close > 0 create a timer resource + system.
        app.add_systems(Startup, setup_autoclose)
            .add_systems(Update, check_autoclose);
    }
}

fn setup_autoclose(mut commands: Commands, cfg: Res<GameConfig>) {
    let secs = cfg.window.auto_close;
    if secs > 0.0 {
        info!(seconds = secs, "AutoClose: will exit after {secs} seconds");
        commands.insert_resource(AutoCloseTimer(Timer::from_seconds(secs, TimerMode::Once)));
    }
}

fn check_autoclose(
    time: Res<Time>,
    mut timer: Option<ResMut<AutoCloseTimer>>,
    mut ev_exit: EventWriter<AppExit>,
) {
    if let Some(t) = timer.as_mut() {
        t.tick(time.delta());
        if t.finished() {
            info!("AutoClose: timer finished, requesting app exit");
            ev_exit.write(AppExit::Success);
        }
    }
}
