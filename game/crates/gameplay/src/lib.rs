// Phase 4 (scaffold): Gameplay crate placeholder.
// Will host spawning, emitter, interactions, clustering logic later.
// Currently only exposes an empty GameplayPlugin so other crates can depend on the symbol.

use bevy::prelude::*;
mod spawning;
mod emitter;
mod interactions;
mod cluster;
// Re-export cluster resources for feature crates (e.g., metaballs) without exposing internal systems.
pub use cluster::Clusters;
pub use spawning::{spawn_initial_ring, START_RING_COUNT};

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        // Spawning scaffold
        app.add_systems(Startup, spawn_initial_ring);
        // Emitter scaffold (runtime spawning)
        app.add_systems(Startup, emitter::emitter_init);
        app.add_systems(Update, emitter::emitter_spawn);
        // Interactions (drag + explosion)
        interactions::add_interaction_systems(app);
        // Clustering
        cluster::add_cluster_systems(app);
        // Future: additional clustering analytics / snapshot export.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_adds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameplayPlugin);
    }

    #[test]
    fn plugin_spawns_ring() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Core plugin provides system sets used by interactions.
        app.add_plugins(bm_core::CorePlugin);
        // Minimal config resource so interaction systems can read interaction settings.
        // Set balls.count to START_RING_COUNT so expected value matches assertion.
        let mut cfg = bm_config::GameConfig::default();
        cfg.balls.count = START_RING_COUNT;
        app.insert_resource(bm_core::GameConfigRes(cfg));
        // Input resources & minimal window/camera so interaction systems don't panic on missing resources.
        app.insert_resource(ButtonInput::<MouseButton>::default());
        app.insert_resource(Touches::default());
        app.world_mut().spawn(Window::default());
        app.world_mut().spawn((Camera::default(), GlobalTransform::default()));
        app.add_plugins(GameplayPlugin);
        app.update(); // run Startup
        let world = app.world_mut();
        let mut q = world.query::<&bm_core::Ball>();
        assert_eq!(q.iter(world).count(), START_RING_COUNT, "ring spawn count mismatch");
    }

    #[test]
    fn plugin_inserts_active_drag() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bm_core::CorePlugin);
        app.insert_resource(bm_core::GameConfigRes(Default::default()));
        app.add_plugins(GameplayPlugin);
        assert!(app.world().get_resource::<crate::interactions::ActiveDrag>().is_some(), "ActiveDrag missing");
    }

    #[test]
    fn plugin_adds_clusters_resource() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bm_core::CorePlugin);
        app.insert_resource(bm_core::GameConfigRes(Default::default()));
        app.add_plugins(GameplayPlugin);
        assert!(app.world().get_resource::<crate::cluster::Clusters>().is_some(), "Clusters resource missing");
    }
}
