/*!
Metaballs Demo
Focused visualization of the metaballs pipeline + parameter tweak keys.

Use:
  cargo run -p metaballs_demo --features metaballs
  cargo run -p metaballs_demo --features "metaballs debug"

If run without the `metaballs` feature the demo logs a warning and exits cleanly.

Keys (when enabled):
  [ / ] : iso -/+
  K / L : normal_z_scale -/+
  , / . : radius_multiplier -/+
  R     : reset params
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
    #[cfg(not(feature = "metaballs"))]
    {
        println!("metaballs_demo: built without `metaballs` feature; exiting");
        return;
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let mut cfg = load_config();
    // Ensure metaballs enabled for this focused demo
    cfg.metaballs_enabled = true;
    // Keep emitter disabled for controlled view (optional)
    cfg.emitter.enabled = false;

    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }

    let title = format!("Metaballs Demo - {}", cfg.window.title);

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(2025))
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
        .add_plugins(GameplayPlugin) // supplies clusters & basic spawning (ring)
        .add_plugins(MetaballsPlugin);

    #[cfg(feature = "debug")]
    {
        if cfg.rapier_debug {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }

    app.run();
}
