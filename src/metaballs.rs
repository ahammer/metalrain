// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d; // Bevy 0.16: Mesh2d is re-exported via prelude; prior internal path caused privacy error.

use crate::cluster::Clusters;
use crate::components::{Ball, BallRadius};
use crate::config::GameConfig;
use crate::materials::BallMaterialIndex;
use crate::palette::color_for_index; // added

// Limits chosen to keep uniform size reasonable (< 64KB)
pub const MAX_BALLS: usize = 1024; // each uses one Vec4
pub const MAX_CLUSTERS: usize = 256; // color table size

#[derive(Clone, Copy, ShaderType, Debug)]
pub(crate) struct MetaballsUniform {
    // Counts
    ball_count: u32,
    cluster_color_count: u32,
    // Tunable scaling so iso surface roughly matches BallRadius.
    radius_scale: f32, // multiplies each stored ball radius
    _pad1: u32,
    // Window (for vertex pass-through scaling)
    window_size: Vec2,
    iso: f32,            // isosurface threshold
    normal_z_scale: f32, // for pseudo-3D lighting (2D path)
    // Shading params
    metallic: f32,
    roughness: f32,
    env_intensity: f32,
    spec_intensity: f32,
    // Debug view variant selector: 0=Normal shaded metaballs, 1=Heightfield (grayscale field), 2=ColorInfo (cluster index coloring)
    debug_view: u32,
    // 0 = smooth blend, 1 = hard cluster boundary
    color_mode: u32,
    color_blend_exponent: f32,
    // User-configured visual expansion multiplier of physical radii (before iso-derived radius_scale)
    radius_multiplier: f32,
    _pad2: Vec3, // keep alignment (replaces previous Vec2 padding)
    // Per-ball packed data: (x, y, radius, cluster_index as float)
    balls: [Vec4; MAX_BALLS],
    // Cluster colors as linear RGB in xyz, w unused (or could store pre-mult factor)
    cluster_colors: [Vec4; MAX_CLUSTERS],
}

impl Default for MetaballsUniform {
    fn default() -> Self {
        Self {
            ball_count: 0,
            cluster_color_count: 0,
            radius_scale: 1.0,
            _pad1: 0,
            window_size: Vec2::ZERO,
            iso: 0.6,
            normal_z_scale: 1.0,
            metallic: 0.5,
            roughness: 0.5,
            env_intensity: 0.0,
            spec_intensity: 0.5,
            debug_view: 0,
            color_mode: 0,
            color_blend_exponent: 1.0,
            radius_multiplier: 1.0,
            _pad2: Vec3::ZERO,
            balls: [Vec4::ZERO; MAX_BALLS],
            cluster_colors: [Vec4::ZERO; MAX_CLUSTERS],
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct MetaballsMaterial {
    #[uniform(0)]
    data: MetaballsUniform,
}

impl MetaballsMaterial {
    #[cfg(feature = "debug")]
    pub fn set_debug_view(&mut self, view: u32) { self.data.debug_view = view; }
}

impl Material2d for MetaballsMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/metaballs.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "shaders/metaballs.wgsl".into()
    }
}

#[derive(Resource, Default)]
pub struct MetaballsToggle(pub bool);

#[derive(Resource, Debug, Clone)]
pub struct MetaballsParams {
    pub iso: f32,
    pub normal_z_scale: f32,
    pub metallic: f32,
    pub roughness: f32,
    pub env_intensity: f32,
    pub spec_intensity: f32,
    pub hard_cluster_boundaries: bool,
    pub color_blend_exponent: f32,
    pub radius_multiplier: f32,
}
#[derive(Component)]
pub struct MetaballsQuad;

impl Default for MetaballsParams {
    fn default() -> Self {
        Self {
            iso: 0.6,
            normal_z_scale: 1.0,
            metallic: 0.5,
            roughness: 0.5,
            env_intensity: 0.0,
            spec_intensity: 0.5,
            hard_cluster_boundaries: false,
            color_blend_exponent: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

pub struct MetaballsPlugin;

impl Plugin for MetaballsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .add_plugins((Material2dPlugin::<MetaballsMaterial>::default(),))
            .add_systems(Startup, (initialize_toggle_from_config, apply_config_to_params, setup_metaballs))
            .add_systems(
                Update,
                (
                    update_metaballs_material,
                    resize_fullscreen_quad,
                    tweak_metaballs_params,
                ),
            );
    }
}

fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) {
    toggle.0 = cfg.metaballs_enabled;
}

fn apply_config_to_params(mut params: ResMut<MetaballsParams>, cfg: Res<GameConfig>) {
    params.iso = cfg.metaballs.iso;
    params.normal_z_scale = cfg.metaballs.normal_z_scale;
    params.metallic = cfg.metaballs.metallic;
    params.roughness = cfg.metaballs.roughness;
    params.env_intensity = cfg.metaballs.env_intensity;
    params.spec_intensity = cfg.metaballs.spec_intensity;
    params.hard_cluster_boundaries = cfg.metaballs.hard_cluster_boundaries;
    params.color_blend_exponent = cfg.metaballs.color_blend_exponent.max(0.01);
    params.radius_multiplier = cfg.metaballs.radius_multiplier.max(0.0001);
}

// (Removed duplicate private MetaballsQuad definition)

fn setup_metaballs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    windows: Query<&Window>,
) {
    let (w, h) = if let Ok(window) = windows.single() {
        (window.width(), window.height())
    } else {
        (800.0, 600.0)
    };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));

    let mut mat = MetaballsMaterial::default();
    mat.data.window_size = Vec2::new(w, h);
    let material_handle = materials.add(mat);

    // Bevy 0.16 migration note: Replaced deprecated MaterialMesh2dBundle usage with explicit
    // component insertion (Mesh2d + MeshMaterial2d + Transform). Visibility components are
    // auto-inferred by engine defaults for simple cases; add explicitly if specialized control needed.
    commands.spawn((
        Mesh2d::from(mesh_handle),
        MeshMaterial2d(material_handle),
        Transform::from_xyz(0.0, 0.0, 50.0),
    Visibility::Visible,
        MetaballsQuad,
    ));
}

// Tests for this module are omitted; rendering pipeline assets are difficult to validate headless.

fn update_metaballs_material(
    clusters: Res<Clusters>,
    q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    #[cfg(feature = "debug")]
    debug_overrides: Option<Res<crate::debug::DebugVisualOverrides>>,
) {
    if !toggle.0 {
        return;
    }
    let Ok(handle_comp) = q_mat.single() else {
        return;
    };
    let Some(mat) = materials.get_mut(&handle_comp.0) else {
        return;
    };

    // Update params
    mat.data.iso = params.iso;
    mat.data.normal_z_scale = params.normal_z_scale;
    mat.data.metallic = params.metallic.clamp(0.0, 1.0);
    mat.data.roughness = params.roughness.clamp(0.04, 1.0);
    mat.data.env_intensity = params.env_intensity.max(0.0);
    mat.data.spec_intensity = params.spec_intensity.max(0.0);
    mat.data.color_mode = if params.hard_cluster_boundaries { 1 } else { 0 };
    mat.data.color_blend_exponent = params.color_blend_exponent.max(0.01);
    mat.data.radius_multiplier = params.radius_multiplier.max(0.0001);
    // Apply debug view (only when debug feature compiled). Falls back to 0 (Normal).
    #[cfg(feature = "debug")]
    {
        if let Some(overrides) = debug_overrides {
            use crate::debug::MetaballsViewVariant;
            mat.data.debug_view = match overrides.metaballs_view_variant {
                MetaballsViewVariant::Normal => 0,
                MetaballsViewVariant::Heightfield => 1,
                MetaballsViewVariant::ColorInfo => 2,
            } as u32;
        }
    }
    // Derive radius_scale so that field at boundary ~ iso.
    // Kernel f = (1 - (d/R)^2)^3. Want radius_visual = R such that f(d=R) = 0 (already), but iso typically <1.
    // If we want iso to hit at d = R_physical, we need original kernel value at physical radius to equal iso.
    // Solve iso = (1 - (R_phys / (R_encoded))^2)^3 with R_encoded = radius_scale * R_phys.
    // Let k = R_phys / (R_encoded) = 1 / radius_scale. Then iso = (1 - k^2)^3 -> k^2 = 1 - iso^(1/3) -> k = sqrt(1 - iso^(1/3)).
    // radius_scale = 1 / k.
    let iso = params.iso.clamp(1e-4, 0.9999);
    let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt();
    mat.data.radius_scale = 1.0 / k;

    // Build cluster color table (stable order up to MAX_CLUSTERS)
    let mut color_count = 0usize;
    for cl in clusters.0.iter() {
        // clusters already grouped by color index; one entry per cluster color instance (could dedupe by color_index if desired)
        if color_count >= MAX_CLUSTERS {
            break;
        }
        let color = color_for_index(cl.color_index);
        let srgb = color.to_srgba();
        mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0);
        color_count += 1;
    }
    mat.data.cluster_color_count = color_count as u32;

    // Pack per-ball data; assign cluster index based on first matching cluster entry with same color_index (fallback 0)
    let mut ball_count = 0usize;
    for (tf, radius, color_idx) in q_balls.iter() {
        if ball_count >= MAX_BALLS {
            break;
        }
        let pos = tf.translation.truncate();
        // Find a cluster index with matching color (linear search; could optimize via hashmap color->first index)
        let mut cluster_slot = 0u32;
        for (i, cl) in clusters.0.iter().enumerate() {
            if cl.color_index == color_idx.0 {
                cluster_slot = i as u32;
                break;
            }
        }
        mat.data.balls[ball_count] = Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32);
        ball_count += 1;
    }
    mat.data.ball_count = ball_count as u32;
}

fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(handle_comp) = q_mat.single() else {
        return;
    };
    if let Some(mat) = materials.get_mut(&handle_comp.0) {
        if mat.data.window_size.x != window.width() || mat.data.window_size.y != window.height() {
            mat.data.window_size = Vec2::new(window.width(), window.height());
        }
    }
}

fn tweak_metaballs_params(mut params: ResMut<MetaballsParams>, keys: Res<ButtonInput<KeyCode>>) {
    let mut dirty = false;
    if keys.just_pressed(KeyCode::BracketLeft) {
        params.iso = (params.iso - 0.05).max(0.2);
        dirty = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        params.iso = (params.iso + 0.05).min(1.5);
        dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        // toggle metallic extremes
        params.metallic = if params.metallic > 0.5 { 0.0 } else { 0.85 };
        dirty = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        // decrease roughness
        params.roughness = (params.roughness - 0.05).max(0.04);
        dirty = true;
    }
    if keys.just_pressed(KeyCode::Equal) {
        // increase roughness
        params.roughness = (params.roughness + 0.05).min(1.0);
        dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        // env intensity toggle
        params.env_intensity = if params.env_intensity > 0.5 { 0.2 } else { 1.0 };
        dirty = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        // spec intensity toggle
        params.spec_intensity = if params.spec_intensity > 0.5 {
            0.0
        } else {
            1.0
        };
        dirty = true;
    }
    if dirty {
        info!(
            "Metaballs params updated: iso={:.2} metal={:.2} rough={:.2} env={:.2} spec={:.2}",
            params.iso,
            params.metallic,
            params.roughness,
            params.env_intensity,
            params.spec_intensity
        );
    }
}
