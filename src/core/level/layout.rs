use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub struct Vec2Def { pub x: f32, pub y: f32 }
impl From<Vec2Def> for Vec2 {
    fn from(v: Vec2Def) -> Self { Vec2::new(v.x, v.y) }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SegmentDef { pub from: Vec2Def, pub to: Vec2Def, pub thickness: f32 }

#[derive(Debug, Deserialize, Clone)]
pub struct WallDef { pub segment: SegmentDef }

// ------------------------------ Timelines / Groups (v2) ------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct TimelineKeyF32 { pub t: f32, pub value: f32 }

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum RepeatModeDef { Once, Loop, PingPong }
impl Default for RepeatModeDef { fn default() -> Self { Self::Loop } }

#[derive(Debug, Deserialize, Clone)]
pub struct TransformTimelineDef {
    pub name: String,
    pub duration: f32,
    #[serde(default)]
    pub repeat: RepeatModeDef,
    #[serde(default)]
    pub rotation: Vec<TimelineKeyF32>, // radians
    #[serde(default)]
    pub scale: Vec<TimelineKeyF32>,    // uniform
}

#[derive(Debug, Deserialize, Clone)]
pub struct WallGroupDef {
    pub name: String,
    pub pivot: Vec2Def,
    pub walls: Vec<WallDef>,
    #[serde(default)]
    pub timeline: Option<TransformTimelineDef>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutFile {
    pub version: u32,
    #[serde(default)]
    pub walls: Vec<WallDef>,             // legacy v1 + still allowed in v2
    #[serde(default)]
    pub groups: Vec<WallGroupDef>,       // v2 optional
}

#[derive(Debug, Clone)]
pub struct WallSegment {
    pub from: Vec2,
    pub to: Vec2,
    pub thickness: f32,
}

impl LayoutFile {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let txt = fs::read_to_string(&path).map_err(|e| format!("read layout {:?}: {e}", path.as_ref()))?;
        let lf: LayoutFile = ron::from_str(&txt).map_err(|e| format!("parse layout {:?}: {e}", path.as_ref()))?;
        if lf.version != 1 && lf.version != 2 {
            return Err(format!("LayoutFile version {} unsupported (expected 1 or 2)", lf.version));
        }
        Ok(lf)
    }

    pub fn to_wall_segments(&self) -> Vec<WallSegment> {
        let mut out = Vec::with_capacity(self.walls.len());
        for w in &self.walls {
            let seg = &w.segment;
            let from: Vec2 = seg.from.clone().into();
            let to: Vec2 = seg.to.clone().into();
            out.push(WallSegment { from, to, thickness: seg.thickness });
        }
        out
    }
}

// (Removed custom deserializer; default Option parsing retained.)
