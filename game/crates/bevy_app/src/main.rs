/*!
Phase 7+: bevy_app main with feature‑gated optional subsystems and Milestone A fast-path improvements.

Adds:
* Config loading (native layered + wasm embed) using bm_config::GameConfig.
* Validation warnings logging.
* Rapier debug render plugin gating (config + feature).
* Feature‑conditional enabling/disabling of metaballs flag in config (forces false if feature absent).
*/

use bevy::prelude::*;
use bm_core::{CorePlugin, GameConfigRes, RngSeed};
use bm_physics::PhysicsPlugin;
use bm_rendering::RenderingPlugin;
use bm_gameplay::GameplayPlugin;

#[cfg(feature = "metaballs")]
use bm_metaballs::MetaballsPlugin;

#[cfg(feature = "debug")]
use bm_debug_tools::DebugToolsPlugin;

#[cfg(feature = "hot-reload")]
use bm_hot_reload::HotReloadPlugin;

#[cfg(any(feature = "debug", not(feature = "debug")))]
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

// ---------------- Config Loading ----------------

#[cfg(target_arch = "wasm32")]
fn load_config() -> bm_config::GameConfig {
    // Embed base config (no layered local override on wasm for now).
    const RAW: &str = include_str!("../../../assets/config/game.ron");
    ron::from_str(RAW).unwrap_or_else(|e| {
        warn!("CONFIG (wasm) parse failure: {e}; using defaults");
        bm_config::GameConfig::default()
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn load_config() -> bm_config::GameConfig {
    let (cfg, used, errors) = bm_config::GameConfig::load_layered([
        std::path::Path::new("assets/config/game.ron"),
        std::path::Path::new("assets/config/game.local.ron"),
    ]);
    for e in errors {
        warn!("CONFIG LOAD ISSUE: {e}");
    }
    if used.is_empty() {
        info!("No config layers found; using defaults");
    } else {
        info!(?used, "Config layers loaded");
    }
    cfg
}

// ---------------- Main ----------------

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        // Better panic messages on wasm
        console_error_panic_hook::set_once();
    }

    let mut cfg = load_config();

    // Force metaballs flag false if feature not compiled; if compiled leave user value.
    #[cfg(not(feature = "metaballs"))]
    {
        if cfg.metaballs_enabled {
            info!("Metaballs feature disabled at compile time; overriding config.metaballs_enabled=false");
            cfg.metaballs_enabled = false;
        }
    }

    // Log validation warnings (non-fatal)
    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }
    info!(?cfg.window, "Window config");
    info!(
        balls = cfg.balls.count,
        emitter_enabled = cfg.emitter.enabled,
        metaballs = cfg.metaballs_enabled,
        "Runtime feature summary"
    );

    let window_title = cfg.window.title.clone();

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(12345))
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: window_title,
                    resolution: (cfg.window.width, cfg.window.height).into(),
                    resizable: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .add_plugins(CorePlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(RenderingPlugin)
        .add_plugins(GameplayPlugin);

    // Optional feature plugins
    #[cfg(feature = "metaballs")]
    {
        app.add_plugins(MetaballsPlugin);
    }
    #[cfg(feature = "debug")]
    {
        app.add_plugins(DebugToolsPlugin);
    }
    #[cfg(feature = "hot-reload")]
    {
        app.add_plugins(HotReloadPlugin);
    }

    // Rapier debug render gating:
    // * If debug feature compiled: always add (matches legacy convenience) OR config flag.
    // * If debug feature absent: only add when config.rapier_debug true (lightweight visual aid).
    #[cfg(feature = "debug")]
    {
        // Always add; runtime modes / overlay can toggle visibility.
        app.add_plugins(RapierDebugRenderPlugin::default());
    }
    #[cfg(not(feature = "debug"))]
    {
        if cfg.rapier_debug {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }

    app.run();
}
