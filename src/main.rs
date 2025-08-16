use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

mod config;
mod components;
mod rapier_physics;
mod spawn;
mod camera;
mod game;
mod emitter;
mod separation;
mod system_order;
mod materials;
mod cluster; // clustering of touching same-color balls
mod metaballs; // shader-based cluster metaball visualization
mod radial_gravity; // custom radial gravity force
mod input_interaction; // tap explosion & drag interactions

use config::GameConfig;
use game::GamePlugin;

fn main() {
    // Load configuration (fall back to defaults if missing)
    let cfg = GameConfig::load_from_file("assets/config/game.ron")
        .expect("Failed to load assets/config/game.ron");

    let mut app = App::new();
    app.insert_resource(cfg.clone())
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: cfg.window.title.clone(),
                    resolution: (cfg.window.width, cfg.window.height).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            }),
        );

    // Add core game plugins
    app.add_plugins(GamePlugin);

    // Conditionally add rapier debug render if enabled
    if cfg.rapier_debug {
        app.add_plugins(RapierDebugRenderPlugin::default());
    }

    app.run();
}
