use bevy::prelude::*;

pub mod app_state;
pub mod bundles;
pub mod components;
pub mod events;
pub mod resources;
mod spawning;

pub use app_state::AppState;
pub use bundles::*;
pub use components::*;
pub use events::*;
pub use resources::*;
pub use spawning::*;

pub struct GameCorePlugin;
impl Plugin for GameCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BallSpawned>()
            .add_event::<TargetDestroyed>()
            .add_event::<GameWon>()
            .add_event::<GameLost>()
            .add_event::<SpawnBallEvent>()
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
