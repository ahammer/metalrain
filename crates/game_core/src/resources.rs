use bevy::prelude::*;
use crate::GameColor;

#[derive(Resource, Debug)]
pub struct GameState {
    pub score: u32,
    pub lives: u8,
    pub won: bool,
    pub lost: bool,
}

impl Default for GameState {
    fn default() -> Self { Self { score: 0, lives: 3, won: false, lost: false } }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct ArenaConfig {
    pub width: f32,
    pub height: f32,
    pub background: GameColor,
}

impl Default for ArenaConfig {
    fn default() -> Self { Self { width: 800.0, height: 600.0, background: GameColor::White } }
}
