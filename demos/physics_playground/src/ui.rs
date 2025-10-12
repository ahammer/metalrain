//! UI update systems for displaying stats and information.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use game_core::Ball;
use game_physics::PhysicsConfig;
use game_rendering::GameCamera;

use crate::components::{MousePositionText, StatsText};
use crate::resources::PlaygroundState;

/// Updates the stats text with FPS, ball count, and physics parameters.
pub fn update_stats_text(
    mut text_query: Query<&mut Text, With<StatsText>>,
    diagnostics: Res<DiagnosticsStore>,
    balls: Query<&Ball>,
    _playground_state: Res<PlaygroundState>,
    physics_config: Res<PhysicsConfig>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let ball_count = balls.iter().count();

    **text = format!(
        "FPS: {:03} | Balls: {} | Gravity: ({:.0}, {:.0}) | Clustering: {:.0}",
        fps as u32,
        ball_count,
        physics_config.gravity.x,
        physics_config.gravity.y,
        physics_config.clustering_strength,
    );
}

/// Updates the mouse position text with current world coordinates.
pub fn update_mouse_position_text(
    mut text_query: Query<&mut Text, With<MousePositionText>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let Ok(window) = windows.single() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let world_pos = match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(p) => p,
        Err(_) => match camera.viewport_to_world(camera_transform, cursor_pos) {
            Ok(ray) => ray.origin.truncate(),
            Err(_) => {
                **text = "Mouse: ---, ---".to_string();
                return;
            }
        },
    };
    **text = format!("Mouse: {:.0}, {:.0}", world_pos.x, world_pos.y);
}
