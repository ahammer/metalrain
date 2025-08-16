use bevy::prelude::*;

use crate::camera::CameraPlugin;
use crate::physics::PhysicsPlugin;
use crate::spawn::BallSpawnPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CameraPlugin, BallSpawnPlugin, PhysicsPlugin));
    }
}
