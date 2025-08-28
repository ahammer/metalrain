#![allow(clippy::type_complexity, clippy::collapsible_match)]
use bevy::input::keyboard::KeyCode;
use bevy::input::ButtonInput;
use bevy::prelude::Mesh2d;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
static METABALLS_UNIFIED_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static METABALLS_UNIFIED_DEBUG_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

use crate::core::components::{Ball, BallRadius, BallState};
use crate::core::config::GameConfig;
use crate::gameplay::state::{BallStateUpdateSet, OverflowLogged};
use crate::physics::clustering::cluster::Clusters;
use crate::rendering::materials::materials::BallMaterialIndex;
use crate::rendering::palette::palette::{color_for_index, secondary_color_for_index};

// =====================================================================================
// Uniform layout (BINARY LAYOUT UNCHANGED – ONLY SEMANTICS UPDATED)
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// balls / cluster_colors arrays unchanged
// =====================================================================================

pub const MAX_BALLS: usize = 1024;
pub const MAX_CLUSTERS: usize = 256;

#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub(crate) struct MetaballsUniform {
    v0: Vec4,
    v1: Vec4,
    v2: Vec4,
    balls: [Vec4; MAX_BALLS],
    cluster_colors: [Vec4; MAX_CLUSTERS],
}

impl Default for MetaballsUniform {
    fn default() -> Self {
        Self {
            // radius_scale (v0.z) will be derived from iso each update
            v0: Vec4::new(0.0, 0.0, 1.0, 0.6),
            // v1.y / v1.z will be written by mode update
            v1: Vec4::new(1.0, 0.0, 0.0, 0.0),
            // v2.w radius_multiplier relocated here (previously v1.z)
            v2: Vec4::new(0.0, 0.0, 0.0, 1.0),
            balls: [Vec4::ZERO; MAX_BALLS],
            cluster_colors: [Vec4::ZERO; MAX_CLUSTERS],
        }
    }
}

// NEW: Mirror of WGSL NoiseParams (group(2) binding(1))
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub struct NoiseParamsUniform {
    pub base_scale: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}
impl Default for NoiseParamsUniform {
    fn default() -> Self {
        Self {
            base_scale: 0.004,
            warp_amp: 0.6,
            warp_freq: 0.5,
            speed_x: 0.03,
            speed_y: 0.02,
            gain: 0.5,
            lacunarity: 2.0,
            contrast_pow: 1.25,
            octaves: 5,
            ridged: 0,
            _pad0: 0,
            _pad1: 0,
        }
    }
}

// NEW: SurfaceNoiseParamsUniform (group(2) binding(2))
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default)]
// NOTE: Must remain exactly 64 bytes (16 * 4-byte scalars) so uniform buffer size is multiple of 16 for downlevel/WebGL.
pub struct SurfaceNoiseParamsUniform {
    pub amp: f32,
    pub base_scale: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: u32,
    pub mode: u32,
    pub enabled: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32, // NEW padding: total scalars = 16 -> 64 bytes (must stay multiple of 16 for downlevel/WebGL)
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct MetaballsUnifiedMaterial {
    #[uniform(0)]
    data: MetaballsUniform,
    #[uniform(1)]
    noise: NoiseParamsUniform, // background noise (binding 1)
    #[uniform(2)]
    surface_noise: SurfaceNoiseParamsUniform, // NEW surface noise (binding 2)
}

impl MetaballsUnifiedMaterial {
    #[cfg(feature = "debug")]
    pub fn set_debug_view(&mut self, view: u32) {
        self.data.v1.w = view as f32;
    }

    /// Debug helper: returns (ball_count, slot_count) for tests / diagnostics.
    pub fn debug_counts(&self) -> (u32, u32) {
        (self.data.v0.x as u32, self.data.v0.y as u32)
    }
}

impl Material2d for MetaballsUnifiedMaterial {
    fn fragment_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            ShaderRef::Handle(METABALLS_UNIFIED_SHADER_HANDLE.get().unwrap().clone())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            "shaders/metaballs_unified.wgsl".into()
        }
    }

    fn vertex_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(handle) = METABALLS_UNIFIED_DEBUG_SHADER_HANDLE.get().cloned() {
                return ShaderRef::Handle(handle);
            }
            return ShaderRef::Path("shaders/metaballs_unified_debug.wgsl".into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            "shaders/metaballs_unified_debug.wgsl".into()
        }
    }
}

// =====================================================================================
// NEW DUAL AXIS MODE RESOURCES (Foreground / Background)
// =====================================================================================

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode {
    ClassicBlend,
    Bevel,
    OutlineGlow,
}
impl MetaballForegroundMode {
    pub const ALL: [Self; 3] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow];
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballBackgroundMode {
    SolidGray,
    ProceduralNoise,
    VerticalGradient,
}
impl MetaballBackgroundMode {
    pub const ALL: [Self; 3] = [
        Self::SolidGray,
        Self::ProceduralNoise,
        Self::VerticalGradient,
    ];
}

#[derive(Resource, Debug, Default)]
pub struct MetaballForeground {
    pub idx: usize,
}
impl MetaballForeground {
    pub fn current(&self) -> MetaballForegroundMode {
        MetaballForegroundMode::ALL[self.idx % MetaballForegroundMode::ALL.len()]
    }
}

#[derive(Resource, Debug, Default)]
pub struct MetaballBackground {
    pub idx: usize,
}
impl MetaballBackground {
    pub fn current(&self) -> MetaballBackgroundMode {
        MetaballBackgroundMode::ALL[self.idx % MetaballBackgroundMode::ALL.len()]
    }
}

// =====================================================================================

#[derive(Component)]
pub struct MetaballsUnifiedQuad;

#[derive(Resource, Default)]
pub struct MetaballsToggle(pub bool);

#[derive(Resource, Debug, Clone)]
pub struct MetaballsParams {
    pub iso: f32,
    pub normal_z_scale: f32,
    pub radius_multiplier: f32,
}
impl Default for MetaballsParams {
    fn default() -> Self {
        Self {
            iso: 0.6,
            normal_z_scale: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

pub struct MetaballsPlugin;

impl Plugin for MetaballsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            use bevy::asset::Assets;
            use bevy::render::render_resource::Shader;
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let unified_handle = shaders.add(Shader::from_wgsl(
                include_str!("../../../assets/shaders/metaballs_unified.wgsl"),
                "metaballs_unified_embedded.wgsl",
            ));
            let debug_handle = shaders.add(Shader::from_wgsl(
                include_str!("../../../assets/shaders/metaballs_unified_debug.wgsl"),
                "metaballs_unified_debug_embedded.wgsl",
            ));
            METABALLS_UNIFIED_SHADER_HANDLE.get_or_init(|| unified_handle.clone());
            METABALLS_UNIFIED_DEBUG_SHADER_HANDLE.get_or_init(|| debug_handle.clone());
        }

        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .init_resource::<MetaballForeground>()
            .init_resource::<MetaballBackground>()
            .add_plugins((Material2dPlugin::<MetaballsUnifiedMaterial>::default(),))
            .add_systems(
                Startup,
                (
                    initialize_toggle_from_config,
                    apply_config_to_params,
                    apply_shader_modes_from_config,
                    setup_metaballs,
                    log_initial_modes,
                ),
            )
            .add_systems(
                Update,
                (
                    update_metaballs_unified_material
                        .after(BallStateUpdateSet), // ensure BallState updated
                    cycle_foreground_mode,
                    cycle_background_mode,
                    resize_fullscreen_quad,
                    tweak_metaballs_params,
                ),
            );
    }
}

// =====================================================================================
// Startup / Config
// =====================================================================================

fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) {
    toggle.0 = cfg.metaballs_enabled;
}

fn apply_config_to_params(mut params: ResMut<MetaballsParams>, cfg: Res<GameConfig>) {
    params.iso = cfg.metaballs.iso;
    params.normal_z_scale = cfg.metaballs.normal_z_scale;
    params.radius_multiplier = cfg.metaballs.radius_multiplier.max(0.0001);
}

fn apply_shader_modes_from_config(
    mut fg: ResMut<MetaballForeground>,
    mut bg: ResMut<MetaballBackground>,
    cfg: Res<GameConfig>,
) {
    fg.idx = cfg
        .metaballs_shader
        .fg_mode
        .min(MetaballForegroundMode::ALL.len() - 1);
    bg.idx = cfg
        .metaballs_shader
        .bg_mode
        .min(MetaballBackgroundMode::ALL.len() - 1);
}

fn setup_metaballs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
    windows: Query<&Window>,
    cfg: Res<GameConfig>,
) {
    let (w, h) = if let Ok(window) = windows.single() {
        (window.width(), window.height())
    } else {
        (800.0, 600.0)
    };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));

    let mut umat = MetaballsUnifiedMaterial::default();
    umat.data.v2.x = w;
    umat.data.v2.y = h;
    // Initialize noise uniform from config (will also be updated per-frame)
    umat.noise.base_scale = cfg.noise.base_scale;
    umat.noise.warp_amp = cfg.noise.warp_amp;
    umat.noise.warp_freq = cfg.noise.warp_freq;
    umat.noise.speed_x = cfg.noise.speed_x;
    umat.noise.speed_y = cfg.noise.speed_y;
    umat.noise.gain = cfg.noise.gain;
    umat.noise.lacunarity = cfg.noise.lacunarity;
    umat.noise.contrast_pow = cfg.noise.contrast_pow;
    umat.noise.octaves = cfg.noise.octaves;
    umat.noise.ridged = if cfg.noise.ridged { 1 } else { 0 };

    // Initialize surface noise uniform from config.surface_noise
    let sn = &cfg.surface_noise;
    umat.surface_noise.amp = sn.amp.clamp(0.0, 0.5);
    umat.surface_noise.base_scale = if sn.base_scale > 0.0 { sn.base_scale } else { 0.008 };
    umat.surface_noise.speed_x = sn.speed_x;
    umat.surface_noise.speed_y = sn.speed_y;
    umat.surface_noise.warp_amp = sn.warp_amp;
    umat.surface_noise.warp_freq = sn.warp_freq;
    umat.surface_noise.gain = sn.gain;
    umat.surface_noise.lacunarity = sn.lacunarity;
    umat.surface_noise.contrast_pow = sn.contrast_pow;
    umat.surface_noise.octaves = sn.octaves.min(6);
    umat.surface_noise.ridged = if sn.ridged { 1 } else { 0 };
    umat.surface_noise.mode = sn.mode.min(1);
    umat.surface_noise.enabled = if sn.enabled { 1 } else { 0 };

    let unified_handle = unified_mats.add(umat);

    commands.spawn((
        Mesh2d::from(mesh_handle),
        MeshMaterial2d(unified_handle),
        Transform::from_xyz(0.0, 0.0, 50.0),
        Visibility::Visible,
        MetaballsUnifiedQuad,
    ));
}

fn log_initial_modes(fg: Res<MetaballForeground>, bg: Res<MetaballBackground>) {
    info!(
        target: "metaballs",
        "Initial modes: Foreground={:?} Background={:?}",
        fg.current(),
        bg.current()
    );
}

// =====================================================================================
// Mode Cycling
// =====================================================================================

fn cycle_foreground_mode(mut fg: ResMut<MetaballForeground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::End) {
        fg.idx = (fg.idx + 1) % MetaballForegroundMode::ALL.len();
        info!(
            target: "metaballs",
            "Foreground mode -> {:?}",
            fg.current()
        );
    }
    if keys.just_pressed(KeyCode::Home) {
        fg.idx =
            (fg.idx + MetaballForegroundMode::ALL.len() - 1) % MetaballForegroundMode::ALL.len();
        info!(
            target: "metaballs",
            "Foreground mode -> {:?}",
            fg.current()
        );
    }
}

fn cycle_background_mode(mut bg: ResMut<MetaballBackground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::PageUp) {
        bg.idx = (bg.idx + 1) % MetaballBackgroundMode::ALL.len();
        info!(
            target: "metaballs",
            "Background mode -> {:?}",
            bg.current()
        );
    }
    if keys.just_pressed(KeyCode::PageDown) {
        bg.idx =
            (bg.idx + MetaballBackgroundMode::ALL.len() - 1) % MetaballBackgroundMode::ALL.len();
        info!(
            target: "metaballs",
            "Background mode -> {:?}",
            bg.current()
        );
    }
}

// =====================================================================================
// Uniform Update (ENABLED vs DISABLED clusters + tweened colors)
// =====================================================================================

#[allow(clippy::too_many_arguments)]
fn update_metaballs_unified_material(
    time: Res<Time>,
    fg: Res<MetaballForeground>,
    bg: Res<MetaballBackground>,
    clusters: Res<Clusters>,
    q_balls: Query<(Entity, &Transform, &BallRadius, Option<&BallState>, &BallMaterialIndex), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    cfg: Res<GameConfig>,
    overflow_logged: Option<ResMut<OverflowLogged>>,
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

    // PACK static fields
    mat.data.v0.w = params.iso;
    mat.data.v1.x = params.normal_z_scale;
    mat.data.v1.y = (fg.current() as u32) as f32;
    mat.data.v1.z = (bg.current() as u32) as f32;
    mat.data.v2.z = time.elapsed_secs();
    mat.data.v2.w = params.radius_multiplier.max(0.0001);
    let iso = params.iso.clamp(1e-4, 0.9999);
    let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt();
    mat.data.v0.z = 1.0 / k;

    // Noise uniforms live update
    {
        let noise_cfg = &cfg.noise;
        mat.noise.base_scale = noise_cfg.base_scale;
        mat.noise.warp_amp = noise_cfg.warp_amp;
        mat.noise.warp_freq = noise_cfg.warp_freq;
        mat.noise.speed_x = noise_cfg.speed_x;
        mat.noise.speed_y = noise_cfg.speed_y;
        mat.noise.gain = noise_cfg.gain;
        mat.noise.lacunarity = noise_cfg.lacunarity;
        mat.noise.contrast_pow = noise_cfg.contrast_pow;
        mat.noise.octaves = noise_cfg.octaves;
        mat.noise.ridged = if noise_cfg.ridged { 1 } else { 0 };

        let sn = &cfg.surface_noise;
        mat.surface_noise.amp = sn.amp.clamp(0.0, 0.5);
        mat.surface_noise.base_scale = if sn.base_scale > 0.0 { sn.base_scale } else { 0.008 };
        mat.surface_noise.speed_x = sn.speed_x;
        mat.surface_noise.speed_y = sn.speed_y;
        mat.surface_noise.warp_amp = sn.warp_amp;
        mat.surface_noise.warp_freq = sn.warp_freq;
        mat.surface_noise.gain = sn.gain;
        mat.surface_noise.lacunarity = sn.lacunarity;
        mat.surface_noise.contrast_pow = sn.contrast_pow;
        mat.surface_noise.octaves = sn.octaves.min(6);
        mat.surface_noise.ridged = if sn.ridged { 1 } else { 0 };
        mat.surface_noise.mode = sn.mode.min(1);
        mat.surface_noise.enabled = if sn.enabled { 1 } else { 0 };
    }

    // Clear previous frame slots
    for c in mat.data.cluster_colors.iter_mut() {
        *c = Vec4::ZERO;
    }

    let now = time.elapsed_secs();
    let tween_dur = cfg.ball_state.tween_duration.max(0.01);

    // Build quick lookup for ball components
    use std::collections::HashMap;
    let mut ball_tf: HashMap<Entity, (Vec2, f32, Option<BallState>, usize)> =
        HashMap::with_capacity(q_balls.iter().len());
    for (e, tf, r, st, color_idx) in q_balls.iter() {
        ball_tf.insert(
            e,
            (tf.translation.truncate(), r.0, st.copied(), color_idx.0),
        );
    }

    // Entity -> slot mapping
    let mut slot_map: HashMap<Entity, usize> = HashMap::with_capacity(ball_tf.len());

    let mut slot_count = 0usize;
    let mut overflow = false;

    // Enabled clusters first: share slot
    for cl in clusters.0.iter() {
        if slot_count >= MAX_CLUSTERS {
            overflow = true;
            break;
        }
        // Determine cluster state (all balls classified the same; use first that has state)
        let mut cluster_enabled = true;
        for &e in &cl.entities {
            if let Some((_p, _r, st_opt, _ci)) = ball_tf.get(&e) {
                if let Some(st) = st_opt {
                    cluster_enabled = st.enabled;
                    break;
                }
            }
        }
        if cluster_enabled {
            let slot = slot_count;
            slot_count += 1;
            // Representative state for tween
            let rep_state = {
                let e = cl.entities[0];
                let (_, _, st_opt, _ci) = ball_tf.get(&e).unwrap();
                *st_opt
            };
            let enabled_color = color_for_index(cl.color_index);
            let disabled_color = secondary_color_for_index(cl.color_index);
            let color_vec = compute_tween_color(enabled_color, disabled_color, rep_state, now, tween_dur);
            mat.data.cluster_colors[slot] = color_vec;
            for &e in &cl.entities {
                slot_map.insert(e, slot);
            }
        }
    }

    // Disabled clusters: unique slot per ball (prevent merging)
    if !overflow {
        for cl in clusters.0.iter() {
            // Determine cluster enabled state again
            let mut cluster_enabled = true;
            for &e in &cl.entities {
                if let Some((_p, _r, st_opt, _ci)) = ball_tf.get(&e) {
                    if let Some(st) = st_opt {
                        cluster_enabled = st.enabled;
                        break;
                    }
                }
            }
            if cluster_enabled {
                continue;
            }
            for &e in &cl.entities {
                if slot_count >= MAX_CLUSTERS {
                    overflow = true;
                    break;
                }
                let (_p, _r, st_opt, ci) = ball_tf.get(&e).unwrap();
                let enabled_color = color_for_index(*ci);
                let disabled_color = secondary_color_for_index(*ci);
                let color_vec = compute_tween_color(enabled_color, disabled_color, *st_opt, now, tween_dur);
                let slot = slot_count;
                slot_count += 1;
                mat.data.cluster_colors[slot] = color_vec;
                slot_map.insert(e, slot);
            }
            if overflow {
                break;
            }
        }
    }

    // Overflow fallback: group disabled balls by palette index (merging may reappear)
    if overflow {
        if let Some(mut flag) = overflow_logged {
            if !flag.0 {
                info!(
                    target: "metaballs",
                    "MAX_CLUSTERS overflow ({}). Falling back to grouping disabled balls by base color.",
                    MAX_CLUSTERS
                );
                flag.0 = true;
            }
        }
        // Assign any unassigned balls a slot based on color index modulo MAX_CLUSTERS
        for (e, (_p, _r, st_opt, ci)) in ball_tf.iter() {
            if slot_map.contains_key(e) {
                continue;
            }
            let slot = *ci % MAX_CLUSTERS;
            if mat.data.cluster_colors[slot] == Vec4::ZERO {
                // pick disabled variant color (no tween per-ball due to grouping)
                let enabled_color = color_for_index(*ci);
                let disabled_color = secondary_color_for_index(*ci);
                let color_vec = compute_tween_color(enabled_color, disabled_color, *st_opt, now, tween_dur);
                mat.data.cluster_colors[slot] = color_vec;
            }
            slot_map.insert(*e, slot);
        }
        slot_count = MAX_CLUSTERS;
    }

    mat.data.v0.y = slot_count as f32;

    // Balls array
    let mut ball_index = 0usize;
    for (e, (pos, radius, _st, _ci)) in ball_tf.iter() {
        if ball_index >= MAX_BALLS {
            break;
        }
        let slot = slot_map.get(e).copied().unwrap_or(0) as f32;
        mat.data.balls[ball_index] = Vec4::new(pos.x, pos.y, *radius, slot);
        ball_index += 1;
    }
    mat.data.v0.x = ball_index as f32;
}

// Color tween helper: returns Vec4 (srgb components) for uniform
fn compute_tween_color(
    enabled_col: Color,
    disabled_col: Color,
    state: Option<BallState>,
    now: f32,
    tween_dur: f32,
) -> Vec4 {
    let st = state.unwrap_or(BallState { enabled: true, last_change: now });
    let t = ((now - st.last_change) / tween_dur).clamp(0.0, 1.0);
    let (from, to) = if st.enabled {
        (disabled_col, enabled_col)
    } else {
        (enabled_col, disabled_col)
    };
    let lerped = lerp_color(from, to, t);
    let srgb = lerped.to_srgba();
    Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0)
}

// Linear color lerp
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let la = a.to_linear();
    let lb = b.to_linear();
    let r = la.red + (lb.red - la.red) * t;
    let g = la.green + (lb.green - la.green) * t;
    let bch = la.blue + (lb.blue - la.blue) * t;
    let a_out = la.alpha + (lb.alpha - la.alpha) * t;
    Color::linear_rgba(r, g, bch, a_out)
}

// =====================================================================================
// Resize handling
// =====================================================================================

fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_unified: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    if let Ok(handle_comp) = q_unified.single() {
        if let Some(mat) = unified_mats.get_mut(&handle_comp.0) {
            if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() {
                mat.data.v2.x = window.width();
                mat.data.v2.y = window.height();
            }
        }
    }
}

// =====================================================================================
// Param tweaks (iso)
// =====================================================================================

fn tweak_metaballs_params(
    mut params: ResMut<MetaballsParams>,
    input_map: Option<Res<crate::interaction::inputmap::types::InputMap>>,
) {
    if let Some(im) = input_map {
        let mut dirty = false;
        if im.just_pressed("MetaballIsoDec") {
            params.iso = (params.iso - 0.05).max(0.2);
            dirty = true;
        }
        if im.just_pressed("MetaballIsoInc") {
            params.iso = (params.iso + 0.05).min(1.5);
            dirty = true;
        }
        if dirty {
            info!("Metaballs params updated: iso={:.2}", params.iso);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::palette::palette::{BASE_COLORS, secondary_color_for_index};
    use bevy::ecs::system::SystemState;
    use bevy::asset::Assets;

    #[test]
    fn lerp_color_midpoint() {
        let a = Color::linear_rgba(0.0, 0.0, 0.0, 1.0);
        let b = Color::linear_rgba(1.0, 1.0, 1.0, 1.0);
        let m = super::lerp_color(a, b, 0.5);
        let l = m.to_linear();
        assert!((l.red - 0.5).abs() < 1e-6);
        assert!((l.green - 0.5).abs() < 1e-6);
        assert!((l.blue - 0.5).abs() < 1e-6);
    }

    #[test]
    fn secondary_palette_mapping_cycles() {
        for i in 0..16 {
            let s = secondary_color_for_index(i);
            let b = BASE_COLORS[i % BASE_COLORS.len()];
            // Just ensure they differ enough in at least one channel
            let sd = s.to_srgba();
            let bd = b.to_srgba();
            let diff = (sd.red - bd.red).abs() + (sd.green - bd.green).abs() + (sd.blue - bd.blue).abs();
            assert!(diff > 0.05, "secondary color too similar to base at index {i}");
        }
    }

    // Integration-style test of slot allocation: disabled cluster -> per-ball slots, then enabled -> shared slot.
    #[test]
    fn disabled_then_enabled_slot_allocation() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Resources
        app.insert_resource(GameConfig::default());
        app.insert_resource(Clusters::default());

        // Minimal material assets & quad
        app.world_mut().init_resource::<Assets<MetaballsUnifiedMaterial>>();
        let mut materials = app.world_mut().resource_mut::<Assets<MetaballsUnifiedMaterial>>();
        let handle = materials.add(MetaballsUnifiedMaterial::default());
        let _quad = app.world_mut().spawn((MetaballsUnifiedQuad, MeshMaterial2d(handle.clone()))).id();

        // Insert time
        app.insert_resource(Time::<()>::default());
        app.insert_resource(MetaballsToggle(true));
        app.insert_resource(MetaballsParams::default());
        app.insert_resource(MetaballForeground::default());
        app.insert_resource(MetaballBackground::default());
        app.insert_resource(OverflowLogged::default());

        // Spawn 3 balls (below default thresholds -> disabled)
        for i in 0..3 {
            let e = app.world_mut().spawn((
                Ball,
                BallRadius(10.0),
                BallMaterialIndex(0),
                Transform::from_xyz(i as f32 * 25.0, 0.0, 0.0),
                GlobalTransform::default(),
                BallState { enabled: false, last_change: 0.0 },
            )).id();
            // Add to clusters resource (single cluster)
            {
                let mut clusters = app.world_mut().resource_mut::<Clusters>();
                if clusters.0.is_empty() {
                    clusters.0.push(crate::physics::clustering::cluster::Cluster {
                        color_index: 0,
                        entities: vec![e],
                        min: Vec2::ZERO,
                        max: Vec2::ZERO,
                        centroid: Vec2::ZERO,
                        total_area: 0.0,
                    });
                } else {
                    clusters.0[0].entities.push(e);
                }
                // recompute area & bounds crudely
                clusters.0[0].total_area += std::f32::consts::PI * 10.0 * 10.0;
            }
        }

        // Run material update manually via SystemState to supply params
        {
            let mut system_state: SystemState<(
                Res<Time>,
                Res<MetaballForeground>,
                Res<MetaballBackground>,
                Res<Clusters>,
                Query<(Entity, &Transform, &BallRadius, Option<&BallState>, &BallMaterialIndex), With<Ball>>,
                ResMut<Assets<MetaballsUnifiedMaterial>>,
                Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
                Res<MetaballsToggle>,
                Res<MetaballsParams>,
                Res<GameConfig>,
                Option<ResMut<OverflowLogged>>,
            )> = SystemState::new(app.world_mut());

            let world = app.world_mut();
            let params = system_state.get_mut(world);
            update_metaballs_unified_material(
                params.0, params.1, params.2, params.3, params.4, params.5, params.6,
                params.7, params.8, params.9, params.10,
            );
            system_state.apply(world);
        }

        // Assert: 3 slots (one per disabled ball)
        let materials_res = app.world().resource::<Assets<MetaballsUnifiedMaterial>>();
        let mat = materials_res.get(handle.id()).unwrap();
        let (_ball_count, slot_count) = mat.debug_counts();
        assert_eq!(slot_count, 3, "expected unique slot per disabled ball");

        // Enable cluster: flip BallState enabled true and lower thresholds
        {
            let mut cfg = app.world_mut().resource_mut::<GameConfig>();
            cfg.interactions.cluster_pop.min_ball_count = 1;
            cfg.interactions.cluster_pop.min_total_area = 0.0;
        }
        {
            let world = app.world_mut();
            let mut q = world.query::<&mut BallState>();
            for mut st in q.iter_mut(world) {
                st.enabled = true;
                st.last_change = 0.0;
            }
        }

        // Run update again
        {
            let mut system_state: SystemState<(
                Res<Time>,
                Res<MetaballForeground>,
                Res<MetaballBackground>,
                Res<Clusters>,
                Query<(Entity, &Transform, &BallRadius, Option<&BallState>, &BallMaterialIndex), With<Ball>>,
                ResMut<Assets<MetaballsUnifiedMaterial>>,
                Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
                Res<MetaballsToggle>,
                Res<MetaballsParams>,
                Res<GameConfig>,
                Option<ResMut<OverflowLogged>>,
            )> = SystemState::new(app.world_mut());

            let world = app.world_mut();
            let params = system_state.get_mut(world);
            update_metaballs_unified_material(
                params.0, params.1, params.2, params.3, params.4, params.5, params.6,
                params.7, params.8, params.9, params.10,
            );
            system_state.apply(world);
        }
        let materials_res = app.world().resource::<Assets<MetaballsUnifiedMaterial>>();
        let mat = materials_res.get(handle.id()).unwrap();
        let (_ball_count2, slot_count2) = mat.debug_counts();
        assert_eq!(slot_count2, 1, "expected shared slot for enabled cluster");
        let (_bc, sc) = mat.debug_counts();
        assert_eq!(sc, 1);
        assert_eq!(mat.data.v0.x as usize, 3);
    }
}
