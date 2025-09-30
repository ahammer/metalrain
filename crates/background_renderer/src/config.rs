use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, PartialEq)]
pub enum BackgroundMode {
    #[default]
    Solid,
    LinearGradient,
    RadialGradient,
    Animated,
}

impl BackgroundMode {
    pub fn as_u32(self) -> u32 {
        match self {
            Self::Solid => 0,
            Self::LinearGradient => 1,
            Self::RadialGradient => 2,
            Self::Animated => 3,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Solid => Self::LinearGradient,
            Self::LinearGradient => Self::RadialGradient,
            Self::RadialGradient => Self::Animated,
            Self::Animated => Self::Solid,
        }
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct BackgroundConfig {
    pub mode: BackgroundMode,
    pub primary_color: LinearRgba,
    pub secondary_color: LinearRgba,
    pub angle: f32,          // radians for linear gradient direction
    pub animation_speed: f32, // cycles per second for animated mode
    pub radial_center: Vec2, // 0..1 uv space center
    pub radial_radius: f32,  // radius in uv distance
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            mode: BackgroundMode::LinearGradient,
            primary_color: LinearRgba::rgb(0.05, 0.05, 0.10),
            secondary_color: LinearRgba::rgb(0.02, 0.02, 0.05),
            angle: 0.25 * std::f32::consts::PI,
            animation_speed: 0.5,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.75,
        }
    }
}
