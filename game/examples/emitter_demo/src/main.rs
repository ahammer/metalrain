/*!
Emitter Demo
Visual slice extending spawn_demo by enabling the runtime emitter to show continuous spawning.

Use:
  cargo run -p emitter_demo
  cargo run -p emitter_demo --features metaballs
  cargo run -p emitter_demo --features "metaballs debug"

Behavior:
* Loads layered config (native) or embedded config (wasm).
* Forces emitter.enabled = true (even if config disables) to showcase spawning.
* Respects config.emitter.rate_per_sec, max_live, burst (future: burst not yet wired).
* Metaballs visualization appears only if both compiled feature and config.metaballs_enabled are true (forced false when feature absent).

WASM build:
  rustup target add wasm32-unknown-unknown
  cargo build -p emitter_demo --target wasm32-unknown-unknown --release
*/

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bm_core::{CorePlugin, GameConfigRes, RngSeed};
use bm_physics::PhysicsPlugin;
use bm_rendering::RenderingPlugin;
use bm_gameplay::GameplayPlugin;

#[cfg(feature = "metaballs")]
use bm_metaballs::MetaballsPlugin;

#[cfg(feature = "debug")]
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

// ---------------- Config Loading (mirrors bevy_app simplified) ----------------

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

    let mut cfg = load_config();
    // Force emitter on for the demo regardless of file value.
    cfg.emitter.enabled = true;
    if cfg.emitter.max_live < cfg.balls.count {
        // Ensure there's headroom for new spawns; bump max_live if needed.
        cfg.emitter.max_live = (cfg.balls.count as f32 * 1.2).ceil() as usize;
    }
    // If metaballs feature absent, disable flag to avoid downstream expectations.
    #[cfg(not(feature = "metaballs"))]
    {
        cfg.metaballs_enabled = false;
    }

    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }

    info!(
        rate_per_sec = cfg.emitter.rate_per_sec,
        max_live = cfg.emitter.max_live,
        "Emitter config (forced enabled)"
    );

    let title = format!("Emitter Demo - {}", cfg.window.title);

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(777)) // distinct seed from spawn_demo for variety
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Run directory becomes the example crate dir; ascend to workspace root assets.
                    file_path: "../../../assets".into(),
                    ..Default::default()
                })
                .set(WindowPlugin {
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
