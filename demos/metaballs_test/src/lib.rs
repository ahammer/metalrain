use bevy::math::UVec2;
use bevy::prelude::*;
use game_core::AppState;
use scaffold::{ScaffoldConfig, ScaffoldIntegrationPlugin};

mod debug_vis;
mod simulation;
use debug_vis::DebugVisPlugin;
use simulation::BouncySimulationPlugin;

pub const DEMO_NAME: &str = "metaballs_test";

pub fn run_metaballs_test() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK));

    app.insert_resource(
        ScaffoldConfig::default()
            .with_metaball_texture_size(UVec2::new(512, 512))
            .with_world_half_extent(simulation::HALF_EXTENT + simulation::COLLISION_PADDING),
    );

    app.add_plugins(ScaffoldIntegrationPlugin::with_demo_name(DEMO_NAME));

    app.init_state::<AppState>();

    app
        .add_plugins(BouncySimulationPlugin)
        .add_plugins(DebugVisPlugin)
        .run();
}
