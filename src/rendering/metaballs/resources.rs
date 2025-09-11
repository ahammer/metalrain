//! ECS Resources, components, enums, and params for metaballs module.
use bevy::prelude::*;

use crate::rendering::metaballs::gpu::GpuBall;

#[derive(Component)]
pub struct MetaballsUnifiedQuad;

#[derive(Resource, Default)]
pub struct MetaballsToggle(pub bool);

#[derive(Resource, Debug, Clone)]
pub struct MetaballsParams {
    pub iso: f32,
    pub normal_z_scale: f32,
    pub radius_multiplier: f32,
}
impl Default for MetaballsParams {
    fn default() -> Self {
        Self { iso: 0.6, normal_z_scale: 1.0, radius_multiplier: 1.0 }
    }
}

// Shadow params resource (single-pass drop shadow halo)
#[derive(Resource, Debug, Clone)]
pub struct MetaballsShadowParams {
    pub enabled: bool,
    pub intensity: f32,
    pub offset: f32,
    pub softness: f32,
}
impl Default for MetaballsShadowParams {
    fn default() -> Self {
        Self { enabled: true, intensity: 0.65, offset: 40.0, softness: 0.6 }
    }
}

// CPU shadow of GPU balls for tiling
#[derive(Resource, Default, Clone)]
pub struct BallCpuShadow(pub Vec<GpuBall>);

#[derive(Resource, Debug, Clone)]
pub struct BallTilingConfig { pub tile_size: u32 }
impl Default for BallTilingConfig { fn default() -> Self { Self { tile_size: 64 } } }

#[derive(Resource, Debug, Clone, Default)]
pub struct BallTilesMeta { pub tiles_x: u32, pub tiles_y: u32, pub last_ball_len: usize }

// Foreground / Background shader modes
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode { ClassicBlend, Bevel, OutlineGlow, Metadata }
impl MetaballForegroundMode { pub const ALL: [Self; 4] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow, Self::Metadata]; }

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballBackgroundMode { SolidGray, ProceduralNoise, VerticalGradient }
impl MetaballBackgroundMode { pub const ALL: [Self; 3] = [Self::SolidGray, Self::ProceduralNoise, Self::VerticalGradient]; }

#[derive(Resource, Debug, Default)]
pub struct MetaballForeground { pub idx: usize }
impl MetaballForeground { pub fn current(&self) -> MetaballForegroundMode { MetaballForegroundMode::ALL[self.idx % MetaballForegroundMode::ALL.len()] } }

#[derive(Resource, Debug, Default)]
pub struct MetaballBackground { pub idx: usize }
impl MetaballBackground { pub fn current(&self) -> MetaballBackgroundMode { MetaballBackgroundMode::ALL[self.idx % MetaballBackgroundMode::ALL.len()] } }

// Debug timer
#[derive(Resource)]
pub struct MetaballsGroupDebugTimer(pub Timer);
impl Default for MetaballsGroupDebugTimer { fn default() -> Self { Self(Timer::from_seconds(1.0, TimerMode::Repeating)) } }
