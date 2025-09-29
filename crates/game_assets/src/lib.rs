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

/// Standardized asset root modes for different crate execution contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetRootMode {
    /// For binaries under `demos/<demo_name>` (cwd = that demo directory)
    DemoCrate,
    /// For binaries under `crates/<game_crate>` (cwd = that crate directory)
    GameCrate,
    /// Executed from workspace root (e.g. `cargo test` aggregating)
    WorkspaceRoot,
}

impl AssetRootMode {
    pub fn path(self) -> &'static str {
        match self {
            AssetRootMode::DemoCrate => "../../assets",
            AssetRootMode::GameCrate => "../assets",
            AssetRootMode::WorkspaceRoot => "assets",
        }
    }
}

/// Configure an app with the standardized AssetPlugin root path and GameAssetsPlugin.
/// Call this BEFORE adding other plugins that may perform asset loading.
pub fn configure_standard_assets(app: &mut App, mode: AssetRootMode) {
    use bevy::asset::AssetPlugin;
    // Remove any pre-existing default AssetPlugin to avoid duplicate warnings.
    // (If DefaultPlugins not added yet, this is a no-op.)
    // (No-op placeholder; previously used to ensure mutable borrow ordering.)
    app.add_plugins(bevy::DefaultPlugins.set(AssetPlugin {
        file_path: mode.path().into(),
        ..Default::default()
    }));
    app.add_plugins(GameAssetsPlugin::default());
}

/// Convenience wrappers for the common use cases.
pub fn configure_demo(app: &mut App) { configure_standard_assets(app, AssetRootMode::DemoCrate); }
pub fn configure_game_crate(app: &mut App) { configure_standard_assets(app, AssetRootMode::GameCrate); }
pub fn configure_workspace_root(app: &mut App) { configure_standard_assets(app, AssetRootMode::WorkspaceRoot); }

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
