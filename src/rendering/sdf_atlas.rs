use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
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
    // Cached for shader uniform staging (explicit instead of re-deriving per frame)
    pub shape_count: u32,
}

// ================= Glyph Mapping Resources (NEW) =================
/// Mapping from glyph character to SDF shape index (u16). Index 0 reserved sentinel (circle fallback).
#[derive(Resource, Debug, Default, Clone)]
pub struct GlyphShapeMap {
    pub map: HashMap<char, u16>,
    /// Sorted list of glyph chars for deterministic iteration/testing (not used per-frame).
    pub ordered: Vec<char>,
}

/// Tracks previous glyph_mode state to know when to (re)assign shapes on toggle transitions.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct GlyphModeState { pub prev_enabled: bool }

/// Monotonic ordinal counter incremented on each ball spawn (u64 to avoid wrap).
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct BallSpawnOrdinal(pub u64);

/// Cached processed glyph sequence (effective chars after whitespace filtering) + hash to avoid rebuild each frame.
#[derive(Resource, Debug, Default, Clone)]
pub struct GlyphSequenceCache {
    pub last_hash: u64,
    pub chars: Vec<char>,
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
            .add_systems(Update, assign_ball_shapes_once)
            .init_resource::<GlyphModeState>()
            .init_resource::<BallSpawnOrdinal>()
            .init_resource::<GlyphSequenceCache>()
            .add_systems(Update, assign_ball_glyph_shapes.after(assign_ball_shapes_once).before(crate::rendering::metaballs::metaballs::MetaballsUpdateSet));
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, bevy::render::render_resource::ShaderType)]
pub struct SdfShapeGpuMeta {
    // uv0.xy, uv1.xy, pivot.xy, pad.xy (pad retained for future metrics e.g. advance_scale)
    pub uv0: Vec2,
    pub uv1: Vec2,
    pub pivot: Vec2,
    // meta.x = tile_size_px, meta.y = distance_range_px (global currently but stored per-shape for future variance)
    // meta.z = reserved (0), meta.w = reserved (0)
    // NOTE: field formerly named `meta` in WGSL; renamed to `params` to avoid reserved identifier collision.
    // params.x = tile_size_px, params.y = distance_range_px, params.z/.w reserved
    pub params: Vec4,
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
    commands.insert_resource(SdfAtlas { texture: Handle::default(), tile_size:0, atlas_width:0, atlas_height:0, distance_range:0.0, channel_mode: SdfChannelMode::SdfR8, shapes: vec![], enabled:false, shape_buffer: None, shape_count: 0 });
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
    gpu_meta.push(SdfShapeGpuMeta { uv0: Vec2::ZERO, uv1: Vec2::ZERO, pivot: Vec2::ZERO, params: Vec4::ZERO }); // index 0 sentinel
    for s in &shapes_meta {
        let (u0,v0,u1,v1) = s.uv;
        gpu_meta.push(SdfShapeGpuMeta {
            uv0: Vec2::new(u0, v0),
            uv1: Vec2::new(u1, v1),
            pivot: Vec2::new(s.pivot.0, s.pivot.1),
            params: Vec4::new(parsed.tile_size as f32, distance_range, 0.0, 0.0),
        });
    }
    let shape_buffer_handle = buffers.add(ShaderStorageBuffer::from(gpu_meta.as_slice()));

    let atlas = SdfAtlas { texture: tex_handle, tile_size: parsed.tile_size, atlas_width: parsed.atlas_width, atlas_height: parsed.atlas_height, distance_range, channel_mode, shapes: shapes_meta, enabled:true, shape_buffer: Some(shape_buffer_handle), shape_count: parsed.shapes.len() as u32 };
    info!(target:"sdf", "Loaded SDF atlas: {} shapes, tile={} range={}", atlas.shapes.len(), atlas.tile_size, atlas.distance_range);
    // Build glyph map (glyph_<char>) entries; keep first occurrence, warn on duplicates.
    if !atlas.shapes.is_empty() {
        let mut glyph_map: HashMap<char, u16> = HashMap::new();
        let mut duplicates: Vec<char> = Vec::new();
        for s in &atlas.shapes {
            let name = s.name.as_str();
            if let Some(rest) = name.strip_prefix("glyph_") {
                // Accept single char A-Z a-z 0-9 punctuation subset (take first char)
                if let Some(ch) = rest.chars().next() {
                    if rest.chars().count() == 1 { // only accept single char suffix per spec
                        let idx_u16 = (s.index as u16).max(0); // index already validated >=1 usually
                        if glyph_map.contains_key(&ch) { duplicates.push(ch); }
                        else { glyph_map.insert(ch, idx_u16); }
                    }
                }
            }
        }
        if !glyph_map.is_empty() {
            let mut ordered: Vec<char> = glyph_map.keys().copied().collect();
            ordered.sort();
            let glyph_count = ordered.len();
            commands.insert_resource(GlyphShapeMap { map: glyph_map, ordered });
            if cfg.sdf_shapes.glyph_mode || cfg!(feature="debug") {
                info!(target="sdf", "Glyph map built: {} glyphs", glyph_count);
            }
            if !duplicates.is_empty() {
                duplicates.sort(); duplicates.dedup();
                warn!(target="sdf", "Duplicate glyph shape entries ignored for chars: {:?}", duplicates);
            }
        } else if cfg.sdf_shapes.glyph_mode {
            warn!(target="sdf", "Glyph mode enabled but no glyph_* entries present in atlas");
        }
    }
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

// ================= Glyph Assignment =================
fn assign_ball_glyph_shapes(
    atlas: Option<Res<SdfAtlas>>,
    glyph_map: Option<Res<GlyphShapeMap>>,
    mut mode_state: ResMut<GlyphModeState>,
    mut seq_cache: ResMut<GlyphSequenceCache>,
    cfg: Res<crate::core::config::GameConfig>,
    mut q: Query<(&crate::core::components::BallOrdinal, &crate::core::components::BallRadius, Option<&mut crate::rendering::materials::materials::BallShapeIndex>), With<crate::core::components::Ball>>,
    mut missing_local: Local<std::collections::HashSet<char>>,
) {
    let Some(atlas) = atlas else { return; };
    if !atlas.enabled || atlas.shapes.is_empty() { return; }
    if !cfg.sdf_shapes.enabled || cfg.sdf_shapes.force_fallback { return; }
    if !cfg.sdf_shapes.glyph_mode { mode_state.prev_enabled = false; return; }
    let Some(glyph_map) = glyph_map else { return; };

    // Build / refresh effective char sequence cache when config changes
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    cfg.sdf_shapes.glyph_text.hash(&mut hasher);
    cfg.sdf_shapes.glyph_wrap.hash(&mut hasher);
    cfg.sdf_shapes.glyph_skip_whitespace.hash(&mut hasher);
    let new_hash = hasher.finish();
    if new_hash != seq_cache.last_hash {
        seq_cache.chars.clear();
        for ch in cfg.sdf_shapes.glyph_text.chars() {
            if cfg.sdf_shapes.glyph_skip_whitespace && ch.is_whitespace() { continue; }
            seq_cache.chars.push(ch);
        }
        seq_cache.last_hash = new_hash;
    }
    if seq_cache.chars.is_empty() { return; }
    let wrap_mode = match cfg.sdf_shapes.glyph_wrap.as_str() { "Repeat"=>0, "Clamp"=>1, "None"=>2, other=>{ warn!(target="sdf", "Invalid glyph_wrap '{}' (expected Repeat|Clamp|None) treating as Repeat", other); 0 } };
    let radius_threshold = cfg.sdf_shapes.use_circle_fallback_when_radius_lt.max(0.0);

    // Gather balls and sort by ordinal for determinism
    let mut items: Vec<(u64, f32, Option<Mut<crate::rendering::materials::materials::BallShapeIndex>>)> = Vec::with_capacity(q.iter().len());
    for (ord, rad, shape_opt) in q.iter_mut() { items.push((ord.0, rad.0, shape_opt)); }
    items.sort_by_key(|(o,_,_)| *o);
    let len_eff = seq_cache.chars.len() as u64;
    for (ord, radius, shape_opt) in items.into_iter() {
        let Some(mut shape_idx_comp) = shape_opt else { continue; };
        if shape_idx_comp.0 != 0 && mode_state.prev_enabled { continue; }
        let ch_opt = match wrap_mode { 0 => Some(seq_cache.chars[(ord % len_eff) as usize]), 1 => { let idx = if ord >= len_eff { len_eff - 1 } else { ord }; Some(seq_cache.chars[idx as usize]) }, 2 => { if ord >= len_eff { None } else { Some(seq_cache.chars[ord as usize]) } }, _ => None };
        if let Some(ch) = ch_opt {
            if radius < radius_threshold { shape_idx_comp.0 = 0; continue; }
            if let Some(si) = glyph_map.map.get(&ch).copied() { shape_idx_comp.0 = si; }
            else { shape_idx_comp.0 = 0; if !missing_local.contains(&ch) { missing_local.insert(ch); warn!(target="sdf", "Missing glyph '{}' in atlas (using circle fallback)", ch); } }
        } else if wrap_mode == 2 { break; }
    }
    mode_state.prev_enabled = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use crate::core::components::{Ball, BallRadius, BallOrdinal};
    use crate::rendering::materials::materials::BallShapeIndex;

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

    // ---------------- Glyph Mapping & Assignment Tests ----------------
    fn make_game_config_glyph(text: &str, wrap: &str) -> crate::core::config::GameConfig {
        let mut cfg = crate::core::config::GameConfig::default();
        cfg.sdf_shapes.glyph_mode = true;
        cfg.sdf_shapes.glyph_text = text.to_string();
        cfg.sdf_shapes.glyph_wrap = wrap.to_string();
        cfg.sdf_shapes.enabled = true;
        cfg
    }

    fn insert_basic_glyph_map(world: &mut bevy::prelude::World) {
        let mut map = HashMap::new();
        map.insert('A', 1u16);
        map.insert('B', 2u16);
        map.insert('z', 3u16);
        let mut ordered: Vec<char> = map.keys().copied().collect(); ordered.sort();
        world.insert_resource(GlyphShapeMap { map, ordered });
        world.insert_resource(GlyphModeState { prev_enabled: false });
        world.insert_resource(GlyphSequenceCache::default());
        world.insert_resource(BallSpawnOrdinal(0));
        // Minimal atlas resource (enabled=true, shapes non-empty)
        world.insert_resource(SdfAtlas { texture: Handle::default(), tile_size:16, atlas_width:16, atlas_height:16, distance_range:4.0, channel_mode: SdfChannelMode::SdfR8, shapes: vec![SdfShapeMeta{name:"glyph_A".into(), index:1, rect_px:(0,0,16,16), uv:(0.0,0.0,1.0,1.0), pivot:(0.5,0.5)}], enabled:true, shape_buffer: None, shape_count:1 });
    }

    fn spawn_balls(world: &mut bevy::prelude::World, n: usize) {
        for i in 0..n { world.spawn((Ball, BallRadius(10.0), BallOrdinal(i as u64), BallShapeIndex(0))); }
    }

    #[test]
    fn assignment_repeat_policy() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let cfg = make_game_config_glyph("AB", "Repeat");
        app.insert_resource(cfg);
        insert_basic_glyph_map(app.world_mut());
        spawn_balls(app.world_mut(), 5);
        let _ = app.world_mut().run_system_once(assign_ball_glyph_shapes);
        let indices: Vec<u16> = {
            let mut world = app.world_mut();
            let mut q = world.query::<(&BallOrdinal, &BallShapeIndex)>();
            let mut out = Vec::new();
            for (_o,s) in q.iter(&world) { out.push(s.0); }
            out
        };
        assert_eq!(indices.len(),5);
        // Expect A(1),B(2),A(1),B(2),A(1) given mapping {A:1,B:2}
        assert_eq!(indices[0],1); assert_eq!(indices[1],2); assert_eq!(indices[2],1); assert_eq!(indices[3],2); assert_eq!(indices[4],1);
    }

    #[test]
    fn assignment_clamp_policy() {
    let mut app = App::new(); app.add_plugins(MinimalPlugins);
    let cfg = make_game_config_glyph("AB", "Clamp"); app.insert_resource(cfg); insert_basic_glyph_map(app.world_mut()); spawn_balls(app.world_mut(),5); let _ = app.world_mut().run_system_once(assign_ball_glyph_shapes); let indices: Vec<u16> = { let mut w = app.world_mut(); let mut q = w.query::<(&BallOrdinal,&BallShapeIndex)>(); let mut v=Vec::new(); for (_o,s) in q.iter(&w){v.push(s.0);} v }; assert_eq!(indices,[1,2,2,2,2]); }

    #[test]
    fn assignment_none_policy() {
    let mut app = App::new(); app.add_plugins(MinimalPlugins); let cfg = make_game_config_glyph("AB", "None"); app.insert_resource(cfg); insert_basic_glyph_map(app.world_mut()); spawn_balls(app.world_mut(),5); let _ = app.world_mut().run_system_once(assign_ball_glyph_shapes); let indices: Vec<u16> = { let mut w = app.world_mut(); let mut q = w.query::<(&BallOrdinal,&BallShapeIndex)>(); let mut v=Vec::new(); for (_o,s) in q.iter(&w){v.push(s.0);} v }; assert_eq!(indices,[1,2,0,0,0]); }

    #[test]
    fn radius_threshold_fallback() {
    let mut app = App::new(); app.add_plugins(MinimalPlugins); let mut cfg = make_game_config_glyph("AB", "Repeat"); cfg.sdf_shapes.use_circle_fallback_when_radius_lt = 50.0; app.insert_resource(cfg); insert_basic_glyph_map(app.world_mut()); spawn_balls(app.world_mut(),3); let _ = app.world_mut().run_system_once(assign_ball_glyph_shapes); let indices: Vec<u16> = { let mut w = app.world_mut(); let mut q = w.query::<(&BallOrdinal,&BallShapeIndex)>(); let mut v=Vec::new(); for (_o,s) in q.iter(&w){v.push(s.0);} v }; assert_eq!(indices,[0,0,0]); }

    #[test]
    fn missing_glyph_fallback() {
    let mut app = App::new(); app.add_plugins(MinimalPlugins); let cfg = make_game_config_glyph("AZ", "Repeat"); app.insert_resource(cfg); insert_basic_glyph_map(app.world_mut()); spawn_balls(app.world_mut(),3); let _ = app.world_mut().run_system_once(assign_ball_glyph_shapes); let indices: Vec<u16> = { let mut w = app.world_mut(); let mut q = w.query::<(&BallOrdinal,&BallShapeIndex)>(); let mut v=Vec::new(); for (_o,s) in q.iter(&w){v.push(s.0);} v }; assert_eq!(indices,[1,0,1]); }
}
