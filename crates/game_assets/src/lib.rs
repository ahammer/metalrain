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

impl bevy::render::extract_resource::ExtractResource for GameAssets {
    type Source = GameAssets;
    fn extract_resource(source: &Self::Source) -> Self { source.clone() }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct AssetsReady;

#[derive(Resource, Debug, Default)]
struct PendingAssetGroup(Vec<UntypedAssetId>);

#[derive(Default)]
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAssets>()
            .add_systems(Startup, load_assets)
            .add_systems(Update, poll_startup_assets)
            .add_systems(Update, check_assets_ready_transition.run_if(in_state(game_core::AppState::Loading)));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetRootMode {
    DemoCrate,
    GameCrate,
    WorkspaceRoot,
}

impl AssetRootMode {
    pub fn path(self) -> &'static str {
        match self {
            AssetRootMode::DemoCrate => {
                #[cfg(target_arch = "wasm32")]
                { "assets" }
                #[cfg(not(target_arch = "wasm32"))]
                { "../../assets" }
            }
            AssetRootMode::GameCrate => {
                #[cfg(target_arch = "wasm32")]
                { "assets" }
                #[cfg(not(target_arch = "wasm32"))]
                { "../assets" }
            }
            AssetRootMode::WorkspaceRoot => "assets",
        }
    }
}

pub fn configure_standard_assets(app: &mut App, mode: AssetRootMode) {
    use bevy::asset::AssetPlugin;
    app.add_plugins(bevy::DefaultPlugins.set(AssetPlugin {
        file_path: mode.path().into(),
        ..Default::default()
    }));
    app.add_plugins(GameAssetsPlugin::default());
}

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
        return;
    }
    let mut all_loaded = true;
    for id in &pending.0 {
        match asset_server.get_load_state(*id) {
            Some(LoadState::Loaded) => {}
            Some(LoadState::Failed(_)) => {
                error!("Startup asset failed to load: {:?}", id);
                all_loaded = false;
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

pub fn assets_ready(world: &World) -> bool { world.contains_resource::<AssetsReady>() }

fn check_assets_ready_transition(
    assets_ready: Option<Res<AssetsReady>>,
    mut next_state: ResMut<NextState<game_core::AppState>>,
) {
    if assets_ready.is_some() {
        info!("All startup assets loaded; transitioning to Playing state.");
        next_state.set(game_core::AppState::Playing);
    }
}
