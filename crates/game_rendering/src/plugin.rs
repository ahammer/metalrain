use bevy::prelude::*;

use crate::camera::{CameraShakeCommand, CameraZoomCommand, GameCamera, GameCameraSettings};
use crate::compositor::{
    setup_compositor_pass, sync_compositor_geometry, sync_compositor_material,
    CompositorMaterialPlugin, CompositorPresentation, CompositorSettings, LayerBlendState,
};
use crate::layers::{LayerToggleState, RenderLayer};
use crate::targets::{handle_window_resize, setup_render_targets, RenderSurfaceSettings};

pub struct GameRenderingPlugin;

impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RenderLayer>()
            .register_type::<GameCamera>()
            .register_type::<LayerToggleState>()
            .register_type::<LayerBlendState>()
            .register_type::<CompositorSettings>()
            .register_type::<GameCameraSettings>()
            .init_resource::<RenderSurfaceSettings>()
            .init_resource::<LayerToggleState>()
            .init_resource::<LayerBlendState>()
            .init_resource::<crate::targets::RenderTargets>()
            .init_resource::<crate::targets::RenderTargetHandles>()
            .init_resource::<CompositorSettings>()
            .init_resource::<GameCameraSettings>()
            .init_resource::<CompositorPresentation>()
            .add_event::<CameraShakeCommand>()
            .add_event::<CameraZoomCommand>()
            .add_plugins(CompositorMaterialPlugin)
            .add_systems(Startup, setup_render_targets)
            .add_systems(Startup, setup_compositor_pass.after(setup_render_targets))
            .add_systems(Update, handle_window_resize)
            .add_systems(Update, (sync_compositor_material, sync_compositor_geometry));
    }
}
