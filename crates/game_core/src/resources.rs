use crate::GameColor;
use bevy::prelude::*;

#[derive(Resource, Debug)]
pub struct GameState {
    pub score: u32,
    pub lives: u8,
    pub won: bool,
    pub lost: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            score: 0,
            lives: 3,
            won: false,
            lost: false,
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct ArenaConfig {
    pub width: f32,
    pub height: f32,
    pub background: GameColor,
}

impl Default for ArenaConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            background: GameColor::White,
        }
    }
}

// === Sprint 4.5 Resources ===

#[derive(Debug, Clone, Copy)]
pub enum BallSpawnPolicyMode {
    Manual,
    Auto(f32), // interval seconds
}

#[derive(Resource, Debug)]
pub struct BallSpawnPolicy {
    pub mode: BallSpawnPolicyMode,
}

impl Default for BallSpawnPolicy {
    fn default() -> Self { Self { mode: BallSpawnPolicyMode::Manual } }
}

#[derive(Resource, Default, Debug)]
pub struct ActiveSpawnRotation {
    pub indices: Vec<Entity>,
    pub current: usize,
}

impl ActiveSpawnRotation {
    pub fn current_entity(&self) -> Option<Entity> { self.indices.get(self.current).copied() }
    pub fn advance(&mut self) {
        if !self.indices.is_empty() { self.current = (self.current + 1) % self.indices.len(); }
    }
    pub fn retreat(&mut self) {
        if !self.indices.is_empty() { self.current = (self.current + self.indices.len() - 1) % self.indices.len(); }
    }
    pub fn set_index(&mut self, idx: usize) {
        if idx < self.indices.len() { self.current = idx; }
    }
}

#[derive(Resource, Default, Debug)]
pub struct SpawnMetrics {
    pub total_spawned: u64,
    pub total_despawned: u64,
    pub active_balls: u64,
}
