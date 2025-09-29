use bevy::prelude::*;

/// Basic color enum for high‑level game logic (separate from Bevy `Color`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameColor {
    Red,
    Green,
    Blue,
    Yellow,
    #[default]
    White,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Ball {
    pub velocity: Vec2,
    pub radius: f32,
    pub color: GameColor,
}

// === Sprint 4 World Elements (updated to match sprint4 spec) ===

#[derive(Component, Clone, Debug)]
pub struct Wall {
    pub start: Vec2,
    pub end: Vec2,
    pub thickness: f32,
    pub color: Color,
}

impl Wall {
    pub fn new(start: Vec2, end: Vec2, thickness: f32, color: Color) -> Self {
        Self { start, end, thickness, color }
    }
    pub fn length(&self) -> f32 { self.start.distance(self.end) }
    pub fn center(&self) -> Vec2 { (self.start + self.end) * 0.5 }
}

#[derive(Clone, Debug, Default)]
pub enum TargetState {
    #[default]
    Idle,
    Hit(f32),        // animation progress 0..1
    Destroying(f32), // animation progress 0..1
}

#[derive(Component, Clone, Debug)]
pub struct Target {
    pub health: u8,
    pub max_health: u8,
    pub radius: f32,
    pub color: Color,
    pub state: TargetState,
}

impl Target {
    pub fn new(health: u8, radius: f32, color: Color) -> Self {
        Self { health, max_health: health, radius, color, state: TargetState::Idle }
    }
    pub fn is_destroyed(&self) -> bool { self.health == 0 }
}

#[derive(Clone, Debug)]
pub enum HazardType { Pit /* future: Laser, SlowZone, etc */ }

#[derive(Component, Clone, Debug)]
pub struct Hazard {
    pub bounds: Rect,
    pub hazard_type: HazardType,
}

impl Hazard {
    pub fn new(bounds: Rect, hazard_type: HazardType) -> Self { Self { bounds, hazard_type } }
    pub fn center(&self) -> Vec2 { self.bounds.center() }
    pub fn size(&self) -> Vec2 { self.bounds.size() }
}

// === Sprint 4.5: Dynamic Interaction Components ===

#[derive(Clone, Debug, Default)]
pub enum PaddleControl {
    #[default]
    Player,
    FollowCursor,
    Static,
}

#[derive(Component, Debug)]
pub struct Paddle {
    pub half_extents: Vec2, // size / 2
    pub move_speed: f32,    // units per second
    pub control: PaddleControl,
}

impl Default for Paddle {
    fn default() -> Self {
        Self {
            half_extents: Vec2::new(60.0, 10.0),
            move_speed: 600.0,
            control: PaddleControl::Player,
        }
    }
}

#[derive(Component, Debug)]
pub struct SpawnPoint {
    pub radius: f32,
    pub active: bool,
    pub cooldown: f32, // seconds between auto spawns (per point)
    pub timer: f32,    // internal accumulator
}

impl Default for SpawnPoint {
    fn default() -> Self {
        Self { radius: 14.0, active: true, cooldown: 0.0, timer: 0.0 }
    }
}

/// Marker for selection highlighting (visual crate may tint when present)
#[derive(Component, Debug, Default)]
pub struct Selected;

