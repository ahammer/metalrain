use bevy::prelude::*;

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
