use bevy::prelude::*;

mod components;
mod compute;
mod coordinates;
mod diagnostics;
mod internal;
mod pack;
#[cfg(feature = "present")]
mod present;
mod settings;
mod spatial;

pub use components::{MetaBall, MetaBallCluster, MetaBallColor};
pub use coordinates::{
    project_world_to_screen, screen_to_metaball_uv, screen_to_world, MetaballCoordinateMapper,
};
pub use diagnostics::{MetaballDiagnosticsConfig, MetaballDiagnosticsPlugin};
#[cfg(feature = "present")]
pub use present::{MetaballDisplayMaterial, MetaballDisplayPlugin, MetaballPresentationQuad};
pub use settings::{MetaballRenderSettings, MetaballRendererPlugin};

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

pub mod consts {
    use crate::internal;
    pub const WORKGROUP_SIZE: u32 = internal::WORKGROUP_SIZE;
}

use internal::{AlbedoTexture, FieldTexture};
pub fn metaball_textures(world: &World) -> Option<(Handle<Image>, Handle<Image>)> {
    let field = world.get_resource::<FieldTexture>()?;
    let albedo = world.get_resource::<AlbedoTexture>()?;
    Some((field.0.clone(), albedo.0.clone()))
}
