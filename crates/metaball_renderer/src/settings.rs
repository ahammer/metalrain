use bevy::prelude::*;
use crate::compute::ComputeMetaballsPlugin;
#[cfg(feature = "present")] use crate::present::MetaballDisplayPlugin;

/// Public settings controlling renderer subsystems.
#[derive(Clone, Resource)]
pub struct MetaballRenderSettings {
    pub present: bool,
    pub texture_size: UVec2,
}
impl Default for MetaballRenderSettings { fn default() -> Self { Self { present: true, texture_size: UVec2::new(1024,1024) } } }

/// Main plugin entry point.
pub struct MetaballRendererPlugin { pub settings: MetaballRenderSettings }
impl Default for MetaballRendererPlugin { fn default() -> Self { Self { settings: MetaballRenderSettings::default() } } }
impl MetaballRendererPlugin { pub fn with(settings: MetaballRenderSettings) -> Self { Self { settings } } }
impl Plugin for MetaballRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.clone());
        app.add_plugins(ComputeMetaballsPlugin);
        #[cfg(feature = "present")] if self.settings.present { app.add_plugins(MetaballDisplayPlugin); }
    }
}
