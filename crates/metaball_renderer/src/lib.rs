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
//! Coordinate space: `MetaBall.center` and `MetaBall.radius` are interpreted in pixel
//! space of the offscreen render texture (0..texture_size). Phase 3 docs will expand
//! this with helper mapping utilities.

use bevy::prelude::*;

mod settings;
mod components;
mod internal;
mod compute;
mod embedded_shaders;
#[cfg(feature = "present")]
mod present;

pub use settings::{MetaballRenderSettings, MetaballRendererPlugin};
pub use components::{MetaBall, MetaBallColor, MetaBallCluster};

// Re-export select constants (namespaced) for advanced users; may become deprecated later.
pub mod consts { use crate::internal; pub const WORKGROUP_SIZE: u32 = internal::WORKGROUP_SIZE; pub const MAX_BALLS: usize = internal::MAX_BALLS; }
