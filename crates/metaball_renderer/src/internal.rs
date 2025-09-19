//! Internal constants & GPU data structs (temporary during extraction Phase 2).
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

pub const WORKGROUP_SIZE: u32 = 8;
pub const MAX_BALLS: usize = 512; // matches POC for now

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BallGpu { pub center: [f32;2], pub radius: f32, pub cluster_id: i32, pub color: [f32;4] }

#[repr(C)]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct TimeUniform { pub time: f32, _pad: [f32;3] }
impl Default for TimeUniform { fn default() -> Self { Self { time: 0.0, _pad: [0.0;3] } } }

// NOTE: Keep layout in sync with WGSL `struct Params` (the shader only consumes the
// leading fields currently; trailing padding is explicit so total size is a 16B multiple).
// We rely on stable C layout for uniform buffer binding safety.
#[repr(C, align(16))]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct ParamsUniform {
    pub screen_size: [f32;2],      // 0..8
    pub num_balls: u32,            // 8..12
    pub _unused0: u32,             // 12..16
    pub iso: f32,                  // 16..20
    pub _unused2: f32,             // 20..24
    pub _unused3: f32,             // 24..28
    pub _unused4: u32,             // 28..32
    pub clustering_enabled: u32,   // 32..36
    pub _pad: [u32;3],             // 36..48 (explicit so no implicit padding; total size 48, 16B aligned)
}

#[derive(Resource, Clone, ExtractResource)]
pub struct FieldTexture(pub Handle<Image>);
#[derive(Resource, Clone, ExtractResource)]
pub struct AlbedoTexture(pub Handle<Image>);

#[derive(Resource, Clone, ExtractResource)]
pub struct BallBuffer { pub balls: Vec<BallGpu> }

pub fn padded_slice(src: &[BallGpu]) -> [BallGpu; MAX_BALLS] {
    let mut fixed = [BallGpu { center: [0.0,0.0], radius: 0.0, cluster_id: 0, color: [0.0;4] }; MAX_BALLS];
    for (i,b) in src.iter().take(MAX_BALLS).enumerate() { fixed[i] = *b; }
    fixed
}

#[derive(Resource, Default)]
pub(crate) struct OverflowWarned(pub bool);
