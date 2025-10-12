use bevy::prelude::*;
use std::collections::VecDeque;

/// Metadata describing the owning demo for diagnostics displays.
#[derive(Resource, Debug, Clone)]
pub struct ScaffoldMetadata {
    demo_name: String,
}

impl ScaffoldMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            demo_name: name.into(),
        }
    }
    pub fn demo_name(&self) -> &str {
        &self.demo_name
    }
}

/// Configuration for world bounds, resolution targets, and baseline physics.
#[derive(Resource, Debug, Clone)]
pub struct ScaffoldConfig {
    pub base_resolution: UVec2,
    pub metaball_texture_size: UVec2,
    pub world_half_extent: f32,
    pub wall_thickness: f32,
    pub default_gravity: Vec2,
}

impl Default for ScaffoldConfig {
    fn default() -> Self {
        Self {
            base_resolution: UVec2::new(1280, 720),
            metaball_texture_size: UVec2::new(512, 512),
            world_half_extent: 256.0,
            wall_thickness: 10.0,
            default_gravity: Vec2::ZERO,
        }
    }
}

impl ScaffoldConfig {
    pub fn with_base_resolution(mut self, resolution: UVec2) -> Self {
        self.base_resolution = resolution;
        self
    }
    pub fn with_metaball_texture_size(mut self, size: UVec2) -> Self {
        self.metaball_texture_size = size;
        self
    }
    pub fn with_world_half_extent(mut self, half_extent: f32) -> Self {
        self.world_half_extent = half_extent;
        self
    }
    pub fn with_wall_thickness(mut self, thickness: f32) -> Self {
        self.wall_thickness = thickness;
        self
    }
    pub fn with_default_gravity(mut self, gravity: Vec2) -> Self {
        self.default_gravity = gravity;
        self
    }
}

/// Toggleable HUD visibility controlled by standard bindings.
#[derive(Resource, Debug, Clone)]
pub struct ScaffoldHudState {
    pub visible: bool,
}

impl Default for ScaffoldHudState {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Sliding-window frame statistics used by the HUD overlay.
#[derive(Resource, Debug, Default)]
pub struct ScaffoldPerformanceStats {
    pub frames: u64,
    pub last_sample_time: f32,
    pub recent: VecDeque<(f32, f32)>,
}

impl ScaffoldPerformanceStats {
    pub fn record_sample(&mut self, timestamp: f32, delta: f32) {
        self.frames += 1;
        self.last_sample_time = timestamp;
        self.recent.push_back((timestamp, delta.max(0.0)));
        while let Some((t, _)) = self.recent.front() {
            if timestamp - *t > 6.0 {
                self.recent.pop_front();
            } else {
                break;
            }
        }
    }
}

/// Runtime metaball render modes supported by the scaffold controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballMode {
    Clustered,
    NoClustering,
    Hidden,
}

impl MetaballMode {
    pub fn label(self) -> &'static str {
        match self {
            MetaballMode::Clustered => "Clustered",
            MetaballMode::NoClustering => "No Clustering",
            MetaballMode::Hidden => "Hidden",
        }
    }

    pub fn next(self) -> Self {
        match self {
            MetaballMode::Clustered => MetaballMode::NoClustering,
            MetaballMode::NoClustering => MetaballMode::Hidden,
            MetaballMode::Hidden => MetaballMode::Clustered,
        }
    }
}

/// Tracks the currently selected metaball presentation mode.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ScaffoldMetaballMode {
    pub mode: MetaballMode,
}

impl Default for ScaffoldMetaballMode {
    fn default() -> Self {
        Self {
            mode: MetaballMode::Clustered,
        }
    }
}
