/*!
Full Demo
Aggregated current game slice with simple CLI toggles (headless + metaballs gating)
Intended to mirror `bevy_app` while allowing quick experimentation without rebuilding that binary.

CLI Flags (order-independent):
  --no-metaballs   : Force disable metaballs even if feature + config enable them
  --headless       : Run without a primary window & without RenderingPlugin/Metaballs (logic + physics only)
  --seed=N         : Override RNG seed (u64)
  --auto-exit=seconds : Override window.autoClose (approx soft exit timer; still uses config elsewhere)

Examples:
  cargo run -p full_demo --features metaballs -- --seed=42
  cargo run -p full_demo --features "metaballs debug" -- --no-metaballs
  cargo run -p full_demo -- --headless --seed=123
  cargo run -p full_demo --features metaballs -- --auto-exit=30

WASM build (visual):
  rustup target add wasm32-unknown-unknown
  cargo build -p full_demo --target wasm32-unknown-unknown --features metaballs --release
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
#[cfg(feature = "debug")]
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;

// ---------------- Config Loading (reuse logic from bevy_app) ----------------
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
    if used.is_empty() {
        info!("No config layers found; using defaults");
    } else {
        info!(?used, "Config layers loaded");
    }
    cfg
}

// ---------------- CLI Parsing (minimal / no dependency) ----------------
#[derive(Debug, Default)]
struct CliOptions {
    no_metaballs: bool,
    headless: bool,
    seed: Option<u64>,
    auto_exit: Option<f32>,
}
fn parse_cli() -> CliOptions {
    let mut opts = CliOptions::default();
    for arg in std::env::args().skip(1) {
        if arg == "--no-metaballs" {
            opts.no_metaballs = true;
        } else if arg == "--headless" {
            opts.headless = true;
        } else if let Some(rest) = arg.strip_prefix("--seed=") {
            if let Ok(v) = rest.parse::<u64>() {
                opts.seed = Some(v);
            }
        } else if let Some(rest) = arg.strip_prefix("--auto-exit=") {
            if let Ok(v) = rest.parse::<f32>() {
                opts.auto_exit = Some(v.max(0.0));
            }
        }
    }
    opts
}

// ---------------- Auto Exit System (optional) ----------------
#[derive(Resource)]
struct AutoExitTimer(Option<Timer>);

fn auto_exit_system(
    time: Res<Time>,
    mut timer: ResMut<AutoExitTimer>,
    mut exit: EventWriter<bevy::app::AppExit>,
) {
    if let Some(t) = timer.0.as_mut() {
        t.tick(time.delta());
        if t.finished() {
            info!("Auto-exit timer elapsed; exiting");
            exit.send(AppExit);
            timer.0 = None;
        }
    }
}

// ---------------- Main ----------------
fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    let cli = parse_cli();
    info!(?cli, "Parsed CLI options");

    let mut cfg = load_config();

    if let Some(ax) = cli.auto_exit {
        cfg.window.auto_close = ax;
    }
    // Metaballs gating: compile-time + CLI
    #[cfg(not(feature = "metaballs"))]
    {
        if cfg.metaballs_enabled {
            info!("Metaballs feature not compiled; disabling config.metaballs_enabled");
            cfg.metaballs_enabled = false;
        }
    }
    if cli.no_metaballs {
        cfg.metaballs_enabled = false;
        info!("Metaballs disabled via --no-metaballs");
    }

    for w in cfg.validate() {
        warn!("CONFIG WARNING: {w}");
    }

    // Headless implies: skip window + rendering + metaballs regardless of previous flags.
    let headless = cli.headless;
    if headless {
        cfg.metaballs_enabled = false;
        info!("Headless mode: rendering & metaballs disabled");
    }

    let seed = cli.seed.unwrap_or(12345);
    let title = cfg.window.title.clone();

    let mut app = App::new();
    app.insert_resource(GameConfigRes(cfg.clone()))
        .insert_resource(RngSeed(seed));

    if cfg.window.auto_close > 0.0 {
        app.insert_resource(AutoExitTimer(Some(Timer::from_seconds(
            cfg.window.auto_close,
            TimerMode::Once,
        ))));
        app.add_systems(Update, auto_exit_system);
    } else {
        app.insert_resource(AutoExitTimer(None));
    }

    // Base plugins (input, time, etc.)
    if headless {
        // Minimal set (avoid full winit / wgpu). Using MinimalPlugins plus what physics/rendering might need.
        app.add_plugins(MinimalPlugins);
    } else {
        app.add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    resolution: (cfg.window.width, cfg.window.height).into(),
                    resizable: true,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        );
    }

    // Core logic always included
    app.add_plugins(CorePlugin)
        .add_plugins(PhysicsPlugin)
        .add_plugins(GameplayPlugin);

    if !headless {
        app.add_plugins(RenderingPlugin);
    }

    // Optional feature plugins
    #[cfg(feature = "metaballs")]
    {
        if cfg.metaballs_enabled && !headless {
            app.add_plugins(MetaballsPlugin);
        }
    }
    #[cfg(feature = "debug")]
    {
        app.add_plugins(DebugToolsPlugin);
        // Rapier debug render always added; overlay / runtime mode toggle can manage visibility.
        app.add_plugins(bevy_rapier2d::prelude::RapierDebugRenderPlugin::default());
    }
    #[cfg(all(not(feature = "debug")))]
    {
        // Without debug feature still allow rapier debug if config requests (mirrors bevy_app logic).
        if cfg.rapier_debug && !headless {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }
    #[cfg(feature = "hot-reload")]
    {
        app.add_plugins(HotReloadPlugin);
    }

    info!(
        headless,
        metaballs_enabled = cfg.metaballs_enabled,
        seed,
        "Runtime configuration summary"
    );

    app.run();
