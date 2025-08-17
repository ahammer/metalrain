// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

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
mod palette; // centralized color palette

use config::GameConfig;

#[cfg(target_arch = "wasm32")]
fn load_config() -> GameConfig {
    // On wasm we cannot (easily) use synchronous std::fs; embed the file at build time.
    const RAW: &str = include_str!("../assets/config/game.ron");
    ron::from_str(RAW).expect("parse embedded game.ron")
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() -> GameConfig {
    // Layered loading: base config, optional local override.
    let (cfg, used, errors) = GameConfig::load_layered([
        std::path::Path::new("assets/config/game.ron"),
        std::path::Path::new("assets/config/game.local.ron"),
    ]);
    for e in errors {
        warn!("CONFIG LOAD ISSUE: {e}");
    }
    if used.is_empty() {
        warn!("No config files loaded; using defaults");
    } else {
        info!(?used, "Config layers loaded");
    }
    cfg
}
use game::GamePlugin;

fn main() {
    // Install better panic hook for wasm (prints to browser console)
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let cfg = load_config();

    info!(?cfg, "Loaded GameConfig");

    // Run config validation and log non-fatal warnings.
    for warn in cfg.validate() {
        warn!("CONFIG WARNING: {warn}");
    }

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
