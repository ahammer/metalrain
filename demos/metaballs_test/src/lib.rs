use bevy::prelude::*;
use metaball_renderer::{MetaballRenderSettings, MetaballRendererPlugin};
use game_assets::configure_demo;
use game_core::AppState;

mod debug_vis;
mod simulation;
use debug_vis::DebugVisPlugin;
use simulation::BouncySimulationPlugin;

pub const DEMO_NAME: &str = "metaballs_test";

pub fn run_metaballs_test() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK));

    configure_demo(&mut app);

    app.init_state::<AppState>();

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

    app.add_systems(OnEnter(AppState::Playing), spawn_camera)
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Name::new("MetaballDemoCamera")));
}
