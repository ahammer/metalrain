use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::config::GameConfig;

pub struct PhysicsSetupPlugin; // our wrapper to configure Rapier & arena

impl Plugin for PhysicsSetupPlugin {
    fn build(&self, app: &mut App) {
    app.add_plugins((RapierPhysicsPlugin::<NoUserData>::default(),))
    .add_systems(Startup, configure_gravity);
    }
}

fn configure_gravity(mut rapier_cfg: ResMut<RapierConfiguration>, _game_cfg: Res<GameConfig>) {
    // Global gravity disabled for custom radial gravity system.
    rapier_cfg.gravity = Vect::new(0.0, 0.0);
}
