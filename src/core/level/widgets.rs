use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::core::config::config::{GravityWidgetConfig, SpawnWidgetConfig};

#[derive(Debug, Deserialize, Clone)]
pub struct Vec2Def {
    pub x: f32,
    pub y: f32,
}
impl From<Vec2Def> for Vec2 {
    fn from(v: Vec2Def) -> Self {
        Vec2::new(v.x, v.y)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RangeF32 {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnSpecRaw {
    pub interval: f32,
    pub batch: u32,
    #[serde(rename = "area_radius")]
    pub area_radius: f32,
    pub ball_radius: RangeF32,
    pub speed: RangeF32,
}

/// Radius definition that can be either a single scalar (legacy Attractor) or a range (TextSpawn usage).
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum RadiusDef {
    Single(f32),
    Range { min: f32, max: f32 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct WidgetDef {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: u32,
    pub pos: Vec2Def,

    // SpawnPoint-specific (inside nested 'spawn' block)
    #[serde(default)]
    pub spawn: Option<SpawnSpecRaw>,

    // Attractor-specific & (overloaded) TextSpawn: unified radius field supporting number or range.
    #[serde(default)]
    pub strength: Option<f32>,
    #[serde(default)]
    pub radius: Option<RadiusDef>,
    #[serde(default)]
    pub falloff: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,

    // ----------------- TextSpawn optional raw fields (all serde default) -----------------
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub font_px: Option<u32>,
    #[serde(default)]
    pub cell: Option<f32>,
    #[serde(default)]
    pub jitter: Option<f32>,
    // NOTE: radius range reuses `radius` field (Range variant) – kept for schema parity with prompt.
    #[serde(default)]
    pub speed: Option<RangeF32>,
    #[serde(default)]
    pub attraction_strength: Option<f32>,
    #[serde(default)]
    pub attraction_damping: Option<f32>,
    #[serde(default)]
    pub snap_distance: Option<f32>,
    #[serde(default)]
    pub color_mode: Option<String>,
    #[serde(default)]
    pub word_colors: Option<Vec<usize>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WidgetsFile {
    pub version: u32,
    pub widgets: Vec<WidgetDef>,
}

impl WidgetsFile {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let txt = fs::read_to_string(&path)
            .map_err(|e| format!("read widgets {:?}: {e}", path.as_ref()))?;
        let wf: WidgetsFile =
            ron::from_str(&txt).map_err(|e| format!("parse widgets {:?}: {e}", path.as_ref()))?;
        if wf.version != 1 {
            return Err(format!(
                "WidgetsFile version {} unsupported (expected 1)",
                wf.version
            ));
        }
        Ok(wf)
    }
}

// -------------------------------- Runtime Specs --------------------------------

#[derive(Debug, Clone)]
pub struct SpawnPointSpec {
    pub id: u32,
    pub pos: Vec2,
    pub interval: f32,
    pub batch: u32,
    pub area_radius: f32,
    pub ball_radius_min: f32,
    pub ball_radius_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub enabled: bool,
}

impl SpawnPointSpec {
    pub fn to_config(&self) -> SpawnWidgetConfig {
        // NOTE: SpawnWidgetConfig currently has no position; the spawn system will be adapted
        // later to use per-widget positions. For now we only map timing & ranges.
        SpawnWidgetConfig {
            id: self.id,
            enabled: self.enabled,
            spawn_interval: self.interval,
            batch: self.batch as usize,
            area_radius: self.area_radius,
            ball_radius_min: self.ball_radius_min,
            ball_radius_max: self.ball_radius_max,
            speed_min: self.speed_min,
            speed_max: self.speed_max,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalloffKind {
    None,
    InverseLinear,
    InverseSquare,
    SmoothEdge,
}
impl FalloffKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "None" => Some(Self::None),
            "InverseLinear" => Some(Self::InverseLinear),
            "InverseSquare" => Some(Self::InverseSquare),
            "SmoothEdge" => Some(Self::SmoothEdge),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::InverseLinear => "InverseLinear",
            Self::InverseSquare => "InverseSquare",
            Self::SmoothEdge => "SmoothEdge",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AttractorSpec {
    pub id: u32,
    pub pos: Vec2,
    pub strength: f32,
    pub radius: f32,
    pub falloff: FalloffKind,
    pub enabled: bool,
}

impl AttractorSpec {
    pub fn to_config(&self) -> GravityWidgetConfig {
        // GravityWidgetConfig currently lacks explicit position; spawned plugin will position sequentially.
        // A later revision will extend config & plugin to use provided coordinates.
        GravityWidgetConfig {
            id: self.id,
            strength: self.strength,
            mode: "Attract".into(),
            radius: self.radius,
            falloff: self.falloff.as_str().into(),
            enabled: self.enabled,
            physics_collider: false,
            _parsed_ok: true,
        }
    }
}

// ----------------------------- Conversion / Extraction -----------------------------

#[derive(Debug, Default)]
pub struct ExtractedWidgets {
    pub spawn_points: Vec<SpawnPointSpec>,
    pub attractors: Vec<AttractorSpec>,
    pub text_spawns: Vec<TextSpawnSpec>,
    pub warnings: Vec<String>,
}

// -------------------------------- TextSpawn Runtime Spec --------------------------------

/// Text color selection strategy for spawned letter balls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextColorMode {
    RandomPerBall,
    WordSolid,
    Single,
}

/// Runtime specification for a TextSpawn widget extracted from RON data.
#[derive(Debug, Clone)]
pub struct TextSpawnSpec {
    pub id: u32,
    pub pos: Vec2,
    pub text: String,
    pub font_px: u32,
    pub cell: f32,
    pub jitter: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub attraction_strength: f32,
    pub attraction_damping: f32,
    pub snap_distance: f32,
    pub color_mode: TextColorMode,
    pub word_palette_indices: Vec<usize>,
}

pub fn extract_widgets(file: &WidgetsFile) -> ExtractedWidgets {
    use std::collections::HashSet;
    let mut out = ExtractedWidgets::default();
    let mut seen_ids: HashSet<u32> = HashSet::new();
    for w in &file.widgets {
        if !seen_ids.insert(w.id) {
            out.warnings.push(format!(
                "LevelLoader: duplicate widget id {}, skipping subsequent occurrence.",
                w.id
            ));
            continue;
        }
        match w.kind.as_str() {
            "SpawnPoint" => {
                if let Some(spawn) = &w.spawn {
                    let mut interval = spawn.interval;
                    if interval <= 0.0 {
                        out.warnings.push(format!(
                            "LevelLoader: spawn point id {} interval {} <= 0 clamped.",
                            w.id, interval
                        ));
                        interval = 0.05;
                    }
                    let (mut br_min, mut br_max) = (spawn.ball_radius.min, spawn.ball_radius.max);
                    if br_min > br_max {
                        out.warnings.push(format!(
                            "LevelLoader: spawn point id {} ball_radius min {} > max {}, swapped.",
                            w.id, br_min, br_max
                        ));
                        std::mem::swap(&mut br_min, &mut br_max);
                    }
                    let (mut sp_min, mut sp_max) = (spawn.speed.min, spawn.speed.max);
                    if sp_min > sp_max {
                        out.warnings.push(format!(
                            "LevelLoader: spawn point id {} speed min {} > max {}, swapped.",
                            w.id, sp_min, sp_max
                        ));
                        std::mem::swap(&mut sp_min, &mut sp_max);
                    }
                    out.spawn_points.push(SpawnPointSpec {
                        id: w.id,
                        pos: w.pos.clone().into(),
                        interval,
                        batch: spawn.batch.max(1),
                        area_radius: spawn.area_radius.max(0.0),
                        ball_radius_min: br_min.max(0.01),
                        ball_radius_max: br_max.max(0.01),
                        speed_min: sp_min.max(0.0),
                        speed_max: sp_max.max(0.0),
                        enabled: true,
                    });
                } else {
                    out.warnings.push(format!(
                        "LevelLoader: SpawnPoint id {} missing 'spawn' block.",
                        w.id
                    ));
                }
            }
            "Attractor" => {
                let strength = w.strength.unwrap_or(0.0);
                let mut s = strength;
                if s < 0.0 {
                    out.warnings.push(format!(
                        "LevelLoader: Attractor id {} negative strength {} clamped to 0.",
                        w.id, s
                    ));
                    s = 0.0;
                }
                // Accept either scalar or range for backward compatibility (range -> use max as radius, warn).
                let radius = match w.radius.clone() {
                    Some(RadiusDef::Single(v)) => v,
                    Some(RadiusDef::Range { min, max }) => {
                        out.warnings.push(format!(
                            "LevelLoader: Attractor id {} provided radius range ({}, {}) – using max as radius.",
                            w.id, min, max
                        ));
                        max
                    }
                    None => 0.0,
                };
                let falloff_raw = w.falloff.clone().unwrap_or_else(|| "InverseLinear".into());
                let falloff = FalloffKind::parse(&falloff_raw).unwrap_or_else(|| {
                    out.warnings.push(format!("LevelLoader: Attractor id {} unknown falloff '{}'; defaulting InverseLinear.", w.id, falloff_raw));
                    FalloffKind::InverseLinear
                });
                let enabled = w.enabled.unwrap_or(true);
                out.attractors.push(AttractorSpec {
                    id: w.id,
                    pos: w.pos.clone().into(),
                    strength: s,
                    radius,
                    falloff,
                    enabled,
                });
            }
                "TextSpawn" => {
                    // Empty or missing text -> warn & skip
                    let text_raw = w.text.clone().unwrap_or_default();
                    if text_raw.trim().is_empty() {
                        out.warnings.push(format!(
                            "LevelLoader: TextSpawn id {} has empty text; skipped.",
                            w.id
                        ));
                        continue;
                    }

                    // Defaults per prompt
                    let font_px = w.font_px.unwrap_or(140).max(1);
                    let mut cell = w.cell.unwrap_or(14.0);
                    if cell <= 1.0 {
                        out.warnings.push(format!(
                            "LevelLoader: TextSpawn id {} cell {} <= 1.0 adjusted to 8.0.",
                            w.id, cell
                        ));
                        cell = 8.0;
                    }
                    let jitter = w.jitter.unwrap_or(42.0).max(0.0);

                    // Radius range sourced from `radius` field Range variant; fallback to default 7..13
                    let (mut rmin, mut rmax) = match w.radius.clone() {
                        Some(RadiusDef::Range { min, max }) => (min, max),
                        Some(RadiusDef::Single(v)) => (v, v),
                        None => (7.0, 13.0),
                    };
                    if rmin > rmax {
                        out.warnings.push(format!(
                            "LevelLoader: TextSpawn id {} radius min {} > max {} swapped.",
                            w.id, rmin, rmax
                        ));
                        std::mem::swap(&mut rmin, &mut rmax);
                    }
                    if rmin <= 0.0 {
                        rmin = 0.01;
                    }
                    if rmax <= 0.0 {
                        rmax = 0.01;
                    }

                    let (mut smin, mut smax) = match w.speed.clone() {
                        Some(r) => (r.min, r.max),
                        None => (0.0, 50.0),
                    };
                    if smin > smax {
                        out.warnings.push(format!(
                            "LevelLoader: TextSpawn id {} speed min {} > max {} swapped.",
                            w.id, smin, smax
                        ));
                        std::mem::swap(&mut smin, &mut smax);
                    }
                    if smin < 0.0 {
                        smin = 0.0;
                    }
                    if smax < 0.0 {
                        smax = 0.0;
                    }

                    let attraction_strength = w.attraction_strength.unwrap_or(60.0).max(0.0);
                    let attraction_damping = w.attraction_damping.unwrap_or(6.5).max(0.0);
                    let snap_distance = w.snap_distance.unwrap_or(3.2).max(0.0);

                    let color_mode_raw = w.color_mode.clone().unwrap_or_else(|| "RandomPerBall".into());
                    let color_mode = match color_mode_raw.as_str() {
                        "RandomPerBall" => TextColorMode::RandomPerBall,
                        "WordSolid" => TextColorMode::WordSolid,
                        "Single" => TextColorMode::Single,
                        other => {
                            out.warnings.push(format!(
                                "LevelLoader: TextSpawn id {} unknown color_mode '{}'; defaulting RandomPerBall.",
                                w.id, other
                            ));
                            TextColorMode::RandomPerBall
                        }
                    };
                    let word_palette_indices = w.word_colors.clone().unwrap_or_default();

                    out.text_spawns.push(TextSpawnSpec {
                        id: w.id,
                        pos: w.pos.clone().into(),
                        text: text_raw,
                        font_px,
                        cell,
                        jitter,
                        radius_min: rmin,
                        radius_max: rmax,
                        speed_min: smin,
                        speed_max: smax,
                        attraction_strength,
                        attraction_damping,
                        snap_distance,
                        color_mode,
                        word_palette_indices,
                    });
                }
            other => {
                out.warnings.push(format!(
                    "LevelLoader: unknown widget type '{}' (id {}) skipped.",
                    other, w.id
                ));
            }
        }
    }
    out
}
