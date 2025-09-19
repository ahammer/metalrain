use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin};
mod simulation;
use simulation::BouncySimulationPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
    .add_plugins(MetaballRendererPlugin::default())
    .add_plugins(BouncySimulationPlugin)
        .run();
}
