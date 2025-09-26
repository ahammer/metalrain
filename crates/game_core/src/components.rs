use bevy::prelude::*;

/// Basic color enum for prototyping (expand or replace later).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameColor { Red, Green, Blue, Yellow, White }

impl Default for GameColor { fn default() -> Self { GameColor::White } }

/// Simple 2D line segment; can be replaced by a math crate or Bevy primitive later.
#[derive(Clone, Copy, Debug)]
pub struct LineSegment { pub start: Vec2, pub end: Vec2 }

#[derive(Component, Clone, Copy, Debug)]
pub struct Ball {
    pub velocity: Vec2,
    pub radius: f32,
    pub color: GameColor,
}

#[derive(Component, Debug)]
pub struct Wall { pub segments: Vec<LineSegment> }

#[derive(Component, Debug)]
pub struct Target { pub health: u8, pub color: Option<GameColor> }

#[derive(Component, Debug)]
pub struct Hazard { pub damage: u32 }
