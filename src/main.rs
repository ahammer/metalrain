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

#[cfg(target_arch = "wasm32")]
fn load_config() -> GameConfig {
    // On wasm we cannot (easily) use synchronous std::fs; embed the file at build time.
    const RAW: &str = include_str!("../assets/config/game.ron");
    ron::from_str(RAW).expect("parse embedded game.ron")
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() -> GameConfig {
    GameConfig::load_from_file("assets/config/game.ron")
        .expect("Failed to load assets/config/game.ron")
}
use game::GamePlugin;

fn main() {
    // Install better panic hook for wasm (prints to browser console)
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let cfg = load_config();

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

    #[cfg(target_arch = "wasm32")]
    {
        // Ensure the canvas is created with the id expected by simple index.html (optional customization)
        use bevy::winit::WinitSettings;
        app.insert_resource(WinitSettings::game());
    }

    app.run();
}
