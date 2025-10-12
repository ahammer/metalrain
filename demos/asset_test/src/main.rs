use bevy::prelude::*;
use game_assets::{configure_demo, AssetsReady, GameAssets};

#[derive(Resource, Default)]
struct PrintedOnce(bool);

fn main() {
    let mut app = App::new();
    configure_demo(&mut app);
    app.insert_resource(PrintedOnce::default())
        .add_systems(Startup, log_startup_banner)
        .add_systems(
            Update,
            log_when_ready.run_if(resource_exists::<AssetsReady>),
        )
        .run();
}

fn log_startup_banner() {
    info!("Asset_Test demo starting. Waiting for standardized startup assets to load...");
}

fn log_when_ready(assets: Res<GameAssets>, mut printed: ResMut<PrintedOnce>) {
    if printed.0 {
        return;
    }
    info!("Asset_Test: All startup assets loaded (7 assets).");
    debug!("Fonts: regular={:?} bold={:?}; Shaders: comp={:?} metaballs={:?} normals={:?} present={:?} bg={:?}",
        assets.fonts.ui_regular,
        assets.fonts.ui_bold,
        assets.shaders.compositor,
        assets.shaders.compute_metaballs,
        assets.shaders.compute_3d_normals,
        assets.shaders.present_fullscreen,
        assets.shaders.background
    );
    printed.0 = true;
}
