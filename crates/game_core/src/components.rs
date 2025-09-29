use bevy::prelude::*;

/// Basic color enum for high‑level game logic (separate from Bevy `Color`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameColor {
    Red,
    Green,
    Blue,
    Yellow,
    White,
}

impl Default for GameColor {
    fn default() -> Self { GameColor::White }
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

#[derive(Clone, Debug)]
pub enum TargetState {
    Idle,
    Hit(f32),        // animation progress 0..1
    Destroying(f32), // animation progress 0..1
}

impl Default for TargetState { fn default() -> Self { TargetState::Idle } }

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

