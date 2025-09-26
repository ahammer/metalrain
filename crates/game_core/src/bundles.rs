use bevy::prelude::*;
use crate::{Ball, GameColor};

#[derive(Bundle)]
pub struct BallBundle {
    pub ball: Ball,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl BallBundle {
    pub fn new(position: Vec2, radius: f32, color: GameColor) -> Self {
        Self {
            ball: Ball { velocity: Vec2::ZERO, radius, color },
            transform: Transform::from_translation(position.extend(0.0)),
            global_transform: GlobalTransform::IDENTITY,
        }
    }
}
