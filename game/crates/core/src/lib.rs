// Phase 1 (partial): Core crate scaffold
// Purpose: foundational ECS components, markers, system set labels (placeholder).
// No game logic yet; enables other crates to compile against stable names.

use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct Ball;

/// Logical radius used both for collider and (later) rendering scale.
#[derive(Component, Debug, Deref, DerefMut, Copy, Clone)]
pub struct BallRadius(pub f32);

/// Tag component for the circle mesh child used in flat rendering modes.
#[derive(Component, Debug)]
pub struct BallCircleVisual;

/// Deterministic RNG seed resource (set once at startup / tests for reproducible spawning & logic).
#[derive(Resource, Debug, Copy, Clone)]
pub struct RngSeed(pub u64);

// Wrapper Bevy resource for the pure-data GameConfig (keeps bm_config free of bevy dependency).
#[derive(Resource, Debug, Clone)]
pub struct GameConfigRes(pub bm_config::GameConfig);

impl Default for GameConfigRes {
    fn default() -> Self {
        Self(bm_config::GameConfig::default())
    }
}

impl Default for RngSeed {
    fn default() -> Self {
        Self(0)
    }
}

// System set labels (ported from legacy/system_order.rs)
#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub struct PrePhysicsSet; // forces applied before physics simulation step
#[derive(SystemSet, Debug, Hash, Eq, PartialEq, Clone)]
pub struct PostPhysicsAdjustSet; // lightweight corrections after physics

// Core plugin (empty for now) registers sets to establish ordering contracts
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, (PrePhysicsSet.before(PostPhysicsAdjustSet),));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_adds_sets() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CorePlugin);
        // Presence check: add a dummy system in each set to ensure they exist.
        fn dummy() {}
        app.add_systems(Update, dummy.in_set(PrePhysicsSet));
        app.add_systems(Update, dummy.in_set(PostPhysicsAdjustSet));
    }
}
