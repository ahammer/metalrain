//! game_core: foundational ECS types (components, resources, events) used across game crates.
//! Minimal initial implementation created in Sprint 1.

use bevy::prelude::*;

pub mod components;
pub mod resources;
pub mod events;
pub mod bundles;

pub use components::*;
pub use resources::*;
pub use events::*;
pub use bundles::*;

pub struct GameCorePlugin;
impl Plugin for GameCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BallSpawned>()
            .add_event::<TargetDestroyed>()
            .add_event::<GameWon>()
            .add_event::<GameLost>()
            .init_resource::<GameState>()
            .init_resource::<ArenaConfig>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_compile() {
        let ball = Ball { velocity: Vec2::new(1.0, 0.0), radius: 5.0, color: GameColor::Red };
        assert!(ball.radius > 0.0);
    }
}
