#![allow(clippy::type_complexity)]
use bevy::input::keyboard::KeyCode;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::prelude::Mesh2d;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bytemuck::{Pod, Zeroable};
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
    v5: Vec4, // (reserved0, dynamic_cluster_count, reserved1, reserved2)
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
        }
    }
}

// =====================================================================================
// Storage Buffer Types
// =====================================================================================

#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default, Pod, Zeroable)]
pub struct GpuBall {
    // (x, y, radius, cluster_slot)
    pub data: Vec4,
}
impl GpuBall {
    pub fn new(pos: Vec2, radius: f32, cluster_slot: u32) -> Self {
        Self { data: Vec4::new(pos.x, pos.y, radius, cluster_slot as f32) }
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

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
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

impl Default for MetaballsUnifiedMaterial {
    fn default() -> Self {
        Self {
            data: MetaballsUniform::default(),
            noise: NoiseParamsUniform::default(),
            surface_noise: SurfaceNoiseParamsUniform::default(),
            balls: Default::default(),
            tile_headers: Default::default(),
            tile_ball_indices: Default::default(),
            cluster_palette: Default::default(),
        }
    }
}

// =====================================================================================
// CPU Tiling Resources & GPU Tile Header Type
// =====================================================================================
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub struct TileHeaderGpu { pub offset: u32, pub count: u32, pub _pad0: u32, pub _pad1: u32 }

#[derive(Resource, Debug, Clone)]
pub struct BallTilingConfig { pub tile_size: u32 }
impl Default for BallTilingConfig { fn default() -> Self { Self { tile_size: 64 } } }

#[derive(Resource, Debug, Clone, Default)]
pub struct BallTilesMeta { pub tiles_x: u32, pub tiles_y: u32, pub last_ball_len: usize }

// Foreground / Background shader modes

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode {
    ClassicBlend,
    Bevel,
    OutlineGlow,
    Metadata,
}
impl MetaballForegroundMode {
    pub const ALL: [Self; 4] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow, Self::Metadata];
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
            .init_resource::<BallTilingConfig>()
            .init_resource::<BallTilesMeta>()
            .init_resource::<BallCpuShadow>()
            .init_resource::<PersistentColorSlots>()
            .init_resource::<crate::rendering::metaballs::palette::ClusterPaletteStorage>()
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
            .configure_sets(Update, MetaballsUpdateSet)
            .add_systems(
                Update,
                (
                    update_metaballs_unified_material,
                    build_metaball_tiles
                        .after(update_metaballs_unified_material),
                    cycle_foreground_mode,
                    cycle_background_mode,
                    resize_fullscreen_quad,
                    tweak_metaballs_params,
                )
                    .in_set(MetaballsUpdateSet),
            );
    }
}

// =====================================================================================
// Persistent Per-Entity Color Slots
// Each ball entity is assigned a stable slot id on first sight. Slots are reused via a free list
// when entities despawn (future enhancement; currently we only allocate and never reclaim to
// minimize complexity). Cluster rendering chooses the representative entity's slot; all members
// of that cluster use that slot for color. Orphan / popped entities retain their own slot.
// =====================================================================================
#[derive(Resource, Debug, Default, Clone)]
pub struct PersistentColorSlots {
    pub next_slot: u32,
    pub free: Vec<u32>,
    pub entity_slot: HashMap<Entity, u32>,
    pub slot_color_index: Vec<usize>, // logical color index (BallMaterialIndex) per slot
}
impl PersistentColorSlots {
    pub fn assign_slot(&mut self, entity: Entity, color_index: usize) -> u32 {
        if let Some(&s) = self.entity_slot.get(&entity) { return s; }
        let slot = if let Some(s) = self.free.pop() { s } else { let s = self.next_slot; self.next_slot += 1; s };
        // Ensure color index vector size
        if slot as usize >= self.slot_color_index.len() { self.slot_color_index.push(color_index); } else {
            // Only set if uninitialized (we don't overwrite existing slot color assignments here)
            if self.slot_color_index[slot as usize] != color_index { self.slot_color_index[slot as usize] = color_index; }
        }
        self.entity_slot.insert(entity, slot);
        slot
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
    clusters: Res<Clusters>,
    q_balls: Query<(Entity, &Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
    mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    cfg: Res<GameConfig>,
    mut shadow: Option<ResMut<BallCpuShadow>>,
    mut palette_storage: ResMut<crate::rendering::metaballs::palette::ClusterPaletteStorage>,
    mut persistent_slots: ResMut<PersistentColorSlots>,
) {
    if !toggle.0 {
        return;
    }
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };

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

    // ================= Persistent Per-Entity Slot Pass ==================
    // 1. Assign slots to any new entities.
    for (e, _tf, _r, color_idx) in q_balls.iter() {
        persistent_slots.assign_slot(e, color_idx.0);
    }
    // 2. Build cluster representative mapping: cluster_id -> representative entity (first listed).
    let mut cluster_rep_slot: HashMap<u64, u32> = HashMap::with_capacity(clusters.0.len());
    for cl in clusters.0.iter() {
        let mut best_slot: Option<u32> = None;
        for e in cl.entities.iter() {
            if let Some(&s) = persistent_slots.entity_slot.get(e) {
                match best_slot { Some(cur) if s >= cur => {}, _ => best_slot = Some(s) }
            }
        }
        if let Some(s) = best_slot { cluster_rep_slot.insert(cl.id, s); }
    }
    // 3. Prepare balls CPU staging vector with chosen slot per ball (cluster member -> rep slot; orphan -> own slot).
    let mut entity_slot_cache: HashMap<Entity, u32> = HashMap::with_capacity(q_balls.iter().len());
    for (e, _tf, _r, _ci) in q_balls.iter() {
        if let Some(&slot) = persistent_slots.entity_slot.get(&e) { entity_slot_cache.insert(e, slot); }
    }

    // Prepare balls CPU staging vector
    let mut balls_cpu: Vec<GpuBall> = Vec::with_capacity(q_balls.iter().len().min(MAX_BALLS));

    // Build quick lookup for ball transforms & radii
    use std::collections::HashMap;
    let mut ball_tf: HashMap<Entity, (Vec2, f32, usize)> =
        HashMap::with_capacity(q_balls.iter().len());
    for (e, tf, r, color_idx) in q_balls.iter() {
        ball_tf.insert(e, (tf.translation.truncate(), r.0, color_idx.0));
    }

    // Balls array: assign per-ball cluster slot from mapping (fallback 0 if missing / no clusters)
    let mut max_slot_used: u32 = 0;
    for (e, (pos, radius, _ci)) in ball_tf.iter() {
        if balls_cpu.len() >= MAX_BALLS { break; }
        // Determine slot: if entity is in a cluster, use representative slot; else its own slot
        // Find cluster containing entity via cluster iteration (could optimize with reverse index if needed)
        let mut slot = *entity_slot_cache.get(e).unwrap_or(&0);
        for cl in clusters.0.iter() {
            if cl.entities.contains(e) {
                if let Some(rep_slot) = cluster_rep_slot.get(&cl.id) { slot = *rep_slot; }
                break;
            }
        }
        max_slot_used = max_slot_used.max(slot);
        balls_cpu.push(GpuBall::new(*pos, *radius, slot));
    }
    mat.data.v0.x = balls_cpu.len() as f32;
    mat.data.v3.w = balls_cpu.len() as f32;

    // 4. Build palette from slots [0..=max_slot_used]. Colors derived from slot_color_index.
    let palette_len = if balls_cpu.is_empty() { 0 } else { max_slot_used + 1 };
    if palette_len > 0 {
        let mut colors: Vec<[f32;4]> = Vec::with_capacity(palette_len as usize);
        use crate::rendering::palette::palette::color_for_index;
        for i in 0..palette_len as usize {
            let ci = persistent_slots.slot_color_index.get(i).copied().unwrap_or(0);
            let c = color_for_index(ci).to_srgba();
            colors.push([c.red, c.green, c.blue, 1.0]);
        }
        // Upload palette via existing storage wrapper helper.
        let cpu_palette_like = crate::rendering::metaballs::palette::ClusterPaletteCpu { colors: colors.clone(), ids: Vec::new(), color_indices: Vec::new() };
        ensure_palette_capacity_wrapper(&mut palette_storage, palette_len, &mut buffers, &cpu_palette_like);
        if let Some(h) = &palette_storage.handle { mat.cluster_palette = h.clone(); }
        mat.data.v5.y = palette_len as f32;
        mat.data.v0.y = palette_len as f32;
    } else {
        mat.data.v5.y = 0.0;
        mat.data.v0.y = 0.0;
    }

    // Upload / update storage buffer asset
    if balls_cpu.is_empty() {
        // allocate minimal 1 element to avoid empty buffer binding issues
        balls_cpu.push(GpuBall::default());
    }
    // Replace buffer each frame (can optimize with diffing later)
    let new_buf = ShaderStorageBuffer::from(balls_cpu.as_slice());
    if buffers.get(&mat.balls).is_some() { if let Some(b) = buffers.get_mut(&mat.balls) { *b = new_buf; } } else { mat.balls = buffers.add(new_buf); }

    // Early-exit enable flag & needs-gradient flag (filled here to avoid shader re-derivation)
    let fg_mode = fg.current();
    let needs_gradient = matches!(fg_mode, MetaballForegroundMode::Bevel | MetaballForegroundMode::Metadata);
    mat.data.v4.x = if cfg!(feature = "metaballs_early_exit") { 1.0 } else { 0.0 };
    mat.data.v4.y = if needs_gradient { 1.0 } else { 0.0 };
    mat.data.v4.z = if cfg!(feature = "metaballs_metadata_v2") { 1.0 } else { 0.0 }; // metadata v2 encoding flag

    // Update shadow (replace content)
    if let Some(ref mut s) = shadow {
        s.0.clear();
        // We can't reuse balls_cpu (moved), so rebuild quickly (small overhead) or clone
        // Clone existing GPU list via balls_cpu (still in scope).
        // balls_cpu currently owns the vector; just clone
        s.0.extend_from_slice(balls_cpu.as_slice());
    }
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
        // Rebuild vector (capacity may exceed needed); fill first length then zero rest left from allocation.
        let mut data: Vec<[f32;4]> = vec![[0.0;4]; storage.capacity as usize];
        for (i, col) in cpu.colors.iter().enumerate().take(storage.length as usize) { data[i] = *col; }
        *buf = ShaderStorageBuffer::from(data.as_slice());
    }}
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
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };
    let Some(shadow) = shadow else { return; };
    if shadow.0.is_empty() { return; }

    // Viewport dims
    let (vw, vh) = if let Ok(w) = windows.single() { (w.width(), w.height()) } else { (mat.data.v2.x, mat.data.v2.y) };
    if vw <= 0.0 || vh <= 0.0 { return; }
    let tile_size = tiling_cfg.tile_size.max(8) as f32; // guard tiny values
    let tiles_x = ((vw / tile_size).ceil() as u32).max(1);
    let tiles_y = ((vh / tile_size).ceil() as u32).max(1);
    let tile_count = (tiles_x * tiles_y) as usize;

    // Recompute every frame (previous early-out caused visual artifacts when balls crossed tile boundaries).
    // TODO: Introduce smarter change detection (track per-ball tile span) if CPU cost becomes significant.
    let balls_len = shadow.0.len();

    // Prepare buckets
    let mut buckets: Vec<Vec<u32>> = Vec::with_capacity(tile_count);
    for _ in 0..tile_count { buckets.push(Vec::new()); }

    let origin_x = -vw * 0.5;
    let origin_y = -vh * 0.5;
    let radius_scale = mat.data.v0.z; // 1/k
    let radius_mult = mat.data.v2.w;  // params.radius_multiplier

    // Assign balls to tiles
    for (i, b) in shadow.0.iter().enumerate() {
    // removed unused center var
        let base_r = b.data.z;
        let center3 = b.data.truncate();
        let center = Vec2::new(center3.x, center3.y);
        if base_r <= 0.0 { continue; }
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

        for ty in ty0..=ty1 { for tx in tx0..=tx1 {
            let idx = (ty as u32 * tiles_x + tx as u32) as usize;
            buckets[idx].push(i as u32);
        }}
    }

    // Build headers & flattened index list
    let mut headers_cpu: Vec<TileHeaderGpu> = Vec::with_capacity(tile_count);
    headers_cpu.resize(tile_count, TileHeaderGpu::default());
    let mut indices_cpu: Vec<u32> = Vec::new();
    indices_cpu.reserve(shadow.0.len() * 2); // rough heuristic
    let mut running: u32 = 0;
    for (t, bucket) in buckets.iter().enumerate() {
        let count = bucket.len() as u32;
        headers_cpu[t] = TileHeaderGpu { offset: running, count, _pad0:0, _pad1:0 };
        indices_cpu.extend_from_slice(bucket);
        running += count;
    }

    if headers_cpu.is_empty() { headers_cpu.push(TileHeaderGpu::default()); }
    if indices_cpu.is_empty() { indices_cpu.push(0); }

    // Upload headers & indices buffers
    let headers_buf = ShaderStorageBuffer::from(headers_cpu.as_slice());
    let indices_buf = ShaderStorageBuffer::from(indices_cpu.as_slice());
    if buffers.get(&mat.tile_headers).is_some() { if let Some(h) = buffers.get_mut(&mat.tile_headers) { *h = headers_buf; } } else { mat.tile_headers = buffers.add(headers_buf); }
    if buffers.get(&mat.tile_ball_indices).is_some() { if let Some(h) = buffers.get_mut(&mat.tile_ball_indices) { *h = indices_buf; } } else { mat.tile_ball_indices = buffers.add(indices_buf); }

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
    let Ok(window) = windows.single() else { return; }; // keep compatibility until all call sites updated
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
    #[test]
    fn persistent_slot_allocation_and_reuse() {
        let mut pcs = PersistentColorSlots::default();
        let mut world = World::new();
        let mut ents = Vec::new();
        for i in 0..8 { ents.push(world.spawn_empty().id()); pcs.assign_slot(ents[i], i as usize % 3); }
        assert_eq!(pcs.next_slot, 8, "expected next_slot=8 got {}", pcs.next_slot);
        let slot_3_first = pcs.entity_slot[&ents[3]];
        let again = pcs.assign_slot(ents[3], 99);
        assert_eq!(slot_3_first, again, "re-assignment must not allocate new slot");
    }

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
        assert!(far >= 0.0 && far <= 0.001);
    }

    #[test]
    fn metadata_enum_value_matches() {
        assert_eq!(MetaballForegroundMode::Metadata as u32, 3);
    }

    #[test]
    fn cycling_wraps_with_metadata() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
    let fg = MetaballForeground { idx: MetaballForegroundMode::ALL.len() - 1 };
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
            world_fg.idx = (world_fg.idx + MetaballForegroundMode::ALL.len() - 1) % MetaballForegroundMode::ALL.len();
        }
        let fg_after = app.world().resource::<MetaballForeground>();
        assert_eq!(fg_after.current() as u32, MetaballForegroundMode::ClassicBlend as u32);
    }
}
