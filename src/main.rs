use bevy::prelude::*;

mod config;
mod components;
mod physics;
mod spawn;
mod camera;
mod game;

use config::GameConfig;
use game::GamePlugin;

fn main() {
    // Load configuration (fall back to defaults if missing)
    let cfg = GameConfig::load_from_file("assets/config/game.ron")
        .expect("Failed to load assets/config/game.ron");

    App::new()
        .insert_resource(cfg.clone())
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
        )
        .insert_resource(cfg) // keep owned copy after clone inserted earlier if needed elsewhere
        .add_plugins(GamePlugin)
        .run();
}
