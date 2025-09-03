use bevy::prelude::*;
use ball_matcher::core::config::config::GameConfig;
use ball_matcher::core::level::{LevelLoaderPlugin, LevelWalls, LevelWidgets, LevelSelection};

// This test only runs when the embedded_levels feature is active (or wasm target). It ensures
// the loader succeeds and selects the expected default id.
#[cfg(any(feature = "embedded_levels", target_arch = "wasm32"))]
#[test]
fn embedded_mode_smoke() {
    std::env::remove_var("LEVEL_ID");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig::default());
    app.add_plugins(LevelLoaderPlugin);
    app.update();

    let sel = app.world().get_resource::<LevelSelection>().expect("LevelSelection missing");
    assert_eq!(sel.id, "test_layout");

    let walls = app.world().get_resource::<LevelWalls>().expect("LevelWalls missing");
    assert!(!walls.0.is_empty(), "Expected universal walls present");

    let widgets = app.world().get_resource::<LevelWidgets>().expect("LevelWidgets missing");
    assert_eq!(widgets.spawn_points.len(), 1);
    assert_eq!(widgets.attractors.len(), 1);
}

// When embedded feature is NOT active, compile a no-op to avoid unused warnings.
#[cfg(all(not(feature = "embedded_levels"), not(target_arch = "wasm32")))]
#[test]
fn embedded_mode_smoke_noop() { /* feature not active; nothing to test */ }
