//! Metaball Renderer Library
//!
//! Phase 2: Ported compute pipeline + (optional) presentation from the original POC.
//! Simulation / packing of `MetaBall` entities will be completed in Phase 3. For now an
//! empty internal buffer is uploaded each frame (no visible metaballs yet).
//!
//! High‑level usage (final form after Phase 3):
//! ```no_run
//! use bevy::prelude::*;
//! use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings, MetaBall};
//!
//! App::new()
//!   .add_plugins(DefaultPlugins)
//!   .add_plugins(MetaballRendererPlugin::default())
//!   .run();
//! ```
//!
//! Coordinate space (Sprint 2.1 refactor):
//! * World space (authoritative) lives inside a configurable `world_bounds` `Rect`.
//! * A `MetaballCoordinateMapper` resource maps each entity's `Transform` (XY) into
//!   metaball texture pixel space every time packing runs.
//! * `MetaBall` stores only a world‑space radius (`radius_world`); no duplicate center.
//! * Helper functions (`project_world_to_screen`, `screen_to_world`, `screen_to_metaball_uv`)
//!   are re‑exported for integration & picking.
//! * The renderer no longer spawns or owns a camera; presentation/compositing is deferred
//!   to a higher‑level pipeline (Sprint 3). Use `metaball_textures(&world)` to fetch the
//!   offscreen textures for custom composition.

use bevy::prelude::*;

mod components;
mod compute;
mod coordinates; // world <-> texture mapping & projection helpers
mod diagnostics;
mod embedded_shaders;
mod internal;
mod pack;
#[cfg(feature = "present")]
mod present; // optional fullscreen quad presentation (offscreen texture to screen)
mod settings; // logging & runtime diagnostics

pub use components::{MetaBall, MetaBallCluster, MetaBallColor};
pub use coordinates::{
    project_world_to_screen, screen_to_metaball_uv, screen_to_world, MetaballCoordinateMapper,
};
pub use diagnostics::{MetaballDiagnosticsConfig, MetaballDiagnosticsPlugin};
pub use embedded_shaders::MetaballShaderSourcePlugin;
#[cfg(feature = "present")]
pub use present::MetaballDisplayPlugin;
pub use settings::{MetaballRenderSettings, MetaballRendererPlugin};

/// Runtime‑mutable settings (public) allowing user code to toggle certain renderer behaviors
/// without accessing internal uniform types. Changes are propagated into GPU uniforms by
/// an internal sync system each frame (cheap compared to full buffer upload already occurring).
#[derive(Resource, Clone)]
pub struct RuntimeSettings {
    pub clustering_enabled: bool,
}
impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            clustering_enabled: true,
        }
    }
}

// Re-export select constants (namespaced) for advanced users; may become deprecated later.
pub mod consts {
    use crate::internal;
    pub const WORKGROUP_SIZE: u32 = internal::WORKGROUP_SIZE;
}

use internal::{AlbedoTexture, FieldTexture};
/// Retrieve the (field, albedo) render texture handles if the renderer is active.
pub fn metaball_textures(world: &World) -> Option<(Handle<Image>, Handle<Image>)> {
    let field = world.get_resource::<FieldTexture>()?;
    let albedo = world.get_resource::<AlbedoTexture>()?;
    Some((field.0.clone(), albedo.0.clone()))
}
