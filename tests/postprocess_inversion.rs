// Tests for metaballs inversion post-process pipeline PoC.

use bevy::prelude::*;
use ball_matcher::rendering::postprocess::{
    InversionPostProcess, MetaballsPostProcessPlugin, PostProcessToggle,
};
use ball_matcher::core::config::config::{GameConfig, MetaballsPostConfig};

#[test]
fn config_default_inversion_disabled() {
    let cfg = GameConfig::default();
    assert!(
        !cfg.metaballs_post.invert_enabled,
        "Default invert should be disabled"
    );
}

#[test]
fn marker_added_when_inversion_enabled() {
    // Build custom config enabling inversion.
    let mut cfg = GameConfig::default();
    cfg.metaballs_post = MetaballsPostConfig { invert_enabled: true };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(cfg.clone());

    // Spawn a camera BEFORE running startup so tag system can find it.
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));

    app.add_plugins(MetaballsPostProcessPlugin);

    // Run Startup schedule.
    app.update();

    // Toggle resource present & true
    {
        let toggle = app.world().resource::<PostProcessToggle>();
        assert!(toggle.invert, "Toggle resource not set to true");
    }

    // Camera should have marker
    let found = app
        .world()
        .iter_entities()
        .any(|e| e.get::<InversionPostProcess>().is_some());
    assert!(found, "InversionPostProcess marker not attached to camera");
}
