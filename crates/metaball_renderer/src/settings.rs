use crate::compute::{ComputeMetaballsPlugin, NormalComputePlugin};
use crate::coordinates::MetaballCoordinateMapper;
use crate::diagnostics::MetaballDiagnosticsPlugin;
use crate::pack::PackingPlugin;
use bevy::prelude::*;
use bevy::render::{Render, RenderApp, renderer::RenderDevice};

/// Public settings controlling renderer subsystems & coordinate mapping.
#[derive(Clone, Resource, Debug)]
pub struct MetaballRenderSettings {
    /// Size (in pixels) of the offscreen metaball simulation / shading textures.
    pub texture_size: UVec2,
    /// Authoritative world bounds mapped onto the texture (Z assumed 0 for mapping).
    pub world_bounds: Rect,
    /// Initial clustering enabled state (controls hard cluster coloring vs blended gradient)
    pub enable_clustering: bool,
    /// When true (and the crate `present` feature is enabled) a simple presentation quad
    /// is spawned mapping the metaball offscreen textures into world space for quick
    /// visualization. The quad covers `world_bounds` exactly. No camera is created; user
    /// code must spawn a 2D camera.
    pub present_via_quad: bool,
    /// Optional render layer index the presentation quad should spawn onto. When `None`
    /// the quad inherits Bevy's implicit default (layer 0). Providing a value (e.g. 2 for
    /// a dedicated Metaballs layer) ensures the offscreen metaball visualization routes
    /// exclusively through the intended compositor layer.
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
    /// Enable/disable built-in presentation quad (requires `present` crate feature).
    pub fn with_presentation(mut self, enabled: bool) -> Self {
        self.present_via_quad = enabled;
        self
    }
    /// Specify a render layer for the spawned presentation quad (only honored when
    /// `present_via_quad` is true and the `present` feature is enabled).
    pub fn with_presentation_layer(mut self, layer: u8) -> Self {
        self.presentation_layer = Some(layer);
        self
    }
}

/// Main plugin entry point.
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
        // Insert static settings resource.
        app.insert_resource(self.settings.clone());
        // Coordinate mapper derived from settings.
        let mapper = MetaballCoordinateMapper::new(
            self.settings.texture_size,
            self.settings.world_bounds.min,
            self.settings.world_bounds.max,
        );
        app.insert_resource(mapper);
        // Runtime settings resource (mutable by user code); mirrors subset of ParamsUniform flags.
        app.init_resource::<crate::RuntimeSettings>();
        {
            let mut rt = app.world_mut().resource_mut::<crate::RuntimeSettings>();
            rt.clustering_enabled = self.settings.enable_clustering;
        }
        // Diagnostics (enabled by default; user can disable by mutating MetaballDiagnosticsConfig resource early).
        app.add_plugins(MetaballDiagnosticsPlugin);

        // Core compute & packing pipeline.
        app.add_plugins(ComputeMetaballsPlugin);
        app.add_plugins(NormalComputePlugin); // normals from packed field
        app.add_plugins(PackingPlugin); // packs entities each frame (or on change)
                                        // Optional presentation path (quad) – only if user enabled in settings & feature active.
        #[cfg(feature = "present")]
        if self.settings.present_via_quad {
            app.add_plugins(crate::present::MetaballDisplayPlugin);
        }

        // Insert GPU adapter limits diagnostics into render world (runs once when render device ready).
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, log_adapter_limits_once);
    }
}

/// Logs adapter limits (once) and asserts storage buffer availability.
fn log_adapter_limits_once(render_device: Res<RenderDevice>, mut done: Local<bool>) {
    if *done { return; }
    let limits = render_device.limits();
    info!(target: "gpu", "Adapter limits: max_storage_buffers_per_shader_stage={}", limits.max_storage_buffers_per_shader_stage);
    assert!(limits.max_storage_buffers_per_shader_stage >= 1, "Storage buffers per shader stage < 1 (unexpected WebGL fallback path)");
    *done = true;
}
