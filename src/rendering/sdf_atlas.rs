use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use serde::Deserialize;
use std::fs;
use std::path::Path;
/// Packing helpers (shape index high 16 bits, color group low 16 bits). Shape index 0 reserved sentinel.
#[inline] pub const fn pack_shape_color(shape_index: u16, color_group: u16) -> u32 { ((shape_index as u32) << 16) | (color_group as u32) }
#[inline] pub const fn unpack_shape(packed: u32) -> u16 { (packed >> 16) as u16 }
#[inline] pub const fn unpack_color_group(packed: u32) -> u16 { (packed & 0xFFFF) as u16 }

/// Runtime resource representing a loaded SDF shape atlas.
#[derive(Resource, Debug, Clone)]
pub struct SdfAtlas {
    pub texture: Handle<Image>,
    pub tile_size: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub distance_range: f32,
    pub channel_mode: SdfChannelMode,
    pub shapes: Vec<SdfShapeMeta>,
    pub enabled: bool,
    pub shape_buffer: Option<Handle<ShaderStorageBuffer>>, // GPU metadata buffer
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SdfChannelMode { #[default] SdfR8, MsdfRgb, MsdfRgba }
impl SdfChannelMode { #[inline] pub fn as_uniform(self) -> f32 { match self { SdfChannelMode::SdfR8 => 0.0, SdfChannelMode::MsdfRgb => 1.0, SdfChannelMode::MsdfRgba => 2.0 } } }

#[derive(Debug, Clone, Deserialize)]
pub struct RawShapeRect { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }
#[derive(Debug, Clone, Deserialize)]
pub struct RawShapeUv { pub u0: f32, pub v0: f32, pub u1: f32, pub v1: f32 }
#[derive(Debug, Clone, Deserialize)]
pub struct RawShapePivot { pub x: f32, pub y: f32 }
#[derive(Debug, Clone, Deserialize)]
pub struct RawShapeEntry {
    pub name: String,
    pub index: u32,
    pub px: RawShapeRect,
    pub uv: RawShapeUv,
    #[serde(default = "default_pivot")] pub pivot: RawShapePivot,
    #[serde(default)] pub advance_scale: f32,
    #[serde(default)] pub metadata: serde_json::Value,
}
fn default_pivot() -> RawShapePivot { RawShapePivot { x: 0.5, y: 0.5 } }

#[derive(Debug, Clone, Deserialize)]
pub struct RawAtlasJson {
    pub version: u32,
    #[serde(default)] pub distance_range: Option<f32>,
    pub tile_size: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    #[serde(default = "default_channel_mode")] pub channel_mode: String,
    pub shapes: Vec<RawShapeEntry>,
}
fn default_channel_mode() -> String { "sdf_r8".to_string() }

#[derive(Debug, Clone)]
pub struct SdfShapeMeta {
    pub name: String,
    pub index: u32,
    pub rect_px: (u32,u32,u32,u32),
    pub uv: (f32,f32,f32,f32),
    pub pivot: (f32,f32),
}

pub struct SdfAtlasPlugin;
impl Plugin for SdfAtlasPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_sdf_atlas)
            // After atlas loads, do a one-shot assignment of shape indices to any balls that
            // currently have the sentinel (0) so the shader path exercises SDF sampling.
            .add_systems(Update, assign_ball_shapes_once);
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, bevy::render::render_resource::ShaderType)]
pub struct SdfShapeGpuMeta {
    // uv0.xy, uv1.xy, pivot.xy, pad.xy (pad retained for future metrics e.g. advance_scale)
    pub uv0: Vec2,
    pub uv1: Vec2,
    pub pivot: Vec2,
    pub pad: Vec2,
}

fn load_sdf_atlas(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    _images: ResMut<Assets<Image>>,
    cfg: Res<crate::core::config::GameConfig>,
    existing: Option<Res<SdfAtlas>>,
) {
    if existing.is_some() { return; }
    let cfg_shapes = &cfg.sdf_shapes;
    // Early exit if disabled in config.
    if !cfg_shapes.enabled { return; }

    // Hard-coded provisional paths; could move to config later.
    // Note: AssetServer expects paths relative to the asset root (i.e. without leading 'assets/').
    let json_fs_path = "assets/shapes/sdf_atlas.json"; // filesystem check path
    let png_fs_path = "assets/shapes/sdf_atlas.png";   // filesystem check path
    let png_asset_path = "shapes/sdf_atlas.png";       // asset server relative path
    if !Path::new(json_fs_path).exists() || !Path::new(png_fs_path).exists() {
        info!(target:"sdf", "SDF atlas not found (expected '{}' and '{}') – falling back to analytic circles", json_fs_path, png_fs_path);
        commands.insert_resource(SdfAtlas { texture: Handle::default(), tile_size:0, atlas_width:0, atlas_height:0, distance_range:0.0, channel_mode: SdfChannelMode::SdfR8, shapes: vec![], enabled:false, shape_buffer: None });
        return;
    }
    let raw_str = match fs::read_to_string(json_fs_path) { Ok(s)=>s, Err(e)=>{ warn!(target:"sdf", "Failed reading atlas json: {e}"); return; } };
    let parsed: RawAtlasJson = match serde_json::from_str(&raw_str) { Ok(p)=>p, Err(e)=>{ warn!(target:"sdf", "JSON parse error: {e}"); return; } };
    if parsed.version != 1 { warn!(target:"sdf", "Unsupported SDF atlas version {}; expected 1", parsed.version); return; }
    if parsed.tile_size == 0 { warn!(target:"sdf", "tile_size must be > 0"); return; }
    if parsed.atlas_width % parsed.tile_size != 0 || parsed.atlas_height % parsed.tile_size != 0 { warn!(target:"sdf", "atlas dimensions must be multiples of tile_size"); }
    let tiles_capacity = (parsed.atlas_width / parsed.tile_size) * (parsed.atlas_height / parsed.tile_size);
    if parsed.shapes.len() as u32 > tiles_capacity { warn!(target:"sdf", "shapes count {} exceeds atlas tile capacity {}", parsed.shapes.len(), tiles_capacity); }

    let mut shapes_meta = Vec::with_capacity(parsed.shapes.len());
    for (i,s) in parsed.shapes.iter().enumerate() {
        let expected = (i + 1) as u32; // 1-based; 0 reserved sentinel
        if s.index == 0 { warn!(target:"sdf", "shape '{}' has index 0 (reserved). Expected {}.", s.name, expected); }
        else if s.index != expected { warn!(target:"sdf", "shape '{}' index {} != expected {} (non-contiguous)", s.name, s.index, expected); }
        shapes_meta.push(SdfShapeMeta { name: s.name.clone(), index: s.index, rect_px: (s.px.x, s.px.y, s.px.w, s.px.h), uv: (s.uv.u0, s.uv.v0, s.uv.u1, s.uv.v1), pivot: (s.pivot.x, s.pivot.y) });
    }

    // Load texture via asset server so it participates in Bevy's lifecycle; fallback if fails later.
    let tex_handle: Handle<Image> = asset_server.load(png_asset_path);
    let mut distance_range = parsed.distance_range.unwrap_or(parsed.tile_size as f32 * 0.125);
    let heuristic = parsed.tile_size as f32 * 0.125;
    if distance_range <= 0.0 { warn!(target:"sdf", "distance_range {} <= 0; using heuristic {}", distance_range, heuristic); distance_range = heuristic; }
    else if distance_range > parsed.tile_size as f32 { warn!(target:"sdf", "distance_range {} > tile_size {}; edges may appear soft", distance_range, parsed.tile_size); }
    let channel_mode = match parsed.channel_mode.as_str() { "sdf_r8"=>SdfChannelMode::SdfR8, "msdf_rgb"=>SdfChannelMode::MsdfRgb, "msdf_rgba"=>SdfChannelMode::MsdfRgba, other=>{ warn!(target:"sdf", "Unknown channel_mode '{}', defaulting to sdf_r8", other); SdfChannelMode::SdfR8 } };

    // Build GPU metadata buffer (index 0 reserved dummy so shape_index==0 => analytic circle fallback)
    let mut gpu_meta: Vec<SdfShapeGpuMeta> = Vec::with_capacity(shapes_meta.len()+1);
    gpu_meta.push(SdfShapeGpuMeta::default());
    for s in &shapes_meta {
        let (u0,v0,u1,v1) = s.uv;
        gpu_meta.push(SdfShapeGpuMeta { uv0: Vec2::new(u0, v0), uv1: Vec2::new(u1, v1), pivot: Vec2::new(s.pivot.0, s.pivot.1), pad: Vec2::ZERO });
    }
    let shape_buffer_handle = buffers.add(ShaderStorageBuffer::from(gpu_meta.as_slice()));

    let atlas = SdfAtlas { texture: tex_handle, tile_size: parsed.tile_size, atlas_width: parsed.atlas_width, atlas_height: parsed.atlas_height, distance_range, channel_mode, shapes: shapes_meta, enabled:true, shape_buffer: Some(shape_buffer_handle) };
    info!(target:"sdf", "Loaded SDF atlas: {} shapes, tile={} range={}", atlas.shapes.len(), atlas.tile_size, atlas.distance_range);
    commands.insert_resource(atlas);
}

// One-shot assignment: when an atlas is present & enabled, map each ball's material index
// deterministically onto a shape index (1..=shape_count). This keeps distribution stable
// across runs while avoiding per-frame work. Shape index 0 remains the analytic fallback.
fn assign_ball_shapes_once(
    atlas: Option<Res<SdfAtlas>>,
    mut done: Local<bool>,
    mut q: Query<(&crate::rendering::materials::materials::BallMaterialIndex, &mut crate::rendering::materials::materials::BallShapeIndex)>,
) {
    if *done { return; }
    let Some(atlas) = atlas else { return; };
    if !atlas.enabled || atlas.shapes.is_empty() { return; }
    let shape_count = atlas.shapes.len() as u16; // indices are 1..=shape_count
    for (mat_idx, mut shape_idx) in &mut q {
        if shape_idx.0 == 0 { // only overwrite sentinel
            // Deterministic mapping: wrap material index across available shapes then add 1
            // (since 0 is reserved sentinel analytic circle).
            shape_idx.0 = (mat_idx.0 as u16 % shape_count) + 1;
        }
    }
    *done = true;
    info!(target:"sdf", "Assigned initial SDF shape indices to balls ({} shapes)", shape_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_roundtrip() {
        let packed = pack_shape_color(0x1234, 0xABCD);
        assert_eq!(unpack_shape(packed), 0x1234);
        assert_eq!(unpack_color_group(packed), 0xABCD);
    }

    #[test]
    fn channel_mode_uniform_values() {
        assert_eq!(SdfChannelMode::SdfR8.as_uniform(), 0.0);
        assert_eq!(SdfChannelMode::MsdfRgb.as_uniform(), 1.0);
        assert_eq!(SdfChannelMode::MsdfRgba.as_uniform(), 2.0);
    }
}
