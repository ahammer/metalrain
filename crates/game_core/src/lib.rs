//! game_core: foundational ECS types (components, resources, events) used across game crates.
//! Minimal initial implementation created in Sprint 1.

use bevy::prelude::*;

pub mod bundles;
pub mod components;
pub mod events;
pub mod resources;

pub use bundles::*;
pub use components::*;
pub use events::*;
pub use resources::*;

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
        let ball = Ball {
            velocity: Vec2::new(1.0, 0.0),
            radius: 5.0,
            color: GameColor::Red,
        };
        assert!(ball.radius > 0.0);
    }
}
