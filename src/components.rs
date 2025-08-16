use bevy::prelude::*;

#[derive(Component, Deref, DerefMut, Debug)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Ball;
