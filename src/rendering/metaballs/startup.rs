//! Startup systems: config application & initial quad/material spawn.
use bevy::prelude::*;
use bevy::sprite::MeshMaterial2d;
use bevy::prelude::Mesh2d;
use bevy::render::storage::ShaderStorageBuffer;

use crate::core::config::GameConfig;
use crate::rendering::metaballs::material::MetaballsUnifiedMaterial;
use crate::rendering::metaballs::resources::*;

pub fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) { toggle.0 = cfg.metaballs_enabled; }

pub fn apply_config_to_params(mut params: ResMut<MetaballsParams>, cfg: Res<GameConfig>) {
    params.iso = cfg.metaballs.iso;
    params.normal_z_scale = cfg.metaballs.normal_z_scale;
    params.radius_multiplier = cfg.metaballs.radius_multiplier.max(0.0001);
}

pub fn apply_shadow_from_config(mut shadow: ResMut<MetaballsShadowParams>, cfg: Res<GameConfig>) {
    let c = &cfg.metaballs_shadow;
    shadow.enabled = c.enabled;
    shadow.intensity = c.intensity.clamp(0.0, 1.0);
    shadow.offset = c.offset.max(0.0);
    shadow.softness = c.softness;
}

pub fn apply_shader_modes_from_config(mut fg: ResMut<MetaballForeground>, mut bg: ResMut<MetaballBackground>, cfg: Res<GameConfig>) {
    fg.idx = cfg.metaballs_shader.fg_mode.min(MetaballForegroundMode::ALL.len() - 1);
    bg.idx = cfg.metaballs_shader.bg_mode.min(MetaballBackgroundMode::ALL.len() - 1);
}

pub fn setup_metaballs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    windows: Query<&Window>,
    cfg: Res<GameConfig>,
) {
    let (w, h) = if let Ok(window) = windows.single() { (window.width(), window.height()) } else { (800.0, 600.0) };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));
    let mut umat = MetaballsUnifiedMaterial::default();
    if buffers.get(&umat.sdf_shape_meta).is_none() { let dummy: [f32; 8] = [0.0; 8]; umat.sdf_shape_meta = buffers.add(ShaderStorageBuffer::from(&dummy)); }
    umat.data.v2.x = w; umat.data.v2.y = h;
    // Noise params
    umat.noise.base_scale = cfg.noise.base_scale; umat.noise.warp_amp = cfg.noise.warp_amp; umat.noise.warp_freq = cfg.noise.warp_freq; umat.noise.speed_x = cfg.noise.speed_x; umat.noise.speed_y = cfg.noise.speed_y; umat.noise.gain = cfg.noise.gain; umat.noise.lacunarity = cfg.noise.lacunarity; umat.noise.contrast_pow = cfg.noise.contrast_pow; umat.noise.octaves = cfg.noise.octaves; umat.noise.ridged = if cfg.noise.ridged { 1 } else { 0 };
    // Surface noise
    let sn = &cfg.surface_noise; umat.surface_noise.amp = sn.amp.clamp(0.0, 0.5); umat.surface_noise.base_scale = if sn.base_scale > 0.0 { sn.base_scale } else { 0.008 }; umat.surface_noise.speed_x = sn.speed_x; umat.surface_noise.speed_y = sn.speed_y; umat.surface_noise.warp_amp = sn.warp_amp; umat.surface_noise.warp_freq = sn.warp_freq; umat.surface_noise.gain = sn.gain; umat.surface_noise.lacunarity = sn.lacunarity; umat.surface_noise.contrast_pow = sn.contrast_pow; umat.surface_noise.octaves = sn.octaves.min(6); umat.surface_noise.ridged = if sn.ridged { 1 } else { 0 }; umat.surface_noise.mode = sn.mode.min(1); umat.surface_noise.enabled = if sn.enabled { 1 } else { 0 };
    let unified_handle = unified_mats.add(umat);
    commands.spawn((Mesh2d::from(mesh_handle), MeshMaterial2d(unified_handle), Transform::from_xyz(0.0, 0.0, 50.0), Visibility::Visible, MetaballsUnifiedQuad));
}

pub fn log_initial_modes(fg: Res<MetaballForeground>, bg: Res<MetaballBackground>) {
    info!(target: "metaballs", "Initial modes: Foreground={:?} Background={:?}", fg.current(), bg.current());
}
