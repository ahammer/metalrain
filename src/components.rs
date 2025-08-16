use bevy::prelude::*;

#[derive(Component)]
pub struct Ball;

#[derive(Component, Debug, Deref, DerefMut, Copy, Clone)]
pub struct BallRadius(pub f32);
