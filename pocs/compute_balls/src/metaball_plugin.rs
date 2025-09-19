//! Unified plugin wrapping compute, simulation, and presentation of metaballs.
//!
//! Consumers can add `MetaballRendererPlugin::default()` to their `App` to get:
//! - GPU compute pipeline populating a 16F field texture + RGBA8 albedo
//! - Simple bouncy-ball CPU simulation feeding per-ball data (position, radius, color)
//! - Fullscreen quad presenting the compute textures via a material shader
//!
//! The internal modules (`compute`, `systems` / simulation, `present`) remain available
//! for custom composition if finer control is required.
//!
//! NOTE: A reusable extracted library version of this renderer now lives in
//! `crates/metaball_renderer` providing a `MetaballRendererPlugin` that consumes
//! `MetaBall` ECS components for rendering. This POC is kept intact for historical
//! reference per extraction prompt (2025-09-19).

use bevy::prelude::*;

use crate::{
    compute::ComputeMetaballsPlugin,
    present::MetaballDisplayPlugin,
    systems::MetaballSimulationPlugin,
};

/// High-level feature flags for optional subsystems.
#[derive(Clone)]
pub struct MetaballRendererSettings {
    /// Include the built-in CPU bouncy-ball simulation feeding the GPU.
    pub with_simulation: bool,
    /// Include the fullscreen presentation material + camera.
    pub with_present: bool,
}

impl Default for MetaballRendererSettings {
    fn default() -> Self { Self { with_simulation: true, with_present: true } }
}

/// Public plugin a downstream Bevy app can add to enable metaball rendering.
#[derive(Default)]
pub struct MetaballRendererPlugin {
    pub settings: MetaballRendererSettings,
}

impl MetaballRendererPlugin {
    pub fn with_settings(settings: MetaballRendererSettings) -> Self { Self { settings } }
}

impl Plugin for MetaballRendererPlugin {
    fn build(&self, app: &mut App) {
        // Always add compute layer.
        app.add_plugins(ComputeMetaballsPlugin);

        if self.settings.with_simulation {
            app.add_plugins(MetaballSimulationPlugin);
        }
        if self.settings.with_present {
            app.add_plugins(MetaballDisplayPlugin);
        }
    }
}
