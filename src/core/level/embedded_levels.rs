//! Embedded / Disk dual-mode level sourcing.
//!
//! This module defines the abstraction used by the level loader to obtain
//! level layout + widget RON contents either from compile-time embedded strings
//! (via `include_str!`) or from disk (native development workflow).
//!
//! Compilation Mode Summary:
//! - `wasm32` target: always uses embedded levels (no runtime FS IO) regardless of feature flags.
//! - Native + feature `embedded_levels`: forces embedded mode (mirrors wasm behavior).
//! - Native default (no feature): disk mode.
//! - Native with `live_levels` (and NOT embedded): disk mode + Live stub logging.
//!
//! Adding a new embedded level requires editing ONLY this file: add new
//! `const` include_str! bindings and append an `EmbeddedLevel` entry to
//! `EMBEDDED_LEVELS`.

use bevy::prelude::*; // For logging macros only

/// A single embedded level definition composed of static string slices.
pub struct EmbeddedLevel {
    pub id: &'static str,
    pub layout_ron: &'static str,
    pub widgets_ron: &'static str,
}

/// Runtime selected mode (for log output & conditional logic higher layers may add later).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSourceMode {
    Embedded,
    Disk,
    DiskLive,
}

/// Abstract level source.
///
/// Embedded builds expose `get_level` returning static slices; disk builds expose
/// `get_level_owned` returning owned Strings. We use cfg to keep the trait surface minimal per build.
pub trait LevelSource {
    fn list_ids(&self) -> &[&'static str];
    fn default_id(&self) -> &str;
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    fn get_level(&self, id: &str) -> Result<(&'static str, &'static str), String>;
    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    fn get_level_owned(&self, id: &str) -> Result<(String, String), String>;
}

// -------------------------------------------------------------------------------------------------
// Embedded Implementation
// -------------------------------------------------------------------------------------------------

#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
mod embedded_impl {
    use super::*;

    // NOTE: path traversal uses relative path from this source file's directory.
    // ../../../ assets/levels/<id>/
    pub const TEST_LAYOUT_LAYOUT: &str =
        include_str!("../../../assets/levels/test_layout/layout.ron");
    pub const TEST_LAYOUT_WIDGETS: &str =
        include_str!("../../../assets/levels/test_layout/widgets.ron");

    pub const EMBEDDED_LEVELS: &[EmbeddedLevel] = &[EmbeddedLevel {
        id: "test_layout",
        layout_ron: TEST_LAYOUT_LAYOUT,
        widgets_ron: TEST_LAYOUT_WIDGETS,
    }];

    pub struct EmbeddedLevelSource {
        ids: &'static [&'static str],
        default: &'static str,
    }

    impl EmbeddedLevelSource {
        pub fn new() -> Self {
            // EMBEDDED_LEVELS is a compile-time const slice populated via include_str!.
            // Rely on the static definition; duplicate detection is useful for logging.
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            for l in EMBEDDED_LEVELS {
                if !seen.insert(l.id) {
                    warn!(
                        target = "level",
                        "EmbeddedLevelSource: duplicate id '{}' (first wins)", l.id
                    );
                }
            }
            let default = EMBEDDED_LEVELS[0].id; // first entry defines default id
            let ids: Vec<&'static str> = EMBEDDED_LEVELS.iter().map(|l| l.id).collect();
            // Leak small Vec to produce 'static slice; acceptable (tiny & stable at program init)
            let leaked: &'static [&'static str] = Box::leak(ids.into_boxed_slice());
            Self {
                ids: leaked,
                default,
            }
        }

        fn find(&self, id: &str) -> Option<&'static EmbeddedLevel> {
            EMBEDDED_LEVELS.iter().find(|l| l.id == id)
        }
    }

    impl Default for EmbeddedLevelSource {
        fn default() -> Self {
            EmbeddedLevelSource::new()
        }
    }

    impl super::LevelSource for EmbeddedLevelSource {
        fn list_ids(&self) -> &[&'static str] {
            self.ids
        }
        fn default_id(&self) -> &str {
            self.default
        }
        fn get_level(&self, id: &str) -> Result<(&'static str, &'static str), String> {
            if let Some(l) = self.find(id) {
                Ok((l.layout_ron, l.widgets_ron))
            } else {
                Err(format!("embedded level '{}' not found", id))
            }
        }
    }

    pub fn make_source() -> (LevelSourceMode, EmbeddedLevelSource) {
        (LevelSourceMode::Embedded, EmbeddedLevelSource::new())
    }
    pub use EmbeddedLevelSource as ActiveEmbeddedLevelSource;
}

#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
pub use embedded_impl::{make_source as make_embedded_source, ActiveEmbeddedLevelSource};

// -------------------------------------------------------------------------------------------------
// Disk Implementation
// -------------------------------------------------------------------------------------------------
#[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
mod disk_impl {
    use super::*;
    use std::{fs, path::PathBuf};

    // Single level entry (all &'static so we can satisfy trait returning &'static [&'static str])
    struct DiskLevelEntry {
        id: &'static str,
        layout_rel: &'static str,
        widgets_rel: &'static str,
    }

    pub struct DiskLevelSource {
        base: PathBuf,
        entries: Vec<DiskLevelEntry>,
        ids: Vec<&'static str>,
        default: &'static str,
        _live: bool,
    }

    impl DiskLevelSource {
        pub fn new(live: bool) -> Self {
            let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
            let base = PathBuf::from(&crate_root).join("assets").join("levels");

            // Attempt to load registry file levels.ron (version 1 schema)
            let registry_path = base.join("levels.ron");
            let (entries, default): (Vec<DiskLevelEntry>, &'static str) = match fs::read_to_string(&registry_path) {
                Ok(txt) => {
                    #[derive(serde::Deserialize)]
                    struct RegistryEntry { id: String, layout: String, widgets: String }
                    #[derive(serde::Deserialize)]
                    struct RegistryFile { version: u32, default: String, list: Vec<RegistryEntry> }
                    match ron::from_str::<RegistryFile>(&txt) {
                        Ok(reg) => {
                            if reg.version != 1 {
                                warn!(target="level", "DiskLevelSource: levels.ron version {} unsupported (expected 1); falling back to hard-coded list", reg.version);
                                Self::hardcoded_fallback()
                            } else {
                                let mut leaks: Vec<DiskLevelEntry> = Vec::new();
                                let mut found_default = false;
                                for e in reg.list {
                                    // Leak strings to get &'static str (tiny, bounded by number of levels)
                                    let id_static: &'static str = Box::leak(e.id.into_boxed_str());
                                    let layout_static: &'static str = Box::leak(e.layout.into_boxed_str());
                                    let widgets_static: &'static str = Box::leak(e.widgets.into_boxed_str());
                                    if id_static == reg.default { found_default = true; }
                                    leaks.push(DiskLevelEntry { id: id_static, layout_rel: layout_static, widgets_rel: widgets_static });
                                }
                                if leaks.is_empty() {
                                    warn!(target="level", "DiskLevelSource: registry empty; using fallback list");
                                    Self::hardcoded_fallback()
                                } else {
                                    let default_id: &'static str = if found_default { Box::leak(reg.default.into_boxed_str()) } else { leaks[0].id };
                                    (leaks, default_id)
                                }
                            }
                        }
                        Err(e) => {
                            warn!(target="level", "DiskLevelSource: parse levels.ron failed: {e}; using fallback list");
                            Self::hardcoded_fallback()
                        }
                    }
                }
                Err(e) => {
                    debug!(target="level", "DiskLevelSource: read {:?} failed: {e}; using fallback list", registry_path);
                    Self::hardcoded_fallback()
                }
            };

            // Build ids slice and duplicate detection
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            let mut ids: Vec<&'static str> = Vec::with_capacity(entries.len());
            for ent in &entries {
                if !seen.insert(ent.id) {
                    warn!(target="level", "DiskLevelSource: duplicate id '{}' (first wins)", ent.id);
                    continue;
                }
                ids.push(ent.id);
            }
            Self { base, entries, ids, default, _live: live }
        }

        // Fallback to hard-coded list (mirrors previous behavior); returns (entries, default)
        fn hardcoded_fallback() -> (Vec<DiskLevelEntry>, &'static str) {
            let mut entries: Vec<DiskLevelEntry> = Vec::new();
            for (id, layout_rel, widgets_rel) in [
                ("menu", "menu/layout.ron", "menu/widgets.ron"),
                ("test_layout", "test_layout/layout.ron", "test_layout/widgets.ron"),
            ] {
                entries.push(DiskLevelEntry { id, layout_rel, widgets_rel });
            }
            let default = entries[0].id;
            (entries, default)
        }

        fn path_pair(&self, id: &str) -> (PathBuf, PathBuf) {
            if let Some(e) = self.entries.iter().find(|e| e.id == id) {
                let layout = self.base.join(e.layout_rel);
                let widgets = self.base.join(e.widgets_rel);
                (layout, widgets)
            } else {
                // Fallback to default if id not found
                let def = self.entries.iter().find(|e| e.id == self.default).expect("default entry must exist");
                let layout = self.base.join(def.layout_rel);
                let widgets = self.base.join(def.widgets_rel);
                (layout, widgets)
            }
        }
    }

    impl super::LevelSource for DiskLevelSource {
        fn list_ids(&self) -> &[&'static str] { &self.ids }
        fn default_id(&self) -> &str { self.default }
        fn get_level_owned(&self, id: &str) -> Result<(String, String), String> {
            let (layout_path, widgets_path) = self.path_pair(id);
            let layout_txt = fs::read_to_string(&layout_path)
                .map_err(|e| format!("read layout {:?}: {e}", layout_path))?;
            let widgets_txt = fs::read_to_string(&widgets_path)
                .map_err(|e| format!("read widgets {:?}: {e}", widgets_path))?;
            Ok((layout_txt, widgets_txt))
        }
    }

    pub fn make_source(live: bool) -> (LevelSourceMode, DiskLevelSource) {
        let mode = if live {
            LevelSourceMode::DiskLive
        } else {
            LevelSourceMode::Disk
        };
        (mode, DiskLevelSource::new(live))
    }

    pub use DiskLevelSource as ActiveDiskLevelSource;
}

#[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
pub use disk_impl::{make_source as make_disk_source, ActiveDiskLevelSource};

// -------------------------------------------------------------------------------------------------
// Public factory (cfg orchestrated in loader)
// -------------------------------------------------------------------------------------------------

/// Select a level source based on compile-time cfg and feature flags. Feature conflict handling
/// (embedded + live) is performed in the loader before calling this to decide whether `live` flag is passed.
pub fn select_level_source(_live_requested: bool) -> (LevelSourceMode, Box<dyn LevelSource>) {
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    {
        let (mode, src) = embedded_impl::make_source();
        (mode, Box::new(src))
    }
    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    {
        let (mode, src) = disk_impl::make_source(_live_requested);
        (mode, Box::new(src))
    }
}
