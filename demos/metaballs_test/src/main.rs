use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings, MetaballShaderSourcePlugin};
mod simulation;
mod debug_vis;
use simulation::BouncySimulationPlugin;
use debug_vis::DebugVisPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        // Register hot-reload asset source BEFORE AssetPlugin / DefaultPlugins
        .add_plugins(MetaballShaderSourcePlugin)
        .add_plugins(DefaultPlugins)
    .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings { texture_size: UVec2::new(512,512), world_bounds: Rect::from_corners(Vec2::new(-256.0,-256.0), Vec2::new(256.0,256.0)), enable_clustering: true }))
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}
