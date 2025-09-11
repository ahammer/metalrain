//! GPU data/layout types and constants for metaballs rendering.
//! Separated from plugin & systems for clarity and reuse in tests.
#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bytemuck::{Pod, Zeroable};

// =====================================================================================
// Constants
// =====================================================================================
/// Legacy cap; dynamic storage buffer length may exceed but we clamp exposed count.
pub const MAX_BALLS: usize = 1024;
/// Legacy clusters removed (kept for layout stability / shader expectations if any remain)
pub const MAX_CLUSTERS: usize = 0;

// =====================================================================================
// Uniform layout vectors (see original file header docs)
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)
// v4: (enable_early_exit, needs_gradient, metadata_v2_flag, reserved1)
// v5: (sdf_enabled_or_shadow_softness, distance_range, shadow_enable, shadow_intensity)
// v6: (atlas_width, atlas_height, atlas_tile_size, shadow_vertical_offset)
// v7: (shadow_dir_deg, shadow_surface_scale, reserved0, reserved1)
// =====================================================================================
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct MetaballsUniform {
    pub(crate) v0: Vec4,
    pub(crate) v1: Vec4,
    pub(crate) v2: Vec4,
    pub(crate) v3: Vec4,
    pub(crate) v4: Vec4,
    pub(crate) v5: Vec4,
    pub(crate) v6: Vec4,
    pub(crate) v7: Vec4,
}
impl Default for MetaballsUniform {
    fn default() -> Self {
        Self {
            v0: Vec4::new(0.0, 0.0, 1.0, 0.6),
            v1: Vec4::new(1.0, 0.0, 0.0, 0.0),
            v2: Vec4::new(0.0, 0.0, 0.0, 1.0),
            v3: Vec4::new(1.0, 1.0, 64.0, 0.0),
            v4: Vec4::new(1.0, 0.0, 0.0, 0.0),
            v5: Vec4::new(0.0, 0.0, 0.0, 0.0),
            v6: Vec4::new(0.0, 0.0, 0.0, 0.0),
            v7: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

// =====================================================================================
// Storage Buffer Types
// =====================================================================================
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default, Pod, Zeroable)]
pub struct GpuBall {
    /// data0: (x, y, radius, packed_gid)
    pub data0: Vec4,
    /// data1: (cos_theta, sin_theta, reserved0, reserved1)
    pub data1: Vec4,
}
impl GpuBall {
    pub fn new(pos: Vec2, radius: f32, packed_gid: u32, cos_theta: f32, sin_theta: f32) -> Self {
        Self {
            data0: Vec4::new(pos.x, pos.y, radius, packed_gid as f32),
            data1: Vec4::new(cos_theta, sin_theta, 0.0, 0.0),
        }
    }
}

// Background noise params
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct NoiseParamsUniform {
    pub base_scale: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}
impl Default for NoiseParamsUniform {
    fn default() -> Self {
        Self {
            base_scale: 0.004,
            warp_amp: 0.6,
            warp_freq: 0.5,
            speed_x: 0.03,
            speed_y: 0.02,
            gain: 0.5,
            lacunarity: 2.0,
            contrast_pow: 1.25,
            octaves: 5,
            ridged: 0,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

// Surface noise params (edge modulation)
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default)]
pub struct SurfaceNoiseParamsUniform {
    pub amp: f32,
    pub base_scale: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: u32,
    pub mode: u32,
    pub enabled: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
}

// Tile header (GPU)
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub struct TileHeaderGpu {
    pub offset: u32,
    pub count: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

// Helper mirroring WGSL normalization for metadata SDF proxy (kept pure for testing)
pub fn map_signed_distance(signed_d: f32, d_scale: f32) -> f32 {
    (0.5 - 0.5 * signed_d / d_scale).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn sdf_mapping_basic() {
        let scale = 8.0;
        let inside = map_signed_distance(-2.0, scale);
        assert!(inside > 0.5);
        let surface = map_signed_distance(0.0, scale);
        assert!((surface - 0.5).abs() < 1e-6);
        let outside = map_signed_distance(4.0, scale);
        assert!(outside < 0.5);
        let far = map_signed_distance(1e6, scale);
        assert!((0.0..=0.001).contains(&far));
    }

    #[allow(clippy::mutable_key_type)]
    #[test]
    fn color_group_assignment_basic() {
        // Simulate three balls with colors [0,0,2] => two groups.
        let mut color_to_group: HashMap<usize, u32> = HashMap::new();
        let colors = [0usize, 0, 2];
        let mut palette: Vec<[f32; 4]> = Vec::new();
        for c in colors.iter() {
            color_to_group.entry(*c).or_insert_with(|| {
                let gid = palette.len() as u32;
                palette.push([*c as f32, 0.0, 0.0, 1.0]);
                gid
            });
        }
        assert_eq!(palette.len(), 2);
        assert_eq!(color_to_group.get(&0).copied(), Some(0));
        assert_eq!(color_to_group.get(&2).copied(), Some(1));
    }
}
