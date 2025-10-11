use bevy::prelude::*;
use metaball_renderer::{MetaballRenderSettings, MetaballRendererPlugin};
use game_assets::configure_demo; // standardized asset root + GameAssets loading

mod debug_vis;
mod simulation;
use debug_vis::DebugVisPlugin;
use simulation::BouncySimulationPlugin;

pub const DEMO_NAME: &str = "metaballs_test";

pub fn run_metaballs_test() {
    // Build manually so we can insert assets via helper before renderer plugin queues pipelines.
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK));

    // Standardized asset root (Demo mode) + GameAssets plugin.
    configure_demo(&mut app);

    // Metaball renderer configured after assets so shader handle is available early.
    app.add_plugins(MetaballRendererPlugin::with(
        MetaballRenderSettings::default()
            .with_texture_size(UVec2::new(512, 512))
            .with_world_bounds(Rect::from_corners(
                Vec2::new(-256.0, -256.0),
                Vec2::new(256.0, 256.0),
            ))
            .clustering_enabled(true)
            .with_presentation(true),
    ));

    app.add_systems(Startup, spawn_camera)
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Name::new("MetaballDemoCamera")));
}
