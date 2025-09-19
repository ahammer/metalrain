//! Internal GPU data structs (dynamic storage buffer version; no MAX_BALLS cap).
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

pub const WORKGROUP_SIZE: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BallGpu {
    pub center: [f32;2],
    pub radius: f32,
    /// Cluster identifier used by the compute shader when `clustering_enabled > 0`.
    /// Currently any i32 value is accepted; 0 is a neutral default. If a "no cluster" sentinel
    /// (e.g. -1) becomes required, adjust the shader to skip those entries and update this doc.
    pub cluster_id: i32,
    pub color: [f32;4],
}

#[repr(C)]
#[derive(Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct TimeUniform { pub time: f32, _pad: [f32;3] }

// Keep layout in sync with WGSL `struct Params`.
#[repr(C, align(16))]
#[derive(Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct ParamsUniform {
    pub screen_size: [f32;2], // 0..8
    pub num_balls: u32,       // 8..12
    pub clustering_enabled: u32, // 12..16
    // total 16 bytes (16B aligned)
}

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct FieldTexture(pub Handle<Image>);
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct AlbedoTexture(pub Handle<Image>);

#[derive(Resource, Clone, Debug, ExtractResource, Default)]
pub struct BallBuffer { pub balls: Vec<BallGpu> }

