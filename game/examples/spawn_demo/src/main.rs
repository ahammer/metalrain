/*!
Spawn Demo
Minimal visual slice: ring spawn + physics + simple rendering (circles/metaballs gated by config + compile features).
No runtime emitter or interactions; focuses on initial state verification.

Use:
  cargo run -p spawn_demo
  cargo run -p spawn_demo --features metaballs   (if workspace crates expose the feature transitively)

WASM build (after adding target):
  rustup target add wasm32-unknown-unknown
  cargo build -p spawn_demo --target wasm32-unknown-unknown --release
*/

use bevy::prelude::*;
use bm_core::{CorePlugin, GameConfigRes, RngSeed};
use bm_physics::PhysicsPlugin;
use bm_rendering::RenderingPlugin;
use bm_gameplay::GameplayPlugin;

#[cfg(feature = "metaballs")]
use bm_metaballs::MetaballsPlugin;

#[cfg(feature = "debug")]
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

// Config loading (mirror bevy_app; simplified)
#[cfg(target_arch = "wasm32")]
fn load_config() -> bm_config::GameConfig {
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
    if !used.is_empty() {
        info!(?used, "Config layers loaded");
    }
    cfg
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    // Load & tweak config for spawn demo: ensure emitter disabled (ring only).
    let mut cfg = load_config();
    cfg.emitter.enabled = false; // force ring-only scenario regardless of file value
    // If metaballs feature not compiled, ensure flag false (avoids systems expecting resources).
    #[cfg(not(feature = "metaballs"))]
    {
        cfg.metaballs_enabled = false;
    }

    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }

    let title = format!("Spawn Demo - {}", cfg.window.title);

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(12345))
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
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

    #[cfg(feature = "metaballs")]
    {
        app.add_plugins(MetaballsPlugin);
    }

    #[cfg(feature = "debug")]
    {
        if cfg.rapier_debug {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }

    app.run();
}
