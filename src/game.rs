use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::emitter::BallEmitterPlugin;
use crate::rapier_physics::PhysicsSetupPlugin;
use crate::separation::SeparationPlugin;
use crate::system_order::{PrePhysicsSet, PostPhysicsAdjustSet};
use crate::materials::MaterialsPlugin;
use crate::cluster::ClusterPlugin;
use crate::metaballs::MetaballsPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Register custom system sets (order constraints added later as needed)
            .configure_sets(Update, (
                PrePhysicsSet,
                PostPhysicsAdjustSet.after(PrePhysicsSet),
            ))
            .add_plugins((
            CameraPlugin,
            MaterialsPlugin,
            PhysicsSetupPlugin,
            BallEmitterPlugin,
            SeparationPlugin,
            ClusterPlugin,
            MetaballsPlugin,
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
