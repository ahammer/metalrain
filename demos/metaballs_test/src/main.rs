use bevy::prelude::*;
use metaball_renderer::{
    MetaballRenderSettings, MetaballRendererPlugin, MetaballShaderSourcePlugin,
};
mod debug_vis;
mod simulation;
use debug_vis::DebugVisPlugin;
use simulation::BouncySimulationPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        // Register hot-reload asset source BEFORE AssetPlugin / DefaultPlugins
        .add_plugins(MetaballShaderSourcePlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(UVec2::new(512, 512))
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-256.0, -256.0),
                    Vec2::new(256.0, 256.0),
                ))
                .clustering_enabled(true)
                .with_presentation(true),
        ))
        .add_systems(Startup, spawn_camera)
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    // Simple 2D camera at origin; users can adjust scaling mode as needed externally.
    commands.spawn((Camera2d, Name::new("MetaballDemoCamera")));
}
