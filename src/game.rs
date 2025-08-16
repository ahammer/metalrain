use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::emitter::BallEmitterPlugin;
use crate::rapier_physics::PhysicsSetupPlugin;
use crate::separation::SeparationPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CameraPlugin,
            PhysicsSetupPlugin,
            BallEmitterPlugin,
            SeparationPlugin,
        ))
        .add_systems(Update, debug_entity_counts);
    }
}

fn debug_entity_counts(
    time: Res<Time>,
    mut timer: Local<f32>,
    q_balls: Query<&crate::components::Ball>,
    q_cam: Query<&Camera>,
) {
    *timer += time.delta_seconds();
    if *timer > 1.0 {
        *timer = 0.0;
        info!(
            "balls={} cameras={}",
            q_balls.iter().count(),
            q_cam.iter().count()
        );
    }
}
