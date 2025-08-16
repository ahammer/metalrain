// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle};

use crate::cluster::Clusters;
use crate::config::GameConfig;
use crate::components::{Ball, BallRadius};
use crate::materials::BallMaterialIndex;

// Limits chosen to keep uniform size reasonable (< 64KB)
pub const MAX_BALLS: usize = 1024; // each uses one Vec4
pub const MAX_CLUSTERS: usize = 256; // color table size

#[derive(Clone, Copy, ShaderType, Debug)]
struct MetaballsUniform {
    // Counts
    ball_count: u32,
    cluster_color_count: u32,
    // Tunable scaling so iso surface roughly matches BallRadius.
    radius_scale: f32, // multiplies each stored ball radius
    _pad1: u32,
    // Window (for vertex pass-through scaling)
    window_size: Vec2,
    iso: f32,              // isosurface threshold
    normal_z_scale: f32,   // for pseudo-3D lighting (2D path)
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
            balls: [Vec4::ZERO; MAX_BALLS],
            cluster_colors: [Vec4::ZERO; MAX_CLUSTERS],
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MetaballsMaterial {
    #[uniform(0)]
    data: MetaballsUniform,
}

impl Default for MetaballsMaterial { fn default() -> Self { Self { data: Default::default() } } }

impl Material2d for MetaballsMaterial {
    fn fragment_shader() -> ShaderRef { "shaders/metaballs.wgsl".into() }
    fn vertex_shader() -> ShaderRef { "shaders/metaballs.wgsl".into() }
}

#[derive(Resource, Default)]
pub struct MetaballsToggle(pub bool);

#[derive(Resource, Debug, Clone)]
pub struct MetaballsParams {
    pub iso: f32,
    pub normal_z_scale: f32,
}

impl Default for MetaballsParams {
    fn default() -> Self { Self { iso: 0.6, normal_z_scale: 1.0 } }
}

pub struct MetaballsPlugin;

impl Plugin for MetaballsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .add_plugins((Material2dPlugin::<MetaballsMaterial>::default(),))
            .add_systems(Startup, (initialize_toggle_from_config, setup_metaballs))
            .add_systems(Update, (update_metaballs_material, resize_fullscreen_quad));
    }
}

fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) {
    toggle.0 = cfg.metaballs_enabled;
}

#[derive(Component)]
struct MetaballsQuad;

fn setup_metaballs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    windows: Query<&Window>,
) {
    let (w, h) = if let Ok(window) = windows.get_single() { (window.width(), window.height()) } else { (800.0, 600.0) };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));

    let mut mat = MetaballsMaterial::default();
    mat.data.window_size = Vec2::new(w, h);
    let material_handle = materials.add(mat);

    commands.spawn((
        MaterialMesh2dBundle::<MetaballsMaterial> {
            mesh: mesh_handle.into(),
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.0, 50.0),
            ..default()
        },
        MetaballsQuad,
    ));
}

// Tests for this module are omitted; rendering pipeline assets are difficult to validate headless.

fn update_metaballs_material(
    clusters: Res<Clusters>,
    q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    q_mat: Query<&Handle<MetaballsMaterial>, With<MetaballsQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
) {
    if !toggle.0 { return; }
    let Ok(handle) = q_mat.get_single() else { return; };
    let Some(mat) = materials.get_mut(handle) else { return; };

    // Update params
    mat.data.iso = params.iso;
    mat.data.normal_z_scale = params.normal_z_scale;
    // Derive radius_scale so that field at boundary ~ iso.
    // Kernel f = (1 - (d/R)^2)^3. Want radius_visual = R such that f(d=R) = 0 (already), but iso typically <1.
    // If we want iso to hit at d = R_physical, we need original kernel value at physical radius to equal iso.
    // Solve iso = (1 - (R_phys / (R_encoded))^2)^3 with R_encoded = radius_scale * R_phys.
    // Let k = R_phys / (R_encoded) = 1 / radius_scale. Then iso = (1 - k^2)^3 -> k^2 = 1 - iso^(1/3) -> k = sqrt(1 - iso^(1/3)).
    // radius_scale = 1 / k.
    let iso = params.iso.max(1e-4).min(0.9999);
    let k = (1.0 - iso.powf(1.0/3.0)).max(1e-4).sqrt();
    mat.data.radius_scale = 1.0 / k;

    // Build cluster color table (stable order up to MAX_CLUSTERS)
    let mut color_count = 0usize;
    for cl in clusters.0.iter() { // clusters already grouped by color index; one entry per cluster color instance (could dedupe by color_index if desired)
        if color_count >= MAX_CLUSTERS { break; }
        let color = match cl.color_index % 6 {
            0 => Color::srgb(0.90, 0.20, 0.25),
            1 => Color::srgb(0.20, 0.55, 0.90),
            2 => Color::srgb(0.95, 0.75, 0.15),
            3 => Color::srgb(0.20, 0.80, 0.45),
            4 => Color::srgb(0.65, 0.45, 0.95),
            _ => Color::srgb(0.95, 0.50, 0.15),
        };
        let srgb = color.to_srgba();
        mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0);
        color_count += 1;
    }
    mat.data.cluster_color_count = color_count as u32;

    // Pack per-ball data; assign cluster index based on first matching cluster entry with same color_index (fallback 0)
    let mut ball_count = 0usize;
    for (tf, radius, color_idx) in q_balls.iter() {
        if ball_count >= MAX_BALLS { break; }
        let pos = tf.translation.truncate();
        // Find a cluster index with matching color (linear search; could optimize via hashmap color->first index)
        let mut cluster_slot = 0u32;
        for (i, cl) in clusters.0.iter().enumerate() { if cl.color_index == color_idx.0 { cluster_slot = i as u32; break; } }
    mat.data.balls[ball_count] = Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32);
        ball_count += 1;
    }
    mat.data.ball_count = ball_count as u32;
}

fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_mat: Query<&Handle<MetaballsMaterial>, With<MetaballsQuad>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
) {
    let Ok(window) = windows.get_single() else { return; };
    let Ok(handle) = q_mat.get_single() else { return; };
    if let Some(mat) = materials.get_mut(handle) {
        if mat.data.window_size.x != window.width() || mat.data.window_size.y != window.height() {
            mat.data.window_size = Vec2::new(window.width(), window.height());
        }
    }
}
