// Phase 9 (scaffold): Hot reload crate placeholder.
// Native-only file watching (later) behind feature "native-watch" and cfg(not(target_arch = "wasm32")).
// For now provides HotReloadPlugin that does nothing so wiring compiles.

use bevy::prelude::*;
use bm_config::GameConfig;

pub struct HotReloadPlugin;

impl Plugin for HotReloadPlugin {
    fn build(&self, _app: &mut App) {
        // Future:
        // - Insert a watcher resource (native only)
        // - Poll for file changes, reload config RON, validate, apply as resource
        // - Emit events for subsystems to react (e.g., metaballs params change)
    }
}

// Placeholder API for future update application
pub fn apply_config_update(_cfg: &GameConfig) {
    // Intentionally empty
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_adds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(HotReloadPlugin);
    }
}
