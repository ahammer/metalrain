use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
    }
}

fn setup_camera(mut commands: Commands) {
    // Bevy 0.16+: spawn Camera2d component directly; Required Components supply defaults.
    commands.spawn(Camera2d);
}
