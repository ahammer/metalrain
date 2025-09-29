//! Centralized game asset management (fonts, shaders, configs)
//! Provides a single plugin that loads and exposes asset handles so other crates
//! don't hardcode paths. Future: embedded + hot-reload abstraction.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct FontAssets {
    pub ui_regular: Handle<Font>,
    pub ui_bold: Handle<Font>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ShaderAssets {
    pub compositor: Handle<Shader>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct GameAssets {
    pub fonts: FontAssets,
    pub shaders: ShaderAssets,
}

pub struct GameAssetsPlugin {
    pub use_embedded: bool,
}

impl Default for GameAssetsPlugin {
    fn default() -> Self { Self { use_embedded: cfg!(feature = "embedded") } }
}

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(Startup, load_assets);
    }
}

fn load_assets(
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
) {
    // NOTE: For now we only support filesystem loading; embedded feature would swap to include_bytes! loaders.
    let ui_regular: Handle<Font> = asset_server.load("fonts/FiraSans-Regular.ttf");
    let ui_bold: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    let compositor: Handle<Shader> = asset_server.load("shaders/compositor.wgsl");

    game_assets.fonts.ui_regular = ui_regular;
    game_assets.fonts.ui_bold = ui_bold;
    game_assets.shaders.compositor = compositor;
}
