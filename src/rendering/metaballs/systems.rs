//! Core update systems: uniform updates, tiling, resizing, param tweaks, mode cycling.
use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite::MeshMaterial2d;
use std::collections::HashMap;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::GameConfig;
use crate::rendering::materials::materials::BallMaterialIndex;
use crate::rendering::metaballs::gpu::*;
use crate::rendering::metaballs::material::MetaballsUnifiedMaterial;
use crate::rendering::metaballs::resources::*;

pub fn cycle_foreground_mode(mut fg: ResMut<MetaballForeground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::End) { fg.idx = (fg.idx + 1) % MetaballForegroundMode::ALL.len(); info!(target: "metaballs", "Foreground mode -> {:?}", fg.current()); }
    if keys.just_pressed(KeyCode::Home) { fg.idx = (fg.idx + MetaballForegroundMode::ALL.len() - 1) % MetaballForegroundMode::ALL.len(); info!(target: "metaballs", "Foreground mode -> {:?}", fg.current()); }
}

pub fn cycle_background_mode(mut bg: ResMut<MetaballBackground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::PageUp) { bg.idx = (bg.idx + 1) % MetaballBackgroundMode::ALL.len(); info!(target: "metaballs", "Background mode -> {:?}", bg.current()); }
    if keys.just_pressed(KeyCode::PageDown) { bg.idx = (bg.idx + MetaballBackgroundMode::ALL.len() - 1) % MetaballBackgroundMode::ALL.len(); info!(target: "metaballs", "Background mode -> {:?}", bg.current()); }
}

pub fn update_metaballs_unified_material(
    time: Res<Time>,
    fg: Res<MetaballForeground>,
    bg: Res<MetaballBackground>,
    q_balls: Query<(Entity, &Transform, &BallRadius, &BallMaterialIndex, Option<&crate::rendering::materials::materials::BallShapeIndex>), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    cfg: Res<GameConfig>,
    mut shadow: Option<ResMut<BallCpuShadow>>,
    shadow_params: Res<MetaballsShadowParams>,
    mut palette_storage: ResMut<crate::rendering::metaballs::palette::ClusterPaletteStorage>,
    mut dbg_timer: ResMut<MetaballsGroupDebugTimer>,
    sdf_atlas: Option<Res<crate::rendering::sdf_atlas::SdfAtlas>>,
) {
    if !toggle.0 { return; }
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };

    mat.data.v0.w = params.iso;
    mat.data.v1.x = params.normal_z_scale;
    mat.data.v1.y = (fg.current() as u32) as f32;
    mat.data.v1.z = (bg.current() as u32) as f32;
    mat.data.v2.z = time.elapsed_secs();
    mat.data.v2.w = params.radius_multiplier.max(0.0001);
    let iso = params.iso.clamp(1e-4, 0.9999);
    let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt();
    mat.data.v0.z = 1.0 / k;

    // Live noise updates from config
    {
        let noise_cfg = &cfg.noise;
        mat.noise.base_scale = noise_cfg.base_scale; mat.noise.warp_amp = noise_cfg.warp_amp; mat.noise.warp_freq = noise_cfg.warp_freq; mat.noise.speed_x = noise_cfg.speed_x; mat.noise.speed_y = noise_cfg.speed_y; mat.noise.gain = noise_cfg.gain; mat.noise.lacunarity = noise_cfg.lacunarity; mat.noise.contrast_pow = noise_cfg.contrast_pow; mat.noise.octaves = noise_cfg.octaves; mat.noise.ridged = if noise_cfg.ridged { 1 } else { 0 };
        let sn = &cfg.surface_noise; mat.surface_noise.amp = sn.amp.clamp(0.0, 0.5); mat.surface_noise.base_scale = if sn.base_scale > 0.0 { sn.base_scale } else { 0.008 }; mat.surface_noise.speed_x = sn.speed_x; mat.surface_noise.speed_y = sn.speed_y; mat.surface_noise.warp_amp = sn.warp_amp; mat.surface_noise.warp_freq = sn.warp_freq; mat.surface_noise.gain = sn.gain; mat.surface_noise.lacunarity = sn.lacunarity; mat.surface_noise.contrast_pow = sn.contrast_pow; mat.surface_noise.octaves = sn.octaves.min(6); mat.surface_noise.ridged = if sn.ridged { 1 } else { 0 }; mat.surface_noise.mode = sn.mode.min(1); mat.surface_noise.enabled = if sn.enabled { 1 } else { 0 };
    }

    use crate::rendering::palette::palette::color_for_index;
    let mut color_to_group: HashMap<usize, u32> = HashMap::new();
    let mut palette_colors: Vec<[f32; 4]> = Vec::new();
    let mut balls_cpu: Vec<GpuBall> = Vec::with_capacity(q_balls.iter().len().min(MAX_BALLS));
    let mut debug_rows: Vec<String> = Vec::new();
    for (e, tf, r, color_idx, shape_idx_opt) in q_balls.iter() {
        if balls_cpu.len() >= MAX_BALLS { break; }
        let gid = *color_to_group.entry(color_idx.0).or_insert_with(|| {
            let new_gid = palette_colors.len() as u32; let c = color_for_index(color_idx.0).to_srgba(); palette_colors.push([c.red, c.green, c.blue, 1.0]); new_gid
        });
        let pos = tf.translation.truncate();
        let shape_idx: u32 = shape_idx_opt.map(|s| s.0 as u32).unwrap_or(0);
        let packed_gid = ((shape_idx & 0xFFFF) << 16) | (gid & 0xFFFF);
        let angle = tf.rotation.to_euler(EulerRot::XYZ).2; let (s, c) = angle.sin_cos();
        balls_cpu.push(GpuBall::new(pos, r.0, packed_gid, c, s));
        if debug_rows.len() < 8 { debug_rows.push(format!("e={:?} color={} gid={} shape={} packed=0x{:08X}", e, color_idx.0, gid, shape_idx, packed_gid)); }
    }
    let group_count = palette_colors.len() as u32;
    mat.data.v0.x = balls_cpu.len() as f32;
    mat.data.v3.w = balls_cpu.len() as f32;

    if group_count > 0 {
        let cpu_palette = crate::rendering::metaballs::palette::ClusterPaletteCpu { colors: palette_colors.clone(), ids: Vec::new(), color_indices: Vec::new() };
        ensure_palette_capacity_wrapper(&mut palette_storage, group_count, &mut buffers, &cpu_palette);
        if let Some(h) = &palette_storage.handle { mat.cluster_palette = h.clone(); }
        mat.data.v0.y = group_count as f32;
    } else { mat.data.v0.y = 0.0; }

    if balls_cpu.is_empty() { balls_cpu.push(GpuBall::default()); }
    let new_buf = ShaderStorageBuffer::from(balls_cpu.as_slice());
    if buffers.get(&mat.balls).is_some() { if let Some(b) = buffers.get_mut(&mat.balls) { *b = new_buf; } } else { mat.balls = buffers.add(new_buf); }

    let fg_mode = fg.current();
    let needs_gradient = matches!(fg_mode, MetaballForegroundMode::Bevel | MetaballForegroundMode::Metadata);
    mat.data.v4.x = if cfg!(feature = "metaballs_early_exit") { 1.0 } else { 0.0 };
    mat.data.v4.y = if needs_gradient { 1.0 } else { 0.0 };
    mat.data.v4.z = if cfg!(feature = "metaballs_metadata_v2") { 1.0 } else { 0.0 };

    if let Some(atlas) = sdf_atlas.as_ref() {
        if atlas.enabled && cfg.sdf_shapes.enabled && !cfg.sdf_shapes.force_fallback {
            mat.data.v5.x = 1.0;
            let feather_norm = if atlas.tile_size > 0 { (atlas.distance_range / atlas.tile_size as f32).clamp(0.0, 0.5) } else { 0.0 };
            mat.data.v5.y = feather_norm;
            mat.data.v6.x = atlas.atlas_width as f32; mat.data.v6.y = atlas.atlas_height as f32; mat.data.v6.z = atlas.tile_size as f32;
            if mat.sdf_atlas_tex.is_none() { mat.sdf_atlas_tex = Some(atlas.texture.clone()); }
            if let Some(shape_buf) = &atlas.shape_buffer { mat.sdf_shape_meta = shape_buf.clone(); }
        } else { mat.data.v5.x = 0.0; }
    } else { mat.data.v5.x = 0.0; }

    if shadow_params.enabled {
        mat.data.v5.z = 1.0; mat.data.v5.w = shadow_params.intensity.clamp(0.0, 1.0); mat.data.v6.w = shadow_params.offset.max(0.0); mat.data.v5.x = if shadow_params.softness <= 0.0 { 0.0 } else { shadow_params.softness };
    } else { mat.data.v5.z = 0.0; mat.data.v5.w = 0.0; }
    mat.data.v7.x = cfg.metaballs_shadow.direction; mat.data.v7.y = cfg.metaballs_shadow.surface.max(0.05);

    if let Some(ref mut s) = shadow { s.0.clear(); s.0.extend_from_slice(balls_cpu.as_slice()); }

    if dbg_timer.0.tick(time.delta()).just_finished() && !debug_rows.is_empty() { info!(target: "metaballs", "ColorGroups: groups={} sample: {}", group_count, debug_rows.join(" | ")); }
}

fn ensure_palette_capacity_wrapper(
    storage: &mut crate::rendering::metaballs::palette::ClusterPaletteStorage,
    needed: u32,
    buffers: &mut Assets<ShaderStorageBuffer>,
    cpu: &crate::rendering::metaballs::palette::ClusterPaletteCpu,
) {
    use crate::rendering::metaballs::palette::ensure_palette_capacity;
    if needed == 0 { return; }
    ensure_palette_capacity(storage, needed, buffers);
    if let Some(handle) = &storage.handle { if let Some(buf) = buffers.get_mut(handle) {
        let mut data: Vec<[f32; 4]> = vec![[0.0; 4]; storage.capacity as usize];
        for (i, col) in cpu.colors.iter().enumerate().take(storage.length as usize) { data[i] = *col; }
        *buf = ShaderStorageBuffer::from(data.as_slice());
    }}
}

pub fn build_metaball_tiles(
    windows: Query<&Window>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    tiling_cfg: Res<BallTilingConfig>,
    mut meta: ResMut<BallTilesMeta>,
    shadow: Option<Res<BallCpuShadow>>,
) {
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };
    let Some(shadow) = shadow else { return; };
    if shadow.0.is_empty() { return; }

    let (vw, vh) = if let Ok(w) = windows.single() { (w.width(), w.height()) } else { (mat.data.v2.x, mat.data.v2.y) };
    if vw <= 0.0 || vh <= 0.0 { return; }
    let tile_size = tiling_cfg.tile_size.max(8) as f32;
    let tiles_x = ((vw / tile_size).ceil() as u32).max(1); let tiles_y = ((vh / tile_size).ceil() as u32).max(1); let tile_count = (tiles_x * tiles_y) as usize;

    let balls_len = shadow.0.len();
    let mut buckets: Vec<Vec<u32>> = Vec::with_capacity(tile_count); for _ in 0..tile_count { buckets.push(Vec::new()); }

    let origin_x = -vw * 0.5; let origin_y = -vh * 0.5; let radius_scale = mat.data.v0.z; let radius_mult = mat.data.v2.w;
    for (i, b) in shadow.0.iter().enumerate() {
        let base_r = b.data0.z; let center3 = b.data0.truncate(); let center = Vec2::new(center3.x, center3.y); if base_r <= 0.0 { continue; }
        let scaled_r = base_r * radius_scale * radius_mult; let pad = 1.5_f32; let effective_r = scaled_r + pad;
        let min_x = center.x - effective_r - origin_x; let max_x = center.x + effective_r - origin_x; let min_y = center.y - effective_r - origin_y; let max_y = center.y + effective_r - origin_y;
        let mut tx0 = (min_x / tile_size).floor() as i32; let mut tx1 = (max_x / tile_size).floor() as i32; let mut ty0 = (min_y / tile_size).floor() as i32; let mut ty1 = (max_y / tile_size).floor() as i32;
        tx0 = tx0.clamp(0, tiles_x as i32 - 1); tx1 = tx1.clamp(0, tiles_x as i32 - 1); ty0 = ty0.clamp(0, tiles_y as i32 - 1); ty1 = ty1.clamp(0, tiles_y as i32 - 1);
        for ty in ty0..=ty1 { for tx in tx0..=tx1 { let idx = (ty as u32 * tiles_x + tx as u32) as usize; buckets[idx].push(i as u32); } }
    }

    let mut headers_cpu: Vec<TileHeaderGpu> = Vec::with_capacity(tile_count); headers_cpu.resize(tile_count, TileHeaderGpu::default());
    let mut indices_cpu: Vec<u32> = Vec::with_capacity(shadow.0.len() * 2); let mut running: u32 = 0;
    for (t, bucket) in buckets.iter().enumerate() { let count = bucket.len() as u32; headers_cpu[t] = TileHeaderGpu { offset: running, count, _pad0: 0, _pad1: 0 }; indices_cpu.extend_from_slice(bucket); running += count; }
    if headers_cpu.is_empty() { headers_cpu.push(TileHeaderGpu::default()); }
    if indices_cpu.is_empty() { indices_cpu.push(0); }

    let headers_buf = ShaderStorageBuffer::from(headers_cpu.as_slice()); let indices_buf = ShaderStorageBuffer::from(indices_cpu.as_slice());
    if buffers.get(&mat.tile_headers).is_some() { if let Some(h) = buffers.get_mut(&mat.tile_headers) { *h = headers_buf; } } else { mat.tile_headers = buffers.add(headers_buf); }
    if buffers.get(&mat.tile_ball_indices).is_some() { if let Some(h) = buffers.get_mut(&mat.tile_ball_indices) { *h = indices_buf; } } else { mat.tile_ball_indices = buffers.add(indices_buf); }

    mat.data.v3.x = tiles_x as f32; mat.data.v3.y = tiles_y as f32; mat.data.v3.z = tile_size;
    meta.tiles_x = tiles_x; meta.tiles_y = tiles_y; meta.last_ball_len = balls_len;
}

pub fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_unified: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
) {
    let Ok(window) = windows.single() else { return; };
    if let Ok(handle_comp) = q_unified.single() { if let Some(mat) = unified_mats.get_mut(&handle_comp.0) { if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() { mat.data.v2.x = window.width(); mat.data.v2.y = window.height(); } } }
}

pub fn tweak_metaballs_params(mut params: ResMut<MetaballsParams>, input_map: Option<Res<crate::interaction::inputmap::types::InputMap>>) {
    if let Some(im) = input_map { let mut dirty = false; if im.just_pressed("MetaballIsoDec") { params.iso = (params.iso - 0.05).max(0.2); dirty = true; } if im.just_pressed("MetaballIsoInc") { params.iso = (params.iso + 0.05).min(1.5); dirty = true; } if dirty { info!("Metaballs params updated: iso={:.2}", params.iso); } }
}
