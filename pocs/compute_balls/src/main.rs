mod constants;
mod compute;
mod present;
mod systems;

use bevy::prelude::*;
use bevy::window::WindowPlugin;

use constants::*;
use compute::ComputeMetaballsPlugin;
use present::PresentPlugin;
use systems::AnimationAndInputPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Compute Metaballs".into(),
                        resolution: (WIDTH as f32 * DISPLAY_SCALE, HEIGHT as f32 * DISPLAY_SCALE).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            ComputeMetaballsPlugin,
            AnimationAndInputPlugin,
            PresentPlugin,
        ))
        .run();
}

