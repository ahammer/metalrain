//! Centralized game asset management (fonts, shaders, configs)
//! Provides a single plugin that loads and exposes asset handles so other crates
//! don't hardcode paths. Future: embedded + hot-reload abstraction.

use bevy::prelude::*;
use bevy::asset::LoadState;
use bevy::asset::UntypedAssetId;

#[derive(Resource, Debug, Clone, Default)]
pub struct FontAssets {
    pub ui_regular: Handle<Font>,
    pub ui_bold: Handle<Font>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ShaderAssets {
    pub compositor: Handle<Shader>,
    pub compute_metaballs: Handle<Shader>,
    pub compute_3d_normals: Handle<Shader>,
    pub present_fullscreen: Handle<Shader>,
    pub background: Handle<Shader>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct GameAssets {
    pub fonts: FontAssets,
    pub shaders: ShaderAssets,
}

// Allow automatic extraction into the render world so render graph pipeline creation can access shader handles.
impl bevy::render::extract_resource::ExtractResource for GameAssets {
    type Source = GameAssets; // same type between main & render worlds
    fn extract_resource(source: &Self::Source) -> Self { source.clone() }
}

/// Marker resource inserted once all startup assets have finished loading successfully.
#[derive(Resource, Debug, Clone, Copy)]
pub struct AssetsReady;

/// Internal resource tracking the untyped handles we still expect to finish loading.
#[derive(Resource, Debug, Default)]
struct PendingAssetGroup(Vec<UntypedAssetId>);

#[derive(Default)]
pub struct GameAssetsPlugin {
    pub use_embedded: bool, // placeholder for future embedding toggle
}

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(Startup, load_assets)
            .add_systems(Update, poll_startup_assets);
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
    mut commands: Commands,
    mut game_assets: ResMut<GameAssets>,
    asset_server: Res<AssetServer>,
) {
    let ui_regular: Handle<Font> = asset_server.load("fonts/FiraSans-Regular.ttf");
    let ui_bold: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");
    let compositor: Handle<Shader> = asset_server.load("shaders/compositor.wgsl");
    let compute_metaballs: Handle<Shader> = asset_server.load("shaders/compute_metaballs.wgsl");
    let compute_3d_normals: Handle<Shader> = asset_server.load("shaders/compute_3d_normals.wgsl");
    let present_fullscreen: Handle<Shader> = asset_server.load("shaders/present_fullscreen.wgsl");
    let background: Handle<Shader> = asset_server.load("shaders/background.wgsl");

    game_assets.fonts.ui_regular = ui_regular;
    game_assets.fonts.ui_bold = ui_bold;
    game_assets.shaders.compositor = compositor;
    game_assets.shaders.compute_metaballs = compute_metaballs;
    game_assets.shaders.compute_3d_normals = compute_3d_normals;
    game_assets.shaders.present_fullscreen = present_fullscreen;
    game_assets.shaders.background = background;

    // Record untyped ids for polling.
    let pending: Vec<UntypedAssetId> = [
        game_assets.fonts.ui_regular.id().untyped(),
        game_assets.fonts.ui_bold.id().untyped(),
        game_assets.shaders.compositor.id().untyped(),
        game_assets.shaders.compute_metaballs.id().untyped(),
        game_assets.shaders.compute_3d_normals.id().untyped(),
        game_assets.shaders.present_fullscreen.id().untyped(),
        game_assets.shaders.background.id().untyped(),
    ].into_iter().collect();
    commands.insert_resource(PendingAssetGroup(pending));
}

fn poll_startup_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pending: Option<Res<PendingAssetGroup>>,
) {
    let Some(pending) = pending else { return; };
    if pending.0.is_empty() {
        // Already processed; nothing to do.
        return;
    }
    let mut all_loaded = true;
    for id in &pending.0 {
        match asset_server.get_load_state(*id) {
            Some(LoadState::Loaded) => {}
            Some(LoadState::Failed(_)) => {
                // Fail fast: surface details for debugging.
                error!("Startup asset failed to load: {:?}", id);
                all_loaded = false; // keep polling; could choose to panic depending on policy.
            }
            Some(_) | None => {
                all_loaded = false;
            }
        }
    }
    if all_loaded {
        info!("All startup assets loaded.");
        commands.remove_resource::<PendingAssetGroup>();
        commands.insert_resource(AssetsReady);
    }
}

/// Helper to query readiness inside external code without directly checking the resource type.
pub fn assets_ready(world: &World) -> bool { world.contains_resource::<AssetsReady>() }
