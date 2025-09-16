mod constants;
mod compute;
mod present;
mod systems;
mod metaball;
mod bouncy;
mod metaball_plugin;

use bevy::prelude::*;
use bevy::window::WindowPlugin;

use constants::*;
use metaball_plugin::MetaballRendererPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Compute Metaballs".into(),
                        resolution: (WIDTH as f32, HEIGHT as f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(MetaballRendererPlugin::default())
        .run();
}

