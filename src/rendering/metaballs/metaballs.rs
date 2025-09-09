#![allow(clippy::type_complexity)]
// Lots of types here are intentionally kept for GPU layout and conditionally used across cfg
// boundaries. Suppress dead_code to keep layout stable and avoid fallout from cfg-based usage.
#![allow(dead_code)]
use bevy::input::keyboard::KeyCode;
use bevy::input::ButtonInput;
use bevy::prelude::Mesh2d;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bytemuck::{Pod, Zeroable};
#[cfg(target_arch = "wasm32")]
static METABALLS_UNIFIED_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::GameConfig;
// Clustering resources intentionally omitted: renderer now groups purely by color.
use crate::rendering::materials::materials::BallMaterialIndex;
use std::collections::HashMap;

// =====================================================================================
// Uniform layout
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// =====================================================================================

pub const MAX_BALLS: usize = 1024; // Legacy cap; dynamic storage buffer length may exceed but we clamp exposed count.
pub const MAX_CLUSTERS: usize = 0; // legacy removed

// =====================================================================================
// GPU Data Layout (Refactored)
// - Large balls array moved to STORAGE buffer (read-only in fragment) for scalability.
// - Uniform keeps small param vectors plus cluster colors (4KB) for simplicity.
// - v0: (ball_count_exposed, cluster_color_count, radius_scale, iso)
// - v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// - v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// - v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)  // defensive clamp uses min(v0.x, v3.w)
// - v4: (enable_early_exit, needs_gradient, reserved0, reserved1)
// NOTE: cluster_colors retained here; future optimization could move to storage buffer if needed.
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub(crate) struct MetaballsUniform {
    v0: Vec4,
    v1: Vec4,
    v2: Vec4,
    v3: Vec4,
    v4: Vec4,
    v5: Vec4, // (sdf_enabled, distance_range, channel_mode, max_gradient_samples)
    v6: Vec4, // (atlas_width, atlas_height, atlas_tile_size, atlas_shape_count | shadow_offset_mag)
    v7: Vec4, // (shadow_dir_deg, shadow_surface_scale, reserved0, reserved1)
}

impl Default for MetaballsUniform {
    fn default() -> Self {
        Self {
            v0: Vec4::new(0.0, 0.0, 1.0, 0.6),
            v1: Vec4::new(1.0, 0.0, 0.0, 0.0),
            v2: Vec4::new(0.0, 0.0, 0.0, 1.0),
            v3: Vec4::new(1.0, 1.0, 64.0, 0.0), // default 1x1 tiles, tile_size=64, balls_len=0
            v4: Vec4::new(1.0, 0.0, 0.0, 0.0), // early-exit enabled by feature flag later; needs_gradient updated per-frame
            v5: Vec4::new(0.0, 0.0, 0.0, 0.0),
            v6: Vec4::new(0.0, 0.0, 0.0, 0.0),
            v7: Vec4::new(0.0, 0.0, 0.0, 0.0),
        }
    }
}

// =====================================================================================
// Storage Buffer Types
// =====================================================================================

#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default, Pod, Zeroable)]
pub struct GpuBall {
    // data0: (x, y, radius, packed_gid)
    pub data0: Vec4,
    // data1: (cos_theta, sin_theta, reserved0, reserved1)
    // Rotation kept separate so future extensions (e.g., per-ball velocity or gradient hints)
    // can reuse remaining lanes without repacking existing shader expectations.
    pub data1: Vec4,
}
impl GpuBall {
    pub fn new(pos: Vec2, radius: f32, packed_gid: u32, cos_theta: f32, sin_theta: f32) -> Self {
        Self {
            data0: Vec4::new(pos.x, pos.y, radius, packed_gid as f32),
            data1: Vec4::new(cos_theta, sin_theta, 0.0, 0.0),
        }
    }
}

// Noise params (background)
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

// Surface noise params (edge modulation)
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default)]
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
    pub _pad2: u32,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct MetaballsUnifiedMaterial {
    #[uniform(0)]
    data: MetaballsUniform,
    #[uniform(1)]
    noise: NoiseParamsUniform,
    #[uniform(2)]
    surface_noise: SurfaceNoiseParamsUniform,
    // Storage buffers (group=2, bindings start after 0..2 uniforms)
    #[storage(3, read_only)]
    balls: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    tile_headers: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    tile_ball_indices: Handle<ShaderStorageBuffer>,
    #[storage(6, read_only)]
    cluster_palette: Handle<ShaderStorageBuffer>,
    // Optional SDF atlas texture binding; sampler will use default for now. When absent, SDF path disabled.
    #[texture(7)]
    #[sampler(9)] // Matches WGSL @group(2) @binding(9) sdf_sampler; uses default filtering (linear) for smooth SDF edges.
    sdf_atlas_tex: Option<Handle<Image>>,
    #[storage(8, read_only)]
    sdf_shape_meta: Handle<ShaderStorageBuffer>,
}

impl MetaballsUnifiedMaterial {
    #[cfg(feature = "debug")]
    pub fn set_debug_view(&mut self, view: u32) {
        self.data.v1.w = view as f32;
    }
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
    fn vertex_shader() -> ShaderRef { Self::fragment_shader() }
}

// Default is derived above.

// =====================================================================================
// CPU Tiling Resources & GPU Tile Header Type
// =====================================================================================
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub struct TileHeaderGpu {
    pub offset: u32,
    pub count: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct BallTilingConfig {
    pub tile_size: u32,
}
impl Default for BallTilingConfig {
    fn default() -> Self {
        Self { tile_size: 64 }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct BallTilesMeta {
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub last_ball_len: usize,
}

// Foreground / Background shader modes

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode {
    ClassicBlend,
    Bevel,
    OutlineGlow,
    Metadata,
}
impl MetaballForegroundMode {
    pub const ALL: [Self; 4] = [
        Self::ClassicBlend,
        Self::Bevel,
        Self::OutlineGlow,
        Self::Metadata,
    ];
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

// Simple shadow params resource (single-pass drop shadow halo). Kept minimal until
// promoted to full GameConfig integration. Uses repurposed uniform lanes:
// v5.x = shadow_softness exponent (<=0 => default 0.7)
// v5.z = shadow_enable (>0.5)
// v5.w = shadow_intensity (0..1)
// v6.w = shadow_vertical_offset (world units; applied negative Y in shader)
#[derive(Resource, Debug, Clone)]
pub struct MetaballsShadowParams {
    pub enabled: bool,
    pub intensity: f32,
    pub offset: f32,
    pub softness: f32,
}
impl Default for MetaballsShadowParams {
    fn default() -> Self {
        Self { enabled: true, intensity: 0.65, offset: 40.0, softness: 0.6 }
    }
}

// Shadow copy of balls for CPU tiling (kept in lock-step with GPU buffer each frame)
#[derive(Resource, Default, Clone)]
pub struct BallCpuShadow(pub Vec<GpuBall>);
impl Default for MetaballsParams {
    fn default() -> Self {
        Self {
            iso: 0.6,
            normal_z_scale: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MetaballsUpdateSet; // Public so other plugins (spawners) can order before this.

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
            METABALLS_UNIFIED_SHADER_HANDLE.get_or_init(|| unified_handle.clone());
        }

        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .init_resource::<MetaballsShadowParams>()
            .init_resource::<MetaballForeground>()
            .init_resource::<MetaballBackground>()
            .init_resource::<BallTilingConfig>()
            .init_resource::<BallTilesMeta>()
            .init_resource::<BallCpuShadow>()
            // Persistent color slots & cluster-based grouping removed (color-based grouping only)
            .init_resource::<MetaballsGroupDebugTimer>()
            .init_resource::<crate::rendering::metaballs::palette::ClusterPaletteStorage>()
            .add_plugins((Material2dPlugin::<MetaballsUnifiedMaterial>::default(),))
            .add_systems(
                Startup,
                (
                    initialize_toggle_from_config,
                    apply_config_to_params,
                    apply_shader_modes_from_config,
                    apply_shadow_from_config,
                    setup_metaballs,
                    log_initial_modes,
                ),
            )
            // Ensure metaball GPU buffer build happens after clustering / post-physics adjustments
            .configure_sets(
                Update,
                MetaballsUpdateSet.after(crate::core::system::system_order::PostPhysicsAdjustSet),
            )
            .add_systems(
                Update,
                (
                    update_metaballs_unified_material,
                    build_metaball_tiles.after(update_metaballs_unified_material),
                    cycle_foreground_mode,
                    cycle_background_mode,
                    resize_fullscreen_quad,
                    tweak_metaballs_params,
                )
                    .in_set(MetaballsUpdateSet),
            );
    }
}

// (Removed) PersistentColorSlots: no longer needed with pure color-based grouping.

// =====================================================================================
// Periodic debug logging of group assignments (fusion id vs persistent slot)
// =====================================================================================
#[derive(Resource)]
pub struct MetaballsGroupDebugTimer(pub Timer);
impl Default for MetaballsGroupDebugTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

// Startup / Config

fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) {
    toggle.0 = cfg.metaballs_enabled;
}

fn apply_config_to_params(mut params: ResMut<MetaballsParams>, cfg: Res<GameConfig>) {
    params.iso = cfg.metaballs.iso;
    params.normal_z_scale = cfg.metaballs.normal_z_scale;
    params.radius_multiplier = cfg.metaballs.radius_multiplier.max(0.0001);
}

fn apply_shadow_from_config(mut shadow: ResMut<MetaballsShadowParams>, cfg: Res<GameConfig>) {
    let c = &cfg.metaballs_shadow;
    shadow.enabled = c.enabled;
    shadow.intensity = c.intensity.clamp(0.0, 1.0);
    shadow.offset = c.offset.max(0.0);
    shadow.softness = c.softness;
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
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
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
    // Provide dummy shape metadata buffer (1 entry) so binding exists until SDF atlas loads
    if buffers.get(&umat.sdf_shape_meta).is_none() {
        let dummy: [f32; 8] = [0.0; 8];
        umat.sdf_shape_meta = buffers.add(ShaderStorageBuffer::from(&dummy));
    }
    umat.data.v2.x = w;
    umat.data.v2.y = h;

    // Noise
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

    // Surface noise
    let sn = &cfg.surface_noise;
    umat.surface_noise.amp = sn.amp.clamp(0.0, 0.5);
    umat.surface_noise.base_scale = if sn.base_scale > 0.0 {
        sn.base_scale
    } else {
        0.008
    };
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

// Mode cycling

fn cycle_foreground_mode(mut fg: ResMut<MetaballForeground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::End) {
        fg.idx = (fg.idx + 1) % MetaballForegroundMode::ALL.len();
        if fg.current() == MetaballForegroundMode::Metadata {
            info!(target: "metaballs", "Foreground mode -> Metadata (metadata RGBA output)");
        } else {
            info!(target: "metaballs", "Foreground mode -> {:?}", fg.current());
        }
    }
    if keys.just_pressed(KeyCode::Home) {
        fg.idx =
            (fg.idx + MetaballForegroundMode::ALL.len() - 1) % MetaballForegroundMode::ALL.len();
        if fg.current() == MetaballForegroundMode::Metadata {
            info!(target: "metaballs", "Foreground mode -> Metadata (metadata RGBA output)");
        } else {
            info!(target: "metaballs", "Foreground mode -> {:?}", fg.current());
        }
    }
}

fn cycle_background_mode(mut bg: ResMut<MetaballBackground>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::PageUp) {
        bg.idx = (bg.idx + 1) % MetaballBackgroundMode::ALL.len();
        info!(target: "metaballs", "Background mode -> {:?}", bg.current());
    }
    if keys.just_pressed(KeyCode::PageDown) {
        bg.idx =
            (bg.idx + MetaballBackgroundMode::ALL.len() - 1) % MetaballBackgroundMode::ALL.len();
        info!(target: "metaballs", "Background mode -> {:?}", bg.current());
    }
}

// Uniform update (simplified: no per-ball state differentiation)

fn update_metaballs_unified_material(
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
    if !toggle.0 {
        return;
    }
    let Ok(handle_comp) = q_mat.single() else {
        return;
    };
    let Some(mat) = materials.get_mut(&handle_comp.0) else {
        return;
    };

    // Static fields
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
        mat.surface_noise.base_scale = if sn.base_scale > 0.0 {
            sn.base_scale
        } else {
            0.008
        };
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

    // ================= Color-Based Grouping ==================
    // Build dense group ids for each encountered BallMaterialIndex color.
    use crate::rendering::palette::palette::color_for_index;
    let mut color_to_group: HashMap<usize, u32> = HashMap::new();
    let mut palette_colors: Vec<[f32; 4]> = Vec::new();
    let mut balls_cpu: Vec<GpuBall> = Vec::with_capacity(q_balls.iter().len().min(MAX_BALLS));
    let mut debug_rows: Vec<String> = Vec::new();
    for (e, tf, r, color_idx, shape_idx_opt) in q_balls.iter() {
        if balls_cpu.len() >= MAX_BALLS {
            break;
        }
        let gid = *color_to_group.entry(color_idx.0).or_insert_with(|| {
            let new_gid = palette_colors.len() as u32;
            let c = color_for_index(color_idx.0).to_srgba();
            palette_colors.push([c.red, c.green, c.blue, 1.0]);
            new_gid
        });
        let pos = tf.translation.truncate();
        // Pack shape index (u16) with color group (u16) => u32 using floats
        let shape_idx: u32 = shape_idx_opt.map(|s| s.0 as u32).unwrap_or(0);
        let packed_gid = ((shape_idx & 0xFFFF) << 16) | (gid & 0xFFFF);
    // Extract rotation (Z axis) -> angle for 2D; if no meaningful rotation use identity.
    let rot = tf.rotation;
    // Convert quaternion to 2D angle (assuming rotation about Z only for balls).
    let angle = rot.to_euler(EulerRot::XYZ).2; // Z angle
    let (s, c) = angle.sin_cos();
    balls_cpu.push(GpuBall::new(pos, r.0, packed_gid, c, s));
        if debug_rows.len() < 8 {
            debug_rows.push(format!("e={:?} color={} gid={} shape={} packed=0x{:08X}", e, color_idx.0, gid, shape_idx, packed_gid));
        }
    }
    let group_count = palette_colors.len() as u32;
    mat.data.v0.x = balls_cpu.len() as f32;
    mat.data.v3.w = balls_cpu.len() as f32;
    // Upload palette (group_count entries)
    if group_count > 0 {
        let cpu_palette = crate::rendering::metaballs::palette::ClusterPaletteCpu {
            colors: palette_colors.clone(),
            ids: Vec::new(),
            color_indices: Vec::new(),
        };
        ensure_palette_capacity_wrapper(
            &mut palette_storage,
            group_count,
            &mut buffers,
            &cpu_palette,
        );
        if let Some(h) = &palette_storage.handle {
            mat.cluster_palette = h.clone();
        }
        // group count tracked in v0.y; repurpose v5.y for sdf distance_range later
        mat.data.v0.y = group_count as f32;
    } else {
        mat.data.v0.y = 0.0;
    }

    // Upload / update storage buffer asset
    if balls_cpu.is_empty() {
        // allocate minimal 1 element to avoid empty buffer binding issues
        balls_cpu.push(GpuBall::default());
    }
    // Replace buffer each frame (can optimize with diffing later)
    let new_buf = ShaderStorageBuffer::from(balls_cpu.as_slice());
    if buffers.get(&mat.balls).is_some() {
        if let Some(b) = buffers.get_mut(&mat.balls) {
            *b = new_buf;
        }
    } else {
        mat.balls = buffers.add(new_buf);
    }

    // Early-exit enable flag & needs-gradient flag (filled here to avoid shader re-derivation)
    let fg_mode = fg.current();
    let needs_gradient = matches!(
        fg_mode,
        MetaballForegroundMode::Bevel | MetaballForegroundMode::Metadata
    );
    mat.data.v4.x = if cfg!(feature = "metaballs_early_exit") {
        1.0
    } else {
        0.0
    };
    mat.data.v4.y = if needs_gradient { 1.0 } else { 0.0 };
    mat.data.v4.z = if cfg!(feature = "metaballs_metadata_v2") {
        1.0
    } else {
        0.0
    }; // metadata v2 encoding flag
    // SDF flags (reuse v5 lanes): v5.x = sdf_enabled, v5.y = distance_range, v5.z = channel_mode (0=r8,1=rgb,2=rgba), v5.w = max_gradient_samples
    if let Some(atlas) = sdf_atlas.as_ref() {
        if atlas.enabled && cfg.sdf_shapes.enabled && !cfg.sdf_shapes.force_fallback {
            mat.data.v5.x = 1.0;
            // Map pixel distance_range -> normalized feather half-width (0..0.5) expected by shader.
            // We divide by tile_size; clamp to 0.5 to avoid excessively soft edges.
            let feather_norm = if atlas.tile_size > 0 { (atlas.distance_range / atlas.tile_size as f32).clamp(0.0, 0.5) } else { 0.0 };
            mat.data.v5.y = feather_norm;
            // NOTE: channel_mode / max_gradient_samples lanes repurposed for shadow.
            // If future SDF channel modes needed concurrently with shadow, introduce new uniform vector.
            // v6 holds atlas dimensions & shape count (tile size redundant with per-shape meta but convenient)
            mat.data.v6.x = atlas.atlas_width as f32;
            mat.data.v6.y = atlas.atlas_height as f32;
            mat.data.v6.z = atlas.tile_size as f32;
            // v6.w repurposed for shadow offset; keep gradient_step_scale latent.
            // Bind atlas texture handle if material missing one
            if mat.sdf_atlas_tex.is_none() { mat.sdf_atlas_tex = Some(atlas.texture.clone()); }
            if let Some(shape_buf) = &atlas.shape_buffer { mat.sdf_shape_meta = shape_buf.clone(); }
        } else { mat.data.v5.x = 0.0; }
    } else {
        mat.data.v5.x = 0.0; // no atlas resource
    }

    // Apply shadow parameters (after SDF so we intentionally override reused lanes).
    if shadow_params.enabled {
        mat.data.v5.z = 1.0; // enable shadow
        mat.data.v5.w = shadow_params.intensity.clamp(0.0, 1.0);
        mat.data.v6.w = shadow_params.offset.max(0.0); // offset magnitude (generic now)
        mat.data.v5.x = if shadow_params.softness <= 0.0 { 0.0 } else { shadow_params.softness }; // softness exponent (0 => shader default)
    } else {
        mat.data.v5.z = 0.0;
        mat.data.v5.w = 0.0;
    }

    // Direction & surface scale from config (MetaballsShadowConfig)
    // We access GameConfig directly for latest (if hot reload arises) else cached values are fine.
    // direction stored in degrees in v7.x; surface multiplier in v7.y
    mat.data.v7.x = cfg.metaballs_shadow.direction;
    mat.data.v7.y = cfg.metaballs_shadow.surface.max(0.05);

    // Update shadow (replace content)
    if let Some(ref mut s) = shadow {
        s.0.clear();
        // We can't reuse balls_cpu (moved), so rebuild quickly (small overhead) or clone
        // Clone existing GPU list via balls_cpu (still in scope).
        // balls_cpu currently owns the vector; just clone
        s.0.extend_from_slice(balls_cpu.as_slice());
    }

    // Periodic debug log (every ~1s) of first few group assignments for visual verification
    if dbg_timer.0.tick(time.delta()).just_finished() && !debug_rows.is_empty() {
        info!(target: "metaballs", "ColorGroups: groups={} sample: {}", group_count, debug_rows.join(" | "));
    }
}

fn ensure_palette_capacity_wrapper(
    storage: &mut crate::rendering::metaballs::palette::ClusterPaletteStorage,
    needed: u32,
    buffers: &mut Assets<ShaderStorageBuffer>,
    cpu: &crate::rendering::metaballs::palette::ClusterPaletteCpu,
) {
    use crate::rendering::metaballs::palette::ensure_palette_capacity;
    if needed == 0 {
        return;
    }
    ensure_palette_capacity(storage, needed, buffers);
    if let Some(handle) = &storage.handle {
        if let Some(buf) = buffers.get_mut(handle) {
            // Rebuild vector (capacity may exceed needed); fill first length then zero rest left from allocation.
            let mut data: Vec<[f32; 4]> = vec![[0.0; 4]; storage.capacity as usize];
            for (i, col) in cpu.colors.iter().enumerate().take(storage.length as usize) {
                data[i] = *col;
            }
            *buf = ShaderStorageBuffer::from(data.as_slice());
        }
    }
}

// =====================================================================================
// Tile Builder System
// =====================================================================================
fn build_metaball_tiles(
    windows: Query<&Window>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    tiling_cfg: Res<BallTilingConfig>,
    mut meta: ResMut<BallTilesMeta>,
    shadow: Option<Res<BallCpuShadow>>,
) {
    let Ok(handle_comp) = q_mat.single() else {
        return;
    };
    let Some(mat) = materials.get_mut(&handle_comp.0) else {
        return;
    };
    let Some(shadow) = shadow else {
        return;
    };
    if shadow.0.is_empty() {
        return;
    }

    // Viewport dims
    let (vw, vh) = if let Ok(w) = windows.single() {
        (w.width(), w.height())
    } else {
        (mat.data.v2.x, mat.data.v2.y)
    };
    if vw <= 0.0 || vh <= 0.0 {
        return;
    }
    let tile_size = tiling_cfg.tile_size.max(8) as f32; // guard tiny values
    let tiles_x = ((vw / tile_size).ceil() as u32).max(1);
    let tiles_y = ((vh / tile_size).ceil() as u32).max(1);
    let tile_count = (tiles_x * tiles_y) as usize;

    // Recompute every frame (previous early-out caused visual artifacts when balls crossed tile boundaries).
    let balls_len = shadow.0.len();

    // Prepare buckets
    let mut buckets: Vec<Vec<u32>> = Vec::with_capacity(tile_count);
    for _ in 0..tile_count {
        buckets.push(Vec::new());
    }

    let origin_x = -vw * 0.5;
    let origin_y = -vh * 0.5;
    let radius_scale = mat.data.v0.z; // 1/k
    let radius_mult = mat.data.v2.w; // params.radius_multiplier

    // Assign balls to tiles
    for (i, b) in shadow.0.iter().enumerate() {
        // removed unused center var
            let base_r = b.data0.z;
            let center3 = b.data0.truncate();
        let center = Vec2::new(center3.x, center3.y);
        if base_r <= 0.0 {
            continue;
        }
    let scaled_r = base_r * radius_scale * radius_mult;
        // Fudge factor to counteract boundary flooring so tiles adjacent to the circle edge don't miss inclusion.
        let pad = 1.5_f32; // in pixels; tiny vs typical radii but prevents hairline cracks
        let effective_r = scaled_r + pad;

        // Compute bounds in screen space relative to origin (convert to [0,vw]/[0,vh])
        let min_x = center.x - effective_r - origin_x; // since origin_x is negative
        let max_x = center.x + effective_r - origin_x;
        let min_y = center.y - effective_r - origin_y;
        let max_y = center.y + effective_r - origin_y;

        // Convert to tile indices
        let mut tx0 = (min_x / tile_size).floor() as i32;
        let mut tx1 = (max_x / tile_size).floor() as i32;
        let mut ty0 = (min_y / tile_size).floor() as i32;
        let mut ty1 = (max_y / tile_size).floor() as i32;
        tx0 = tx0.clamp(0, tiles_x as i32 - 1);
        tx1 = tx1.clamp(0, tiles_x as i32 - 1);
        ty0 = ty0.clamp(0, tiles_y as i32 - 1);
        ty1 = ty1.clamp(0, tiles_y as i32 - 1);

        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                let idx = (ty as u32 * tiles_x + tx as u32) as usize;
                buckets[idx].push(i as u32);
            }
        }
    }

    // Build headers & flattened index list
    let mut headers_cpu: Vec<TileHeaderGpu> = Vec::with_capacity(tile_count);
    headers_cpu.resize(tile_count, TileHeaderGpu::default());
    let mut indices_cpu: Vec<u32> = Vec::with_capacity(shadow.0.len() * 2); // rough heuristic
    let mut running: u32 = 0;
    for (t, bucket) in buckets.iter().enumerate() {
        let count = bucket.len() as u32;
        headers_cpu[t] = TileHeaderGpu {
            offset: running,
            count,
            _pad0: 0,
            _pad1: 0,
        };
        indices_cpu.extend_from_slice(bucket);
        running += count;
    }

    if headers_cpu.is_empty() {
        headers_cpu.push(TileHeaderGpu::default());
    }
    if indices_cpu.is_empty() {
        indices_cpu.push(0);
    }

    // Upload headers & indices buffers
    let headers_buf = ShaderStorageBuffer::from(headers_cpu.as_slice());
    let indices_buf = ShaderStorageBuffer::from(indices_cpu.as_slice());
    if buffers.get(&mat.tile_headers).is_some() {
        if let Some(h) = buffers.get_mut(&mat.tile_headers) {
            *h = headers_buf;
        }
    } else {
        mat.tile_headers = buffers.add(headers_buf);
    }
    if buffers.get(&mat.tile_ball_indices).is_some() {
        if let Some(h) = buffers.get_mut(&mat.tile_ball_indices) {
            *h = indices_buf;
        }
    } else {
        mat.tile_ball_indices = buffers.add(indices_buf);
    }

    // Update uniform tile meta
    mat.data.v3.x = tiles_x as f32;
    mat.data.v3.y = tiles_y as f32;
    mat.data.v3.z = tile_size;

    // Record meta for change detection
    meta.tiles_x = tiles_x;
    meta.tiles_y = tiles_y;
    meta.last_ball_len = balls_len;
}

// Resize handling

fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_unified: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
) {
    let Ok(window) = windows.single() else {
        return;
    }; // keep compatibility until all call sites updated
    if let Ok(handle_comp) = q_unified.single() {
        if let Some(mat) = unified_mats.get_mut(&handle_comp.0) {
            if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() {
                mat.data.v2.x = window.width();
                mat.data.v2.y = window.height();
            }
        }
    }
}

// Param tweaks (iso)

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

// Helper mirroring WGSL normalization for metadata SDF proxy (kept pure for testing)
pub fn map_signed_distance(signed_d: f32, d_scale: f32) -> f32 {
    (0.5 - 0.5 * signed_d / d_scale).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    // (Removed) persistent_slot_allocation_and_reuse test; color grouping no longer uses per-entity slots.

    #[test]
    fn sdf_mapping_basic() {
        let scale = 8.0;
        // Inside surface: signed_d negative => value > 0.5
        let inside = map_signed_distance(-2.0, scale);
        assert!(inside > 0.5, "inside expected > 0.5 got {}", inside);
        // On surface: signed_d = 0 => 0.5
        let surface = map_signed_distance(0.0, scale);
        assert!((surface - 0.5).abs() < 1e-6);
        // Outside: signed_d positive => value < 0.5
        let outside = map_signed_distance(4.0, scale);
        assert!(outside < 0.5, "outside expected < 0.5 got {}", outside);
        // Far outside clamps to 0.0
        let far = map_signed_distance(1e6, scale);
        assert!((0.0..=0.001).contains(&far));
    }

    #[test]
    fn metadata_enum_value_matches() {
        assert_eq!(MetaballForegroundMode::Metadata as u32, 3);
    }

    #[test]
    fn cycling_wraps_with_metadata() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let fg = MetaballForeground {
            idx: MetaballForegroundMode::ALL.len() - 1,
        };
        app.insert_resource(fg);
        // Simulate End key press to wrap around
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::End);
        app.insert_resource(keys);
        // Manually invoke system logic
        // Borrow ordering: clone keys first, then take mut fg
        let world_keys = app.world().resource::<ButtonInput<KeyCode>>().clone();
        let mut world_fg = app.world_mut().resource_mut::<MetaballForeground>();
        if world_keys.just_pressed(KeyCode::End) {
            world_fg.idx = (world_fg.idx + 1) % MetaballForegroundMode::ALL.len();
        }
        if world_keys.just_pressed(KeyCode::Home) {
            world_fg.idx = (world_fg.idx + MetaballForegroundMode::ALL.len() - 1)
                % MetaballForegroundMode::ALL.len();
        }
        let fg_after = app.world().resource::<MetaballForeground>();
        assert_eq!(
            fg_after.current() as u32,
            MetaballForegroundMode::ClassicBlend as u32
        );
    }

    #[test]
    fn color_group_assignment_basic() {
        // Simulate three balls with colors [0,0,2] => two groups.
        let mut color_to_group: HashMap<usize, u32> = HashMap::new();
        let colors = [0usize, 0, 2];
        let mut palette: Vec<[f32; 4]> = Vec::new();
        for c in colors.iter() {
            color_to_group.entry(*c).or_insert_with(|| {
                let gid = palette.len() as u32;
                palette.push([*c as f32, 0.0, 0.0, 1.0]);
                gid
            });
        }
        assert_eq!(palette.len(), 2, "expected 2 distinct color groups");
        assert_eq!(color_to_group.get(&0).copied(), Some(0));
        assert_eq!(color_to_group.get(&2).copied(), Some(1));
    }
}
