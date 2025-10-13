use crate::compute::{ComputeMetaballsPlugin, NormalComputePlugin};
use crate::coordinates::MetaballCoordinateMapper;
use crate::diagnostics::MetaballDiagnosticsPlugin;
use crate::pack::PackingPlugin;
use bevy::prelude::*;
use bevy::render::{renderer::RenderDevice, Render, RenderApp};

#[derive(Clone, Resource, Debug)]
pub struct MetaballRenderSettings {
    pub texture_size: UVec2,
    pub world_bounds: Rect,
    pub enable_clustering: bool,
    pub present_via_quad: bool,
    pub presentation_layer: Option<u8>,
}
impl Default for MetaballRenderSettings {
    fn default() -> Self {
        Self {
            texture_size: UVec2::new(1024, 1024),
            world_bounds: Rect::from_corners(Vec2::new(-256.0, -256.0), Vec2::new(256.0, 256.0)),
            enable_clustering: true,
            present_via_quad: false,
            presentation_layer: None,
        }
    }
}

impl MetaballRenderSettings {
    pub fn with_texture_size(mut self, size: UVec2) -> Self {
        self.texture_size = size;
        self
    }
    pub fn with_world_bounds(mut self, rect: Rect) -> Self {
        self.world_bounds = rect;
        self
    }
    pub fn clustering_enabled(mut self, enabled: bool) -> Self {
        self.enable_clustering = enabled;
        self
    }
    pub fn with_presentation(mut self, enabled: bool) -> Self {
        self.present_via_quad = enabled;
        self
    }
    pub fn with_presentation_layer(mut self, layer: u8) -> Self {
        self.presentation_layer = Some(layer);
        self
    }
}

pub struct MetaballRendererPlugin {
    pub settings: MetaballRenderSettings,
}
impl Default for MetaballRendererPlugin {
    fn default() -> Self {
        Self {
            settings: MetaballRenderSettings::default(),
        }
    }
}
impl MetaballRendererPlugin {
    pub fn with(settings: MetaballRenderSettings) -> Self {
        Self { settings }
    }
}
impl Plugin for MetaballRendererPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.clone());
        let mapper = MetaballCoordinateMapper::new(
            self.settings.texture_size,
            self.settings.world_bounds.min,
            self.settings.world_bounds.max,
        );
        app.insert_resource(mapper);
        app.init_resource::<crate::RuntimeSettings>();
        {
            let mut rt = app.world_mut().resource_mut::<crate::RuntimeSettings>();
            rt.clustering_enabled = self.settings.enable_clustering;
        }
        app.add_plugins(MetaballDiagnosticsPlugin);

        app.add_plugins(ComputeMetaballsPlugin);
        app.add_plugins(NormalComputePlugin);
        app.add_plugins(PackingPlugin);
        #[cfg(feature = "present")]
        if self.settings.present_via_quad {
            app.add_plugins(crate::present::MetaballDisplayPlugin);
        }

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, log_adapter_limits_once);
    }
}

fn log_adapter_limits_once(render_device: Res<RenderDevice>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let limits = render_device.limits();
    info!(target: "gpu", "Adapter limits: max_storage_buffers_per_shader_stage={}", limits.max_storage_buffers_per_shader_stage);
    assert!(
        limits.max_storage_buffers_per_shader_stage >= 1,
        "Storage buffers per shader stage < 1 (unexpected WebGL fallback path)"
    );
    *done = true;
}
