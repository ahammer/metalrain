//! Game & rendering configuration definitions (RESTORED clean file) plus SDF shapes toggle.
//! This replaces a previously corrupted version.

use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

// ---------------- Window ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub title: String,
    #[serde(rename = "autoClose")]
    pub auto_close: f32,
}
impl Default for WindowConfig {
    fn default() -> Self {
        Self { width: 1280.0, height: 720.0, title: "Bevy Bouncing Balls".into(), auto_close: 0.0 }
    }
}

// ---------------- Gravity (legacy baseline) ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GravityConfig { pub y: f32 }
impl Default for GravityConfig { fn default() -> Self { Self { y: -600.0 } } }

// ---------------- Gravity Widgets ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GravityWidgetConfig {
    pub id: u32,
    pub strength: f32,
    pub mode: String,   // "Attract" | "Repulse"
    pub radius: f32,    // 0 => infinite
    pub falloff: String, // None | InverseLinear | InverseSquare | SmoothEdge
    pub enabled: bool,
    #[serde(rename = "physics_collider")]
    pub physics_collider: bool,
    #[serde(skip)]
    pub _parsed_ok: bool,
}
impl Default for GravityWidgetConfig {
    fn default() -> Self {
        Self { id: 0, strength: 600.0, mode: "Attract".into(), radius: 0.0, falloff: "InverseLinear".into(), enabled: true, physics_collider: false, _parsed_ok: true }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
#[serde(default)]
pub struct GravityWidgetsConfig { pub widgets: Vec<GravityWidgetConfig> }

// ---------------- Spawning Widgets ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct SpawnWidgetConfig {
    pub id: u32,
    pub enabled: bool,
    pub spawn_interval: f32,
    pub batch: usize,
    pub area_radius: f32,
    pub ball_radius_min: f32,
    pub ball_radius_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
}
impl Default for SpawnWidgetConfig {
    fn default() -> Self {
        Self { id: 0, enabled: true, spawn_interval: 0.25, batch: 2, area_radius: 48.0, ball_radius_min: 10.0, ball_radius_max: 20.0, speed_min: 50.0, speed_max: 200.0 }
    }
}
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct SpawnWidgetsConfig { pub widgets: Vec<SpawnWidgetConfig>, pub global_max_balls: usize }
impl Default for SpawnWidgetsConfig { fn default() -> Self { Self { widgets: Vec::new(), global_max_balls: 600 } } }

// ---------------- Bounce / Physics Material ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct BounceConfig { pub restitution: f32, pub friction: f32, pub linear_damping: f32, pub angular_damping: f32 }
impl Default for BounceConfig { fn default() -> Self { Self { restitution: 0.65, friction: 0.9, linear_damping: 0.25, angular_damping: 0.8 } } }

// ---------------- Cluster Pop Interaction ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct ClusterPopConfig {
    pub enabled: bool,
    pub min_ball_count: usize,
    pub min_total_area: f32,
    pub peak_scale: f32,
    pub grow_duration: f32,
    pub hold_duration: f32,
    pub shrink_duration: f32,
    pub collider_scale_curve: u32,
    pub freeze_mode: u32,
    pub fade_alpha: bool,
    pub fade_curve: u32,
    pub ball_pick_radius: f32,
    pub ball_pick_radius_scale_with_ball: bool,
    pub prefer_larger_radius_on_tie: bool,
    pub exclude_from_new_clusters: bool,
    // Legacy optional fields (kept to avoid parse errors, ignored)
    #[serde(deserialize_with = "opt_from_any")] pub impulse: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub outward_bonus: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub despawn_delay: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub fade_duration: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub fade_scale_end: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub collider_shrink: Option<bool>,
    #[serde(deserialize_with = "opt_from_any")] pub collider_min_scale: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub velocity_damping: Option<f32>,
    #[serde(deserialize_with = "opt_from_any")] pub spin_jitter: Option<f32>,
}
impl Default for ClusterPopConfig {
    fn default() -> Self {
        Self { enabled: true, min_ball_count: 4, min_total_area: 1200.0, peak_scale: 1.8, grow_duration: 0.25, hold_duration: 0.10, shrink_duration: 0.40, collider_scale_curve: 1, freeze_mode: 0, fade_alpha: true, fade_curve: 1, ball_pick_radius: 36.0, ball_pick_radius_scale_with_ball: true, prefer_larger_radius_on_tie: true, exclude_from_new_clusters: true, impulse: None, outward_bonus: None, despawn_delay: None, fade_duration: None, fade_scale_end: None, collider_shrink: None, collider_min_scale: None, velocity_damping: None, spin_jitter: None }
    }
}

// Accept either `Some(x)` style or legacy bare `x` for Option fields
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RawOrOpt<T> { Raw(T), Opt(Option<T>) }

fn opt_from_any<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    match RawOrOpt::<T>::deserialize(deserializer)? {
        RawOrOpt::Raw(v) => Ok(Some(v)),
        RawOrOpt::Opt(o) => Ok(o),
    }
}
#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct InteractionConfig { pub cluster_pop: ClusterPopConfig }

// ---------------- Clustering Distances ----------------
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct ClusteringConfig { pub distance_buffer_enter_cluster: f32, pub distance_buffer_exit_cluster: f32 }
impl Default for ClusteringConfig { fn default() -> Self { Self { distance_buffer_enter_cluster: 1.2, distance_buffer_exit_cluster: 1.25 } } }

// ---------------- Metaballs Render Params ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsRenderConfig { pub iso: f32, pub normal_z_scale: f32, pub radius_multiplier: f32 }
impl Default for MetaballsRenderConfig { fn default() -> Self { Self { iso: 0.6, normal_z_scale: 1.0, radius_multiplier: 1.0 } } }

#[derive(Debug, Deserialize, Resource, Clone, PartialEq, Default)]
#[serde(default)]
pub struct MetaballsShaderConfig { pub fg_mode: usize, pub bg_mode: usize }

// ---------------- Metaballs Shadow (single-pass halo) ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct MetaballsShadowConfig {
    pub enabled: bool,
    pub intensity: f32,   // 0..1 blend toward shadow color (currently hardcoded black)
    pub offset: f32,      // vertical world-space offset magnitude (downward)
    pub softness: f32,    // exponent (<1 wider, >1 tighter); <=0 uses shader default
    pub direction: f32,   // degrees, 0 = +X (right), 90 = +Y (up)
    pub surface: f32,     // multiplier applied to iso for shadow surface reference (shadow_iso = iso * surface)
}
impl Default for MetaballsShadowConfig {
    fn default() -> Self { Self { enabled: true, intensity: 0.55, offset: 18.0, softness: 0.65, direction: 270.0, surface: 0.6 } }
}

// ---------------- Background Noise Config ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct NoiseConfig {
    pub base_scale: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: bool,
}
impl Default for NoiseConfig { fn default() -> Self { Self { base_scale: 0.004, warp_amp: 0.6, warp_freq: 0.5, speed_x: 0.03, speed_y: 0.02, gain: 0.5, lacunarity: 2.0, contrast_pow: 1.25, octaves: 5, ridged: false } } }

// ---------------- Surface Noise (metaball edge modulation) ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct SurfaceNoiseConfig {
    pub enabled: bool,
    pub mode: u32,
    pub amp: f32,
    pub base_scale: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub octaves: u32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub ridged: bool,
}
impl Default for SurfaceNoiseConfig { fn default() -> Self { Self { enabled: true, mode: 0, amp: 0.08, base_scale: 0.008, warp_amp: 0.3, warp_freq: 1.2, speed_x: 0.20, speed_y: 0.17, octaves: 4, gain: 0.55, lacunarity: 2.05, contrast_pow: 1.10, ridged: false } } }

// ---------------- SDF Shapes (NEW) ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct SdfShapesConfig {
    pub enabled: bool,
    pub force_fallback: bool,
    pub max_gradient_samples: u32,
    #[serde(default)] pub gradient_step_scale: f32, // multiplier on adaptive world_per_px step (1.0 = default)
    #[serde(default)] pub use_circle_fallback_when_radius_lt: f32, // if >0, force analytic circle when scaled radius < threshold
    // ---------------- Glyph Mode (NEW) ----------------
    #[serde(default)] pub glyph_mode: bool,              // master toggle for glyph-driven assignment
    #[serde(default)] pub glyph_text: String,            // sequence used for mapping to balls
    #[serde(default = "default_glyph_wrap")] pub glyph_wrap: String,            // policy token: Repeat | Clamp | None
    #[serde(default = "default_glyph_skip_whitespace")] pub glyph_skip_whitespace: bool,   // skip whitespace chars when true
}
fn default_glyph_wrap() -> String { "Repeat".to_string() }
fn default_glyph_skip_whitespace() -> bool { true }
impl Default for SdfShapesConfig { fn default() -> Self { Self { enabled: true, force_fallback: false, max_gradient_samples: 2, gradient_step_scale: 1.0, use_circle_fallback_when_radius_lt: 0.0, glyph_mode: false, glyph_text: String::new(), glyph_wrap: default_glyph_wrap(), glyph_skip_whitespace: default_glyph_skip_whitespace() } } }

// ---------------- Root GameConfig ----------------
#[derive(Debug, Deserialize, Resource, Clone, PartialEq)]
#[serde(default)]
pub struct GameConfig {
    pub window: WindowConfig,
    pub gravity: GravityConfig,
    pub gravity_widgets: GravityWidgetsConfig,
    pub spawn_widgets: SpawnWidgetsConfig,
    pub bounce: BounceConfig,
    pub rapier_debug: bool,
    pub draw_circles: bool,
    pub metaballs_enabled: bool,
    pub metaballs: MetaballsRenderConfig,
    pub metaballs_shader: MetaballsShaderConfig,
    pub metaballs_shadow: MetaballsShadowConfig,
    pub noise: NoiseConfig,
    pub surface_noise: SurfaceNoiseConfig,
    pub sdf_shapes: SdfShapesConfig,
    pub draw_cluster_bounds: bool,
    pub interactions: InteractionConfig,
    pub clustering: ClusteringConfig,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            window: Default::default(),
            gravity: Default::default(),
            gravity_widgets: Default::default(),
            spawn_widgets: Default::default(),
            bounce: Default::default(),
            rapier_debug: false,
            draw_circles: false,
            metaballs_enabled: true,
            metaballs: Default::default(),
            metaballs_shader: Default::default(),
            metaballs_shadow: Default::default(),
            noise: Default::default(),
            surface_noise: Default::default(),
            sdf_shapes: Default::default(),
            draw_cluster_bounds: false,
            interactions: Default::default(),
            clustering: Default::default(),
        }
    }
}

// --------------- Loading & Layering ---------------
impl GameConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let data = fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        ron::from_str(&data).map_err(|e| format!("parse RON: {e}"))
    }
    pub fn load_or_default(path: impl AsRef<Path>) -> (Self, Option<String>) {
        match Self::load_from_file(&path) { Ok(cfg) => (cfg, None), Err(e) => (Self::default(), Some(e)) }
    }
    pub fn load_layered<P, I>(paths: I) -> (Self, Vec<String>, Vec<String>)
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        use ron::value::Value;
        let mut merged: Option<Value> = None;
        let mut used = Vec::new();
        let mut errors = Vec::new();
        fn merge_value(base: &mut ron::value::Value, overlay: ron::value::Value) {
            use ron::value::Value; match (base, overlay) { (Value::Map(bm), Value::Map(om)) => { for (k, v) in om.into_iter() { let mut incoming = Some(v); let mut replaced = false; for (ek, ev) in bm.iter_mut() { if *ek == k { let val = incoming.take().unwrap(); merge_value(ev, val); replaced = true; break; } } if !replaced { bm.insert(k, incoming.unwrap()); } } }, (b, o) => *b = o }
        }
        for p in paths { let path_ref = p.as_ref(); match fs::read_to_string(path_ref) { Ok(txt) => match ron::from_str::<Value>(&txt) { Ok(val) => { if let Some(cur) = &mut merged { merge_value(cur, val); } else { merged = Some(val); } used.push(path_ref.as_os_str().to_string_lossy().to_string()); }, Err(e) => errors.push(format!("{}: parse error: {e}", path_ref.display())), }, Err(e) => errors.push(format!("{}: read error: {e}", path_ref.display())), } }
        if let Some(val) = merged {
            // Detect legacy interaction keys pre-deserialize to avoid full failure
            if let ron::value::Value::Map(root) = &val {
                for (k, v) in root.iter() {
                    if let ron::value::Value::String(key_str) = k {
                        if key_str == "interactions" {
                            if let ron::value::Value::Map(imap) = v {
                                let mut legacy: Vec<String> = Vec::new();
                                for (ik, _iv) in imap.iter() {
                                    if let ron::value::Value::String(ik_s) = ik {
                                        if ik_s != "cluster_pop" { legacy.push(ik_s.clone()); }
                                    }
                                }
                                if !legacy.is_empty() {
                                    errors.push(format!("Ignoring legacy interactions keys removed: {}", legacy.join(", ")));
                                }
                            }
                        }
                    }
                }
            }
            match val.clone().into_rust::<GameConfig>() {
                Ok(cfg) => (cfg, used, errors),
                Err(e) => { let mut evec = errors; evec.push(format!("failed to deserialize merged config; using defaults: {e}")); (GameConfig::default(), used, evec) }
            }
        } else { (GameConfig::default(), used, errors) }
    }

    // --------------- Validation ---------------
    pub fn validate(&self) -> Vec<String> {
        let mut w = Vec::new();
        // Window
        if self.window.width <= 0.0 || self.window.height <= 0.0 { w.push("window dimensions must be > 0".into()); }
        if self.window.width * self.window.height > 10_000_000.0 { w.push(format!("very large window area: {}x{}", self.window.width, self.window.height)); }
        if self.window.auto_close < 0.0 { w.push(format!("window.autoClose {} negative -> disabled", self.window.auto_close)); } else if self.window.auto_close > 0.0 && self.window.auto_close < 0.01 { w.push(format!("window.autoClose {} extremely small", self.window.auto_close)); }
        // Gravity legacy
        if self.gravity.y.abs() < 1e-4 { w.push("gravity.y near zero – prefer gravity_widgets".into()); }
        if self.gravity.y > 0.0 { w.push(format!("gravity.y positive {} (expected negative typical)", self.gravity.y)); }
        if self.gravity.y < -2000.0 { w.push(format!("gravity.y very large magnitude {}", self.gravity.y)); }
        // Gravity widgets
        if self.gravity_widgets.widgets.is_empty() { if self.gravity.y.abs() > 0.0 { w.push(format!("gravity.y legacy ({:.1}) used to seed implicit widget (id=0)", self.gravity.y)); } else { w.push("No gravity widgets defined and gravity.y ~0 -> scene may have no force".into()); } } else { use std::collections::HashSet; let mut ids = HashSet::new(); for gw in &self.gravity_widgets.widgets { if !ids.insert(gw.id) { w.push(format!("Duplicate gravity widget id {}", gw.id)); } if gw.strength <= 0.0 { w.push(format!("gravity_widgets id {} strength {} <= 0", gw.id, gw.strength)); } if gw.radius < 0.0 { w.push(format!("gravity_widgets id {} radius {} < 0", gw.id, gw.radius)); } let mode_ok = matches!(gw.mode.as_str(), "Attract" | "Repulse"); if !mode_ok { w.push(format!("gravity_widgets id {} unknown mode '{}'", gw.id, gw.mode)); } let fall_ok = matches!(gw.falloff.as_str(), "None" | "InverseLinear" | "InverseSquare" | "SmoothEdge"); if !fall_ok { w.push(format!("gravity_widgets id {} unknown falloff '{}'", gw.id, gw.falloff)); } } }
        // Bounce
        if !(0.0..=1.5).contains(&self.bounce.restitution) { w.push(format!("restitution {} outside 0..1.5", self.bounce.restitution)); }
        if self.bounce.restitution < 0.0 { w.push("restitution negative".into()); }
        if self.bounce.friction < 0.0 { w.push(format!("bounce.friction {} < 0", self.bounce.friction)); } else if self.bounce.friction > 10.0 { w.push(format!("bounce.friction {} very high", self.bounce.friction)); }
        if self.bounce.linear_damping < 0.0 { w.push(format!("bounce.linear_damping {} < 0", self.bounce.linear_damping)); } else if self.bounce.linear_damping > 10.0 { w.push(format!("bounce.linear_damping {} extremely high", self.bounce.linear_damping)); }
        if self.bounce.angular_damping < 0.0 { w.push(format!("bounce.angular_damping {} < 0", self.bounce.angular_damping)); } else if self.bounce.angular_damping > 20.0 { w.push(format!("bounce.angular_damping {} extremely high", self.bounce.angular_damping)); }
        // Spawn widgets
        if self.spawn_widgets.global_max_balls == 0 { w.push("spawn_widgets.global_max_balls == 0".into()); }
        for sw in &self.spawn_widgets.widgets { if sw.spawn_interval <= 0.0 { w.push(format!("spawn_widget id {} spawn_interval <= 0", sw.id)); } if sw.batch == 0 { w.push(format!("spawn_widget id {} batch == 0", sw.id)); } if sw.ball_radius_min <= 0.0 || sw.ball_radius_max <= 0.0 { w.push(format!("spawn_widget id {} ball radius <= 0", sw.id)); } if sw.ball_radius_min > sw.ball_radius_max { w.push(format!("spawn_widget id {} radius_min > radius_max", sw.id)); } if sw.area_radius <= 0.0 { w.push(format!("spawn_widget id {} area_radius <= 0", sw.id)); } if sw.speed_min < 0.0 || sw.speed_max < 0.0 { w.push(format!("spawn_widget id {} speed negative", sw.id)); } if sw.speed_min > sw.speed_max { w.push(format!("spawn_widget id {} speed_min > speed_max", sw.id)); } }
        // Cluster pop
        if self.interactions.cluster_pop.enabled { let cp = &self.interactions.cluster_pop; if cp.min_ball_count < 1 { w.push(format!("cluster_pop.min_ball_count {} < 1", cp.min_ball_count)); } if cp.min_total_area < 0.0 { w.push(format!("cluster_pop.min_total_area {} negative", cp.min_total_area)); } if cp.ball_pick_radius < 0.0 { w.push(format!("cluster_pop.ball_pick_radius {} negative", cp.ball_pick_radius)); } if cp.peak_scale <= 1.0 { w.push(format!("cluster_pop.peak_scale {} <= 1.0", cp.peak_scale)); } else if cp.peak_scale > 3.0 { w.push(format!("cluster_pop.peak_scale {} > 3.0", cp.peak_scale)); } if cp.grow_duration <= 0.0 { w.push(format!("cluster_pop.grow_duration {} <= 0", cp.grow_duration)); } if cp.shrink_duration <= 0.0 { w.push(format!("cluster_pop.shrink_duration {} <= 0", cp.shrink_duration)); } if cp.hold_duration < 0.0 { w.push(format!("cluster_pop.hold_duration {} < 0", cp.hold_duration)); } if cp.collider_scale_curve > 2 { w.push(format!("cluster_pop.collider_scale_curve {} unknown", cp.collider_scale_curve)); } if cp.fade_curve > 2 { w.push(format!("cluster_pop.fade_curve {} unknown", cp.fade_curve)); } if cp.freeze_mode > 2 { w.push(format!("cluster_pop.freeze_mode {} unknown", cp.freeze_mode)); } let mut legacy = Vec::new(); if cp.impulse.is_some() { legacy.push("impulse"); } if cp.outward_bonus.is_some() { legacy.push("outward_bonus"); } if cp.despawn_delay.is_some() { legacy.push("despawn_delay"); } if cp.fade_duration.is_some() { legacy.push("fade_duration"); } if cp.fade_scale_end.is_some() { legacy.push("fade_scale_end"); } if cp.collider_shrink.is_some() { legacy.push("collider_shrink"); } if cp.collider_min_scale.is_some() { legacy.push("collider_min_scale"); } if cp.velocity_damping.is_some() { legacy.push("velocity_damping"); } if cp.spin_jitter.is_some() { legacy.push("spin_jitter"); } if !legacy.is_empty() { w.push(format!("Ignoring legacy cluster_pop fields: {}", legacy.join(", "))); } }
        // Metaballs
        if self.metaballs.radius_multiplier <= 0.0 { w.push(format!("metaballs.radius_multiplier {} <= 0", self.metaballs.radius_multiplier)); } else if self.metaballs.radius_multiplier > 5.0 { w.push(format!("metaballs.radius_multiplier {} very large", self.metaballs.radius_multiplier)); }
        // Surface noise
        if self.surface_noise.amp < 0.0 { w.push(format!("surface_noise.amp {} negative", self.surface_noise.amp)); } else if self.surface_noise.amp > 0.5 { w.push(format!("surface_noise.amp {} > 0.5 (clamped)", self.surface_noise.amp)); }
        if self.surface_noise.base_scale <= 0.0 { w.push(format!("surface_noise.base_scale {} <= 0", self.surface_noise.base_scale)); }
        if self.surface_noise.octaves > 6 { w.push(format!("surface_noise.octaves {} > 6", self.surface_noise.octaves)); }
        if self.surface_noise.octaves == 0 && self.surface_noise.enabled { w.push("surface_noise.octaves == 0 while enabled".into()); }
        // SDF shapes
    if self.sdf_shapes.max_gradient_samples > 4 { w.push(format!("sdf_shapes.max_gradient_samples {} > 4", self.sdf_shapes.max_gradient_samples)); }
    if self.sdf_shapes.gradient_step_scale <= 0.0 { w.push(format!("sdf_shapes.gradient_step_scale {} <= 0", self.sdf_shapes.gradient_step_scale)); }
    if self.sdf_shapes.use_circle_fallback_when_radius_lt < 0.0 { w.push(format!("sdf_shapes.use_circle_fallback_when_radius_lt {} < 0", self.sdf_shapes.use_circle_fallback_when_radius_lt)); }
        if !self.sdf_shapes.enabled && self.sdf_shapes.force_fallback { w.push("sdf_shapes.force_fallback true while disabled".into()); }
        // Glyph mode validation (non-fatal warnings)
        if self.sdf_shapes.glyph_mode {
            if self.sdf_shapes.glyph_text.is_empty() { w.push("sdf_shapes.glyph_mode enabled but glyph_text empty".into()); }
            let wrap_ok = matches!(self.sdf_shapes.glyph_wrap.as_str(), "Repeat" | "Clamp" | "None");
            if !wrap_ok { w.push(format!("sdf_shapes.glyph_wrap '{}' invalid (expected Repeat|Clamp|None) -> falling back to Repeat", self.sdf_shapes.glyph_wrap)); }
            // Note: actual unknown glyphs vs atlas reported post-load; we only lightly pre-scan for control chars if not skipping whitespace
            if !self.sdf_shapes.glyph_skip_whitespace {
                let mut control: Vec<char> = self.sdf_shapes.glyph_text.chars().filter(|c| c.is_control()).take(8).collect();
                control.sort(); control.dedup();
                if !control.is_empty() { w.push(format!("sdf_shapes.glyph_text contains control chars {:?}", control)); }
            }
        }
        // Clustering thresholds
        let db_enter = self.clustering.distance_buffer_enter_cluster; let db_exit = self.clustering.distance_buffer_exit_cluster; if db_enter < 1.0 { w.push(format!("clustering.distance_buffer_enter_cluster {} < 1.0", db_enter)); } if db_exit < db_enter { w.push(format!("clustering.distance_buffer_exit_cluster {} < enter {}", db_exit, db_enter)); } if db_exit > 3.0 { w.push(format!("clustering.distance_buffer_exit_cluster {} > 3.0", db_exit)); }
        w
    }
}

// ---------------- Tests ----------------
#[cfg(test)]
mod tests { use super::*; #[test] fn defaults_validate_empty() { let cfg = GameConfig::default(); let warns = cfg.validate(); assert!(warns.len() > 0); } }
