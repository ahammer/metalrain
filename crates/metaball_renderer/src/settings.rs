use bevy::prelude::*;
use crate::compute::ComputeMetaballsPlugin;
use crate::pack::PackingPlugin;
#[cfg(feature = "present")] use crate::present::MetaballDisplayPlugin;

/// Public settings controlling renderer subsystems.
#[derive(Clone, Resource)]
pub struct MetaballRenderSettings {
    pub present: bool,
    pub texture_size: UVec2,
    /// Initial clustering enabled state (controls hard cluster coloring vs blended gradient)
    pub enable_clustering: bool,
}
impl Default for MetaballRenderSettings { fn default() -> Self { Self { present: true, texture_size: UVec2::new(1024,1024), enable_clustering: true } } }

/// Main plugin entry point.
pub struct MetaballRendererPlugin { pub settings: MetaballRenderSettings }
impl Default for MetaballRendererPlugin { fn default() -> Self { Self { settings: MetaballRenderSettings::default() } } }
impl MetaballRendererPlugin { pub fn with(settings: MetaballRenderSettings) -> Self { Self { settings } } }
impl Plugin for MetaballRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.clone());
        // Runtime settings resource (mutable by user code); mirrors subset of ParamsUniform flags.
        app.init_resource::<crate::RuntimeSettings>();
        {
            let mut rt = app.world_mut().resource_mut::<crate::RuntimeSettings>();
            rt.clustering_enabled = self.settings.enable_clustering;
        }
    app.add_plugins(ComputeMetaballsPlugin);
    app.add_plugins(PackingPlugin); // Phase 3 packing
        #[cfg(feature = "present")] if self.settings.present { app.add_plugins(MetaballDisplayPlugin); }
    }
}
