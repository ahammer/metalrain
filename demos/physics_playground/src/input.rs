//! Input handling systems for the physics playground.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::prelude::*;

use bevy_rapier2d::prelude::{Ccd, RigidBody as RapierRigidBody};
use game_core::{Ball, BallBundle, GameColor};
use game_physics::PhysicsConfig;
use game_rendering::{RenderLayer, RenderTargets};
use metaball_renderer::MetaBall;

use crate::resources::PlaygroundState;

/// Spawns a ball at the mouse cursor position when left-clicked.
pub fn spawn_ball_on_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    targets: Res<RenderTargets>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut playground_state: ResMut<PlaygroundState>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        error!("No primary window found");
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        warn!("No cursor position");
        return;
    };

    trace!(
        "Mouse click window pos = ({:.1},{:.1})",
        cursor_pos.x,
        cursor_pos.y
    );

    let Some(layer) = targets.layers.get(&RenderLayer::GameWorld) else {
        warn!("GameWorld layer camera missing (spawn_ball_on_click)");
        return;
    };

    let Ok((camera, camera_transform)) = cameras.get(layer.camera) else {
        warn!("Failed to access GameWorld camera (spawn_ball_on_click)");
        return;
    };

    let world_pos = match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(p) => p,
        Err(_) => match camera.viewport_to_world(camera_transform, cursor_pos) {
            Ok(ray) => ray.origin.truncate(),
            Err(e) => {
                warn!("Failed viewport->world conversion: {e:?}");
                return;
            }
        },
    };

    trace!("World click pos = ({:.1},{:.1})", world_pos.x, world_pos.y);

    let mut rng = rand::thread_rng();
    let colors = [
        GameColor::Red,
        GameColor::Blue,
        GameColor::Green,
        GameColor::Yellow,
        GameColor::White,
    ];
    let color = *colors.choose(&mut rng).unwrap();
    let radius = rng.gen_range(15.0..30.0);

    commands.spawn((
        BallBundle::new(world_pos, radius, color),
        MetaBall {
            radius_world: radius,
        },
        Name::new("Ball"),
    ));

    playground_state.balls_spawned += 1;

    info!(
        "Spawned ball #{} at {:?}",
        playground_state.balls_spawned, world_pos
    );
}

/// Resets the simulation by despawning all balls when 'R' is pressed.
pub fn reset_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    balls: Query<Entity, With<Ball>>,
    mut playground_state: ResMut<PlaygroundState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        let ball_count = balls.iter().count();
        for entity in &balls {
            commands.entity(entity).despawn();
        }

        playground_state.balls_spawned = 0;

        info!("Reset simulation - despawned {} balls", ball_count);
    }
}

/// Toggles physics simulation pause when 'P' is pressed.
pub fn pause_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut rapier_config: Query<&mut bevy_rapier2d::prelude::RapierConfiguration>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        if let Ok(mut config) = rapier_config.single_mut() {
            config.physics_pipeline_active = !config.physics_pipeline_active;
            info!("Physics paused: {}", !config.physics_pipeline_active);
        }
    }
}

/// Adjusts physics parameters using keyboard input.
/// Arrow keys adjust gravity, +/- keys adjust clustering strength.
pub fn adjust_physics_with_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut physics_config: ResMut<PhysicsConfig>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let speed = 500.0;

    let mut changed = false;

    if keys.pressed(KeyCode::ArrowUp) {
        physics_config.gravity.y += speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        physics_config.gravity.y -= speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        physics_config.gravity.x -= speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        physics_config.gravity.x += speed * delta;
        changed = true;
    }

    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        physics_config.clustering_strength =
            (physics_config.clustering_strength + 100.0 * delta).min(500.0);
        changed = true;
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        physics_config.clustering_strength =
            (physics_config.clustering_strength - 100.0 * delta).max(0.0);
        changed = true;
    }

    if changed {
        info!(
            "Physics - Gravity: ({:.0}, {:.0}), Clustering: {:.0}",
            physics_config.gravity.x, physics_config.gravity.y, physics_config.clustering_strength
        );
    }
}

/// Enables CCD (Continuous Collision Detection) for dynamic balls.
pub fn enable_ccd_for_balls(
    mut commands: Commands,
    q: Query<(Entity, &RapierRigidBody), (With<Ball>, Without<Ccd>)>,
) {
    for (e, body) in &q {
        if matches!(body, RapierRigidBody::Dynamic) {
            commands.entity(e).insert(Ccd::enabled());
        }
    }
}
