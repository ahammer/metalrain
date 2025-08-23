use bevy::prelude::*;

/// Marker component identifying a ball entity parent (holds physics body & collider).
#[derive(Component)]
pub struct Ball;

/// Logical radius used both for the collider and rendering scale.
#[derive(Component, Debug, Deref, DerefMut, Copy, Clone)]
pub struct BallRadius(pub f32);

/// Tag component for the circle mesh child used in flat rendering modes.
#[derive(Component)]
pub struct BallCircleVisual;
