use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use crate::constants::MAX_BALLS;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ball {
    pub center: [f32; 2],
    pub radius: f32,
    pub cluster_id: i32,
    pub color: [f32; 4],
}

#[derive(Resource, Clone, ExtractResource)]
pub struct MetaballBuffer {
    pub balls: Vec<Ball>,
}

#[repr(C)]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct MetaballTime {
    pub time: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct MetaballParams {
    // Order must match WGSL struct Params
    pub screen_size: [f32; 2],
    pub num_balls: u32,
    pub debug_mode: u32,     // _unused0 in WGSL
    pub iso: f32,            // _unused1 in WGSL (currently used here)
    pub ambient: f32,        // _unused2 in WGSL (currently used here)
    pub rim_power: f32,      // _unused3 in WGSL (currently used here)
    pub show_centers: u32,   // _unused4 in WGSL (currently used here)
    pub clustering_enabled: u32,
    pub _pad: f32,
}

#[derive(Resource, Clone, ExtractResource)]
pub struct MetaballOutputTexture {
    pub texture: Handle<Image>,
}

#[derive(Resource, Clone, ExtractResource)]
pub struct MetaballAlbedoTexture {
    pub texture: Handle<Image>,
}

impl Default for MetaballTime {
    fn default() -> Self { Self { time: 0.0, _pad: [0.0; 3] } }
}

pub fn padded_balls_slice(src: &[Ball]) -> [Ball; MAX_BALLS] {
    let mut fixed = [Ball { center: [0.0, 0.0], radius: 0.0, cluster_id: 0, color: [0.0, 0.0, 0.0, 0.0] }; MAX_BALLS];
    for (i, b) in src.iter().take(MAX_BALLS).enumerate() { fixed[i] = *b; }
    fixed
}
