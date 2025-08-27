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

use crate::core::components::{Ball, BallRadius};
use crate::core::config::GameConfig;
use crate::physics::clustering::cluster::Clusters;
use crate::rendering::materials::materials::BallMaterialIndex;
use crate::rendering::palette::palette::color_for_index;

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
}

impl Material2d for MetaballsUnifiedMaterial {
    fn fragment_shader() -> ShaderRef {
        // TEMP: Default to debug shader for diagnostics. Restore original once issue resolved.
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
    OutlineGlow, // (initially can behave like Classic; kept for future expansion)
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
                    update_metaballs_unified_material,
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
// Mode Cycling (independent axes)
// Home / End : foreground (Home=prev, End=next) for directional semantics
// PageDown / PageUp : background (PageDown=prev, PageUp=next)
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
// Uniform Update
// =====================================================================================

#[allow(clippy::too_many_arguments)] // Aggregates necessary ECS params for uniform update; keeping single pass for cache locality.
fn update_metaballs_unified_material(
    time: Res<Time>,
    fg: Res<MetaballForeground>,
    bg: Res<MetaballBackground>,
    clusters: Res<Clusters>,
    q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    cfg: Res<GameConfig>, // access noise config
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

    // PACK UNIFORM FIELDS (semantics updated but binary layout constant)
    mat.data.v0.w = params.iso; // iso
    mat.data.v1.x = params.normal_z_scale; // normal z scale
    mat.data.v1.y = (fg.current() as u32) as f32; // foreground_mode index
    mat.data.v1.z = (bg.current() as u32) as f32; // background_mode index: 0=SolidGray,1=ProceduralNoise,2=VerticalGradient
    mat.data.v2.z = time.elapsed_secs(); // animated time (noise / future reactive bg)
    mat.data.v2.w = params.radius_multiplier.max(0.0001); // radius_multiplier relocated (was v1.z)
                                                          // Derived radius scale maintaining legacy behavior (inverse from iso shaping)
    let iso = params.iso.clamp(1e-4, 0.9999);
    let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt();
    mat.data.v0.z = 1.0 / k;

    // Update noise uniform every frame (allows hot-reload of config or dynamic edits)
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

    // Update surface noise uniform
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

    // Cluster colors
    let mut color_count = 0usize;
    for cl in clusters.0.iter() {
        if color_count >= MAX_CLUSTERS {
            break;
        }
        let color = color_for_index(cl.color_index);
        let srgb = color.to_srgba();
        mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0);
        color_count += 1;
    }
    mat.data.v0.y = color_count as f32;

    // Balls
    let mut ball_count = 0usize;
    for (tf, radius, color_idx) in q_balls.iter() {
        if ball_count >= MAX_BALLS {
            break;
        }
        let pos = tf.translation.truncate();
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
    mat.data.v0.x = ball_count as f32;
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
// Param tweaks (iso) – unchanged semantics
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
