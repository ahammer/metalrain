// Phase 0 scaffold binary:
// Minimal Bevy app wiring core plugins + our placeholder feature crates.
// Real wiring order & config will be refined in later phases.

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

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    // Core configuration & deterministic seed resources
    app.insert_resource(GameConfigRes::default());
    app.insert_resource(RngSeed(0));

    // Base layer plugins
    app.add_plugins(CorePlugin);
    app.add_plugins(PhysicsPlugin);
    app.add_plugins(RenderingPlugin);
    app.add_plugins(GameplayPlugin);

    // Optional feature plugins (order may adjust later)
    #[cfg(feature = "metaballs")]
    app.add_plugins(MetaballsPlugin);

    #[cfg(feature = "debug")]
    app.add_plugins(DebugToolsPlugin);

    #[cfg(feature = "hot-reload")]
    app.add_plugins(HotReloadPlugin);

    // Temporary: window configuration skipped (API changed in Bevy 0.16; will configure later).
    // Phase 2 note: physics baseline (radial gravity + separation) active.

    app.run();
}
