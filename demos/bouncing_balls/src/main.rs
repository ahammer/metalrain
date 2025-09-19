use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings};
mod simulation;
mod debug_vis;
use simulation::BouncySimulationPlugin;
use debug_vis::DebugVisPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
    .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings { present: true, texture_size: UVec2::new(512,512), enable_clustering: true }))
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}
