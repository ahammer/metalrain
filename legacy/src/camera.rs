use bevy::prelude::*;
// ClearColorConfig is re-exported in prelude in Bevy 0.16.

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera);
    }
}

fn setup_camera(mut commands: Commands) {
    // Spawn primary 2D camera with no automatic clear; background plugin draws first.
    commands.spawn((Camera2d, Camera { clear_color: ClearColorConfig::None, ..default() }));
}
