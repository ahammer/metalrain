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

/// Enabled/Disabled state + timestamp of last change (seconds since startup).
#[derive(Component, Debug, Copy, Clone)]
pub struct BallState {
    pub enabled: bool,
    pub last_change: f32,
}
impl BallState {
    pub fn new(now: f32) -> Self {
        Self {
            enabled: true,
            last_change: now,
        }
    }
}
