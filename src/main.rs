// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

use ball_matcher::app::game::GamePlugin;
use ball_matcher::core::config::config::GameConfig;

#[cfg(target_arch = "wasm32")]
fn load_config() -> GameConfig {
    const RAW: &str = include_str!("../assets/config/game.ron");
    ron::from_str(RAW).expect("parse embedded game.ron")
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() -> GameConfig {
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

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }
    let cfg = load_config();
    info!(?cfg, "Loaded GameConfig");
    for warn in cfg.validate() {
        warn!("CONFIG WARNING: {warn}");
    }
    let mut app = App::new();
    app.insert_resource(cfg.clone())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: cfg.window.title.clone(),
                resolution: (cfg.window.width, cfg.window.height).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }));
    app.add_plugins(GamePlugin);
    #[cfg(feature = "debug")]
    {
        app.add_plugins(RapierDebugRenderPlugin::default());
    }
    #[cfg(not(feature = "debug"))]
    {
        if cfg.rapier_debug {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        use bevy::winit::WinitSettings;
        app.insert_resource(WinitSettings::game());
    }
    app.run();
}
