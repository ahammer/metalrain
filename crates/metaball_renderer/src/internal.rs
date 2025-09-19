//! Internal constants & GPU data structs (temporary during extraction Phase 2).
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

pub const WORKGROUP_SIZE: u32 = 8;
pub const MAX_BALLS: usize = 512; // MUST stay in sync with compute shader loop bounds / expectations.
// NOTE: Mirror value used for compile-time sanity. If shader-side max changes, update both.
const _ASSERT_MAX_BALLS: () = {
    static_assertions::const_assert!(MAX_BALLS == 512);
};

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

// NOTE: Keep layout in sync with WGSL `struct Params` (the shader only consumes the
// leading fields currently; trailing padding is explicit so total size is a 16B multiple).
// We rely on stable C layout for uniform buffer binding safety.
#[repr(C, align(16))]
#[derive(Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct ParamsUniform {
    pub screen_size: [f32;2],      // 0..8
    pub num_balls: u32,            // 8..12
    pub _unused0: u32,             // 12..16 (reserved: future field scaling or dispatch variant id)
    pub iso: f32,                  // 16..20 (isosurface threshold for present pass SDF use)
    pub _unused2: f32,             // 20..24 (reserved: gradient normalization tweak)
    pub _unused3: f32,             // 24..28 (reserved)
    pub _unused4: u32,             // 28..32 (reserved flags extension)
    pub clustering_enabled: u32,   // 32..36
    pub _pad: [u32;3],             // 36..48 (explicit so no implicit padding; total size 48, 16B aligned)
}

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct FieldTexture(pub Handle<Image>);
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct AlbedoTexture(pub Handle<Image>);

#[derive(Resource, Clone, Debug, ExtractResource, Default)]
pub struct BallBuffer { pub balls: Vec<BallGpu> }

// TODO: Add unit test verifying: length < MAX_BALLS zero-fills remainder, length > MAX_BALLS truncates and warns once.
pub fn padded_slice(src: &[BallGpu], warned: &mut OverflowWarned) -> [BallGpu; MAX_BALLS] {
    // TODO: replace with a persistent resource or storage buffer pathway once dynamic counts grow.
    let mut fixed = [BallGpu { center: [0.0,0.0], radius: 0.0, cluster_id: 0, color: [0.0;4] }; MAX_BALLS];
    if src.len() > MAX_BALLS && !warned.0 {
        warn!(target: "metaballs", "Ball count {} exceeds MAX_BALLS {}; truncating (this warning is one-time)", src.len(), MAX_BALLS);
        warned.0 = true;
    }
    for (i,b) in src.iter().take(MAX_BALLS).enumerate() { fixed[i] = *b; }
    fixed
}

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct OverflowWarned(pub bool);
