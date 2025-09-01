use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

use crate::core::config::config::{GravityWidgetConfig, SpawnWidgetConfig};

#[derive(Debug, Deserialize, Clone)]
pub struct Vec2Def { pub x: f32, pub y: f32 }
impl From<Vec2Def> for Vec2 {
    fn from(v: Vec2Def) -> Self { Vec2::new(v.x, v.y) }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RangeF32 { pub min: f32, pub max: f32 }

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnSpecRaw {
    pub interval: f32,
    pub batch: u32,
    #[serde(rename = "area_radius")]
    pub area_radius: f32,
    pub ball_radius: RangeF32,
    pub speed: RangeF32,
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

    // Attractor-specific
    #[serde(default)]
    pub strength: Option<f32>,
    #[serde(default)]
    pub radius: Option<f32>,
    #[serde(default)]
    pub falloff: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WidgetsFile {
    pub version: u32,
    pub widgets: Vec<WidgetDef>,
}

impl WidgetsFile {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let txt = fs::read_to_string(&path).map_err(|e| format!("read widgets {:?}: {e}", path.as_ref()))?;
        let wf: WidgetsFile = ron::from_str(&txt).map_err(|e| format!("parse widgets {:?}: {e}", path.as_ref()))?;
        if wf.version != 1 {
            return Err(format!("WidgetsFile version {} unsupported (expected 1)", wf.version));
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
pub enum FalloffKind { None, InverseLinear, InverseSquare, SmoothEdge }
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
    pub warnings: Vec<String>,
}

pub fn extract_widgets(file: &WidgetsFile) -> ExtractedWidgets {
    use std::collections::HashSet;
    let mut out = ExtractedWidgets::default();
    let mut seen_ids: HashSet<u32> = HashSet::new();
    for w in &file.widgets {
        if !seen_ids.insert(w.id) {
            out.warnings.push(format!("LevelLoader: duplicate widget id {}, skipping subsequent occurrence.", w.id));
            continue;
        }
        match w.kind.as_str() {
            "SpawnPoint" => {
                if let Some(spawn) = &w.spawn {
                    let mut interval = spawn.interval;
                    if interval <= 0.0 {
                        out.warnings.push(format!("LevelLoader: spawn point id {} interval {} <= 0 clamped.", w.id, interval));
                        interval = 0.05;
                    }
                    let (mut br_min, mut br_max) = (spawn.ball_radius.min, spawn.ball_radius.max);
                    if br_min > br_max {
                        out.warnings.push(format!("LevelLoader: spawn point id {} ball_radius min {} > max {}, swapped.", w.id, br_min, br_max));
                        std::mem::swap(&mut br_min, &mut br_max);
                    }
                    let (mut sp_min, mut sp_max) = (spawn.speed.min, spawn.speed.max);
                    if sp_min > sp_max {
                        out.warnings.push(format!("LevelLoader: spawn point id {} speed min {} > max {}, swapped.", w.id, sp_min, sp_max));
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
                    out.warnings.push(format!("LevelLoader: SpawnPoint id {} missing 'spawn' block.", w.id));
                }
            }
            "Attractor" => {
                let strength = w.strength.unwrap_or(0.0);
                let mut s = strength;
                if s < 0.0 {
                    out.warnings.push(format!("LevelLoader: Attractor id {} negative strength {} clamped to 0.", w.id, s));
                    s = 0.0;
                }
                let radius = w.radius.unwrap_or(0.0);
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
            other => {
                out.warnings.push(format!("LevelLoader: unknown widget type '{}' (id {}) skipped.", other, w.id));
            }
        }
    }
    out
}
