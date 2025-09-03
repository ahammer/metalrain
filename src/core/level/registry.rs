#![allow(dead_code)]
// Deprecated: replaced by embedded_levels LevelSource abstraction (kept for reference during migration)
#[deprecated(note = "Replaced by embedded_levels LevelSource abstraction; no longer used in loader.")]
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub struct LevelEntry {
    pub id: String,
    pub layout: String,
    pub widgets: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LevelRegistry {
    pub version: u32,
    pub default: String,
    pub list: Vec<LevelEntry>,
}

impl LevelRegistry {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let txt = fs::read_to_string(&path).map_err(|e| format!("read registry {:?}: {e}", path.as_ref()))?;
        let reg: LevelRegistry = ron::from_str(&txt).map_err(|e| format!("parse registry {:?}: {e}", path.as_ref()))?;
        if reg.version != 1 {
            return Err(format!("LevelRegistry version {} unsupported (expected 1)", reg.version));
        }
        if reg.list.is_empty() {
            return Err("LevelRegistry list empty".into());
        }
        Ok(reg)
    }

    pub fn select_level(&self, requested: Option<&str>) -> Result<LevelEntry, String> {
        let mut id = requested.map(|s| s.to_string());
        if id.as_deref().is_none() || id.as_deref().unwrap().is_empty() {
            id = Some(self.default.clone());
        }
        let sel = id.unwrap();
        if let Some(found) = self.list.iter().find(|e| e.id == sel) {
            return Ok(found.clone());
        }
        // Fallback to default if requested not found
        if let Some(found) = self.list.iter().find(|e| e.id == self.default) {
            return Ok(found.clone());
        }
        Err(format!("Requested level '{sel}' not found and default '{}' missing.", self.default))
    }
}

/// Determine selected level id via (precedence):
///  1. CLI args: --level <id>
///  2. Env var: LEVEL_ID
///  3. Registry default
pub fn resolve_requested_level_id() -> Option<String> {
    // CLI scan (very small / naive)
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--level" {
            if let Some(id) = args.next() {
                if !id.trim().is_empty() {
                    return Some(id);
                }
            }
        }
    }
    // Env
    if let Ok(val) = std::env::var("LEVEL_ID") {
        if !val.trim().is_empty() {
            return Some(val);
        }
    }
    None
}
