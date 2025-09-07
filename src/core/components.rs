use bevy::prelude::*;

/// Marker component identifying a ball entity parent (holds physics body & collider).
#[derive(Component)]
pub struct Ball;

/// Logical radius used both for the collider and rendering scale.
#[derive(Component, Debug, Deref, DerefMut, Copy, Clone)]
pub struct BallRadius(pub f32);

/// Stable spawn order ordinal captured at creation for deterministic glyph mapping.
#[derive(Component, Debug, Copy, Clone)]
pub struct BallOrdinal(pub u64);
