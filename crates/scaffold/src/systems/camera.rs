use bevy::prelude::*;
use game_rendering::{targets::RenderTargets, GameCamera};

use crate::resources::ScaffoldConfig;

/// Positions the shared game camera rig and applies baseline zoom limits.
pub fn align_game_camera(
    config: Res<ScaffoldConfig>,
    targets: Res<RenderTargets>,
    mut transforms: Query<&mut Transform>,
    mut game_camera: Query<&mut GameCamera>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    if let Some(rig) = targets.camera_rig {
        if let Ok(mut transform) = transforms.get_mut(rig) {
            transform.translation = Vec3::new(0.0, 0.0, 500.0);
            transform.rotation = Quat::IDENTITY;
        }
    } else {
        return;
    }

    if let Some(mut camera) = game_camera.iter_mut().next() {
        camera.base_resolution = Vec2::new(
            config.base_resolution.x as f32,
            config.base_resolution.y as f32,
        );
        camera.zoom_bounds = Vec2::new(0.35, 3.0);
        camera.target_viewport_scale = camera.target_viewport_scale.clamp(0.35, 3.0);
        camera.viewport_scale = camera.viewport_scale.clamp(0.35, 3.0);
    }

    *done = true;
}
