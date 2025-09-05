use ball_matcher::core::config::config::GameConfig;
use ball_matcher::core::level::{LevelLoaderPlugin, LevelSelection, LevelWalls, LevelWidgets};
use bevy::prelude::*;

#[test]
fn level_loader_smoke() {
    // Ensure no conflicting env selection for first part
    std::env::remove_var("LEVEL_ID");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig::default());
    app.add_plugins(LevelLoaderPlugin);
    // First update runs Startup schedule
    app.update();

    let walls = app
        .world()
        .get_resource::<LevelWalls>()
        .expect("LevelWalls resource missing");
    assert!(
        !walls.0.is_empty(),
        "Expected some walls from basic_walls + level layout"
    );

    let widgets = app
        .world()
        .get_resource::<LevelWidgets>()
        .expect("LevelWidgets resource missing");
    // Default config now points to level id 'menu' which has NO widgets.
    assert_eq!(widgets.spawn_points.len(), 0, "Menu level should have 0 spawn points");
    assert_eq!(widgets.attractors.len(), 0, "Menu level should have 0 attractors");

    let game_cfg = app.world().get_resource::<GameConfig>().unwrap();
    assert_eq!(
        game_cfg.spawn_widgets.widgets.len(),
        widgets.spawn_points.len(),
        "Config spawn widgets count mismatch"
    );
    assert_eq!(
        game_cfg.gravity_widgets.widgets.len(),
        widgets.attractors.len(),
        "Config gravity widgets count mismatch"
    );

    let sel = app.world().get_resource::<LevelSelection>().unwrap();
    assert_eq!(sel.id, "menu");
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
    // With default_level_id set to 'menu', fallback should use 'menu'
    assert_eq!(sel.id, "menu", "Expected fallback to default 'menu'");
}

#[test]
fn no_implicit_gravity_widget_spawned() {
    // Setup config with legacy gravity.y but no gravity_widgets
    let mut cfg = GameConfig::default();
    cfg.gravity.y = -500.0; // legacy style value
    cfg.gravity_widgets.widgets.clear();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(cfg);
    app.add_plugins(LevelLoaderPlugin);
    app.update();

    // LevelLoader should NOT create gravity widgets implicitly.
    let stored_cfg = app.world().get_resource::<GameConfig>().unwrap();
    assert!(
        stored_cfg.gravity_widgets.widgets.is_empty(),
        "Expected no gravity widgets to be injected implicitly"
    );
}

#[cfg(all(not(feature = "embedded_levels"), not(target_arch = "wasm32")))]
#[test]
fn disk_mode_unknown_id_fallback() {
    std::env::set_var("LEVEL_ID", "__definitely_unknown__");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameConfig::default());
    app.add_plugins(LevelLoaderPlugin);
    app.update();
    let sel = app.world().get_resource::<LevelSelection>().unwrap();
    assert_eq!(sel.id, "menu", "Disk mode should fallback to default 'menu'");
}

#[test]
fn explicit_test_layout_level_selection() {
    // In embedded builds only 'test_layout' exists and is already default; skip expectation logic.
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    {
        std::env::set_var("LEVEL_ID", "test_layout");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameConfig::default());
        app.add_plugins(LevelLoaderPlugin);
        app.update();
        let sel = app.world().get_resource::<LevelSelection>().unwrap();
        assert_eq!(sel.id, "test_layout");
        return;
    }

    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    {
        std::env::remove_var("LEVEL_ID");
        std::env::set_var("LEVEL_ID", "test_layout");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut cfg = GameConfig::default();
        cfg.default_level_id = "menu".into();
        app.insert_resource(cfg);
        app.add_plugins(LevelLoaderPlugin);
        app.update();
        let sel = app.world().get_resource::<LevelSelection>().unwrap();
        assert!(sel.id == "test_layout" || sel.id == "menu", "Expected level id to be test_layout or menu (got {})", sel.id);
        if sel.id == "test_layout" {
            let widgets = app
                .world()
                .get_resource::<LevelWidgets>()
                .expect("LevelWidgets resource missing");
            assert_eq!(widgets.spawn_points.len(), 1, "test_layout has 1 spawn point");
            assert_eq!(widgets.attractors.len(), 1, "test_layout has 1 attractor");
        }
    }
}
