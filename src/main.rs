// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

use ball_matcher::app::game::GamePlugin;
use ball_matcher::app::state::{AppState, GameplayState};
use ball_matcher::app::menu::MenuPlugin;
use ball_matcher::core::config::config::GameConfig;
use ball_matcher::core::level::LevelLoaderPlugin;

mod webgpu_guard;

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

// Assert the selected adapter backend matches policy:
//  - wasm32: BrowserWebGpu only
//  - native: Vulkan / Metal / DX12 only
fn assert_backend(info: Res<RenderAdapterInfo>) {
    // Avoid needing variant helper methods (not exposed); compare Debug string.
    let backend_str = format!("{:?}", info.backend);
    #[cfg(target_arch = "wasm32")]
    {
        assert!(
            backend_str == "BrowserWebGpu",
            "Expected BrowserWebGpu backend; got {backend_str}. GL/WebGL not compiled."
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(
            matches!(backend_str.as_str(), "Vulkan" | "Metal" | "Dx12"),
            "Only Vulkan / Metal / DX12 allowed (got {backend_str})"
        );
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        // Early environment guard (panics if navigator.gpu missing)
        webgpu_guard::assert_webgpu_available();
    }

    let cfg = load_config();
    info!(?cfg, "Loaded GameConfig");
    for warn in cfg.validate() {
        warn!("CONFIG WARNING: {warn}");
    }

    let mut app = App::new();

    app.insert_resource(cfg.clone());

    // Window + explicit RenderPlugin with strict backend masks.
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: cfg.window.title.clone(),
                    resolution: (cfg.window.width, cfg.window.height).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::VULKAN | Backends::METAL | Backends::DX12),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: cfg.window.title.clone(),
                    resolution: (cfg.window.width, cfg.window.height).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::BROWSER_WEBGPU),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    );

    // Register high-level states early so downstream plugins can add state-based systems.
    app.insert_state(AppState::MainMenu);
    app.insert_state(GameplayState::Playing);

    // Menu first (reads registry), then level loader (reacts to state transitions), then gameplay.
    app.add_plugins((MenuPlugin, LevelLoaderPlugin, GamePlugin));

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

    // Backend assertion system (runs at startup)
    app.add_systems(Startup, assert_backend);

    app.run();
}
