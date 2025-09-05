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

    pub struct DiskLevelSource {
        base: PathBuf,
        ids: Vec<&'static str>,
        default: &'static str,
        _live: bool,
    }

    impl DiskLevelSource {
        pub fn new(live: bool) -> Self {
            // Hard-code recognized ids (registry file `levels.ron` currently NOT consulted in disk mode).
            // Order matters: first entry becomes the default. Keep this list in sync with assets/levels.
            // TODO: optionally parse levels.ron to remove duplication.
            let ids: Vec<&'static str> = vec!["menu", "test_layout"]; // Extend here for new disk-only levels
                                                              // Duplicate detection not necessary for literals but retain pattern for future extension.
            use std::collections::HashSet;
            let mut seen = HashSet::new();
            for id in &ids {
                if !seen.insert(*id) {
                    warn!(
                        target = "level",
                        "DiskLevelSource: duplicate id '{}' (first wins)", id
                    );
                }
            }
            let default = ids[0];
            let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
            let base = PathBuf::from(crate_root).join("assets").join("levels");
            Self {
                base,
                ids,
                default,
                _live: live,
            }
        }
        fn path_pair(&self, id: &str) -> (PathBuf, PathBuf) {
            let layout = self.base.join(id).join("layout.ron");
            let widgets = self.base.join(id).join("widgets.ron");
            (layout, widgets)
        }
    }

    impl super::LevelSource for DiskLevelSource {
        fn list_ids(&self) -> &[&'static str] {
            &self.ids
        }
        fn default_id(&self) -> &str {
            self.default
        }
        fn get_level_owned(&self, id: &str) -> Result<(String, String), String> {
            let chosen = if self.ids.contains(&id) {
                id
            } else {
                warn!(
                    target = "level",
                    "DiskLevelSource: requested id '{}' unknown; falling back to default '{}'",
                    id,
                    self.default
                );
                self.default
            };
            let (layout_path, widgets_path) = self.path_pair(chosen);
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
