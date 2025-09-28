use bevy::prelude::*;

/// Component attached to the primary gameplay camera controlling shake/zoom state.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct GameCamera {
    pub base_resolution: Vec2,
    pub viewport_scale: f32,
    pub shake_intensity: f32,
    pub shake_decay_rate: f32,
    pub shake_offset: Vec2,
    pub zoom_bounds: Vec2,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            base_resolution: Vec2::new(1280.0, 720.0),
            viewport_scale: 1.0,
            shake_intensity: 0.0,
            shake_decay_rate: 2.5,
            shake_offset: Vec2::ZERO,
            zoom_bounds: Vec2::new(0.5, 2.0),
        }
    }
}

/// Configuration resource for camera behavior.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct GameCameraSettings {
    pub shake_decay: f32,
    pub zoom_speed: f32,
}

impl Default for GameCameraSettings {
    fn default() -> Self {
        Self {
            shake_decay: 2.5,
            zoom_speed: 1.25,
        }
    }
}

/// Event to trigger camera shake with a starting intensity.
#[derive(Event, Debug, Clone, Copy)]
pub struct CameraShakeCommand {
    pub intensity: f32,
}

/// Event to request a zoom change (positive zooms in, negative zooms out).
#[derive(Event, Debug, Clone, Copy)]
pub struct CameraZoomCommand {
    pub delta_scale: f32,
}
