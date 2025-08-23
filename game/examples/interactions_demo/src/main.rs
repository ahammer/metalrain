/*!
Interactions Demo
Visual slice showcasing pointer/touch interactions (drag + tap explosion) in addition to ring spawn
and (optionally) emitter + metaballs.

Use:
  cargo run -p interactions_demo
  cargo run -p interactions_demo --features metaballs
  cargo run -p interactions_demo --features "metaballs debug"

Behavior:
* Loads layered config (native) or embedded config (wasm).
* Leaves emitter.enabled as configured (so you can observe combined effects); set EMITTER=0 env to force off.
* Drag: hold left mouse (or first touch) on a ball (within grab radius) and move.
* Tap Explosion: quick click/tap (without significant movement) applies radial impulse.

WASM build:
  rustup target add wasm32-unknown-unknown
  cargo build -p interactions_demo --target wasm32-unknown-unknown --release
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

    // Optional env override to disable emitter for focused interaction testing.
    if std::env::var("EMITTER").map(|v| v == "0" || v.eq_ignore_ascii_case("false")).unwrap_or(false) {
        cfg.emitter.enabled = false;
        info!("Emitter disabled via EMITTER env override");
    }

    #[cfg(not(feature = "metaballs"))]
    {
        cfg.metaballs_enabled = false;
    }

    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }

    info!(
        drag_enabled = cfg.interactions.drag.enabled,
        explosion_enabled = cfg.interactions.explosion.enabled,
        emitter_enabled = cfg.emitter.enabled,
        metaballs = cfg.metaballs_enabled,
        "Interaction demo feature summary"
    );

    let title = format!("Interactions Demo - {}", cfg.window.title);

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(4242))
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
