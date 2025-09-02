use bevy::prelude::{App, MinimalPlugins};
use ball_matcher::core::config::config::GameConfig;
use ball_matcher::core::level::{LevelLoaderPlugin, LevelWalls, LevelWidgets, LevelSelection};

#[test]
fn level_loader_smoke() {
    // Ensure no conflicting env selection for first part
    std::env::remove_var("LEVEL_ID");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig::default());
    app.add_plugins(LevelLoaderPlugin);
    // First update runs Startup schedule (fallback autoload)
    app.update();

    let walls = app.world().get_resource::<LevelWalls>().expect("LevelWalls resource missing");
    assert!(!walls.0.is_empty(), "Expected some walls from basic_walls + level layout");

    let widgets = app.world().get_resource::<LevelWidgets>().expect("LevelWidgets resource missing");
    assert_eq!(widgets.spawn_points.len(), 1, "Expected exactly 1 spawn point from test_layout widgets");
    assert_eq!(widgets.attractors.len(), 1, "Expected exactly 1 attractor from test_layout widgets");

    let game_cfg = app.world().get_resource::<GameConfig>().unwrap();
    assert_eq!(game_cfg.spawn_widgets.widgets.len(), widgets.spawn_points.len(), "Config spawn widgets count mismatch");
    assert_eq!(game_cfg.gravity_widgets.widgets.len(), widgets.attractors.len(), "Config gravity widgets count mismatch");

    let sel = app.world().get_resource::<LevelSelection>().unwrap();
    assert_eq!(sel.id, "test_layout");
}

#[test]
fn level_loader_env_fallback_to_default() {
    // Request a non-existent level id; should fall back to default (test_layout)
    std::env::set_var("LEVEL_ID", "missing_level_id");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig::default());
    app.add_plugins(LevelLoaderPlugin);
    app.update();

    let sel = app.world().get_resource::<LevelSelection>().unwrap();
    assert_eq!(sel.id, "test_layout", "Expected fallback to registry default level id");
}
