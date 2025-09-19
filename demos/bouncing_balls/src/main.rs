use bevy::prelude::*;
use metaball_renderer::MetaballRendererPlugin;
mod simulation;
mod debug_vis;
use simulation::BouncySimulationPlugin;
use debug_vis::DebugVisPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::default())
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}
