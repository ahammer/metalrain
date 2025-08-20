use bevy::prelude::*;
use crate::fluid_sim; // access FluidDisplayQuad component for toggling FluidSim background
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
static BG_WORLDGRID_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static BG_FLUID_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[derive(Clone, Copy, ShaderType, Debug)]
struct BgData {
    window_size: Vec2,
    cell_size: f32,
    line_thickness: f32,
    dark_factor: f32,
    _pad: f32,
}

impl Default for BgData {
    fn default() -> Self { Self { window_size: Vec2::ZERO, cell_size: 128.0, line_thickness: 0.015, dark_factor: 0.15, _pad: 0.0 } }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
struct BgMaterial { #[uniform(0)] data: BgData }

impl Material2d for BgMaterial {
    fn fragment_shader() -> ShaderRef {
    #[cfg(target_arch = "wasm32")]
    { return BG_WORLDGRID_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/bg_worldgrid.wgsl".into())); }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_worldgrid.wgsl".into() }
    }
    fn vertex_shader() -> ShaderRef {
    #[cfg(target_arch = "wasm32")]
    { return BG_WORLDGRID_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/bg_worldgrid.wgsl".into())); }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_worldgrid.wgsl".into() }
    }
}

#[derive(Component)]
struct BackgroundQuad;

#[derive(Component)]
struct FluidBackgroundQuad;

// Active background selector
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBackground { Grid, Fluid, #[default] FluidSim }

#[derive(Clone, Copy, ShaderType, Debug)]
struct FluidData { window_size: Vec2, time: f32, scale: f32, intensity: f32, _pad: f32 }

impl Default for FluidData {
    fn default() -> Self { Self { window_size: Vec2::ZERO, time: 0.0, scale: 1.25, intensity: 0.9, _pad: 0.0 } }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
struct FluidMaterial { #[uniform(0)] data: FluidData }

impl Material2d for FluidMaterial {
    fn fragment_shader() -> ShaderRef {
    #[cfg(target_arch = "wasm32")]
    { return BG_FLUID_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/bg_fluid.wgsl".into())); }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_fluid.wgsl".into() }
    }
    fn vertex_shader() -> ShaderRef {
    #[cfg(target_arch = "wasm32")]
    { return BG_FLUID_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/bg_fluid.wgsl".into())); }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_fluid.wgsl".into() }
    }
}

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            use bevy::asset::Assets;
            use bevy::render::render_resource::Shader;
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let worldgrid = shaders.add(Shader::from_wgsl(include_str!("../assets/shaders/bg_worldgrid.wgsl"), "bg_worldgrid_embedded.wgsl"));
            let fluid = shaders.add(Shader::from_wgsl(include_str!("../assets/shaders/bg_fluid.wgsl"), "bg_fluid_embedded.wgsl"));
            BG_WORLDGRID_SHADER_HANDLE.get_or_init(|| worldgrid.clone());
            BG_FLUID_SHADER_HANDLE.get_or_init(|| fluid.clone());
        }
        app.insert_resource(ActiveBackground::FluidSim)
            .add_plugins((Material2dPlugin::<BgMaterial>::default(), Material2dPlugin::<FluidMaterial>::default()))
            .add_systems(Startup, setup_backgrounds)
            .add_systems(Update, (resize_bg_uniform, resize_fluid_uniform, update_fluid_time, toggle_background));
    }
}
fn setup_backgrounds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut grid_mats: ResMut<Assets<BgMaterial>>,
    mut fluid_mats: ResMut<Assets<FluidMaterial>>,
    windows: Query<&Window>,
) {
    let (w,h) = if let Ok(win) = windows.single() {(win.width(), win.height())} else {(800.0,600.0)};
    let mut grid_mat = BgMaterial::default();
    grid_mat.data.window_size = Vec2::new(w,h);
    let grid_handle = grid_mats.add(grid_mat);

    let mut fluid_mat = FluidMaterial::default();
    fluid_mat.data.window_size = Vec2::new(w,h);
    let fluid_handle = fluid_mats.add(fluid_mat);

    let mesh = meshes.add(Mesh::from(Rectangle::new(2.0,2.0)));
    // Spawn grid hidden initially; fluid visible by default
    commands.spawn((
        Mesh2d::from(mesh.clone()),
        MeshMaterial2d(grid_handle),
        Transform::from_xyz(0.0,0.0,-500.0),
        Visibility::Hidden,
        InheritedVisibility::HIDDEN,
        BackgroundQuad));
    commands.spawn((
        Mesh2d::from(mesh),
        MeshMaterial2d(fluid_handle),
        Transform::from_xyz(0.0,0.0,-499.9),
        Visibility::Visible,
        InheritedVisibility::VISIBLE,
        FluidBackgroundQuad));
    info!("Background quads spawned (grid + fluid)");
}

fn resize_bg_uniform(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<BgMaterial>, With<BackgroundQuad>>,
    mut materials: ResMut<Assets<BgMaterial>>,
) {
    let Ok(win) = windows.single() else { return; };
    let Ok(handle) = q_mat.single() else { return; };
    if let Some(mat) = materials.get_mut(&handle.0) { if mat.data.window_size.x != win.width() || mat.data.window_size.y != win.height() { mat.data.window_size = Vec2::new(win.width(), win.height()); }}
}

fn resize_fluid_uniform(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<FluidMaterial>, With<FluidBackgroundQuad>>,
    mut materials: ResMut<Assets<FluidMaterial>>,
) {
    let Ok(win) = windows.single() else { return; };
    let Ok(handle) = q_mat.single() else { return; };
    if let Some(mat) = materials.get_mut(&handle.0) { if mat.data.window_size.x != win.width() || mat.data.window_size.y != win.height() { mat.data.window_size = Vec2::new(win.width(), win.height()); }}
}

fn update_fluid_time(
    time: Res<Time>,
    q_mat: Query<&MeshMaterial2d<FluidMaterial>, With<FluidBackgroundQuad>>,
    mut materials: ResMut<Assets<FluidMaterial>>,
) {
    let Ok(handle) = q_mat.single() else { return; };
    if let Some(mat) = materials.get_mut(&handle.0) { mat.data.time += time.delta_secs(); }
}

fn toggle_background(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ActiveBackground>,
    mut q_grid: Query<&mut Visibility, (With<BackgroundQuad>, Without<FluidBackgroundQuad>)>,
    mut q_fluid: Query<&mut Visibility, (With<FluidBackgroundQuad>, Without<BackgroundQuad>)>,
    mut q_sim: Query<&mut Visibility, (With<fluid_sim::FluidDisplayQuad>, Without<BackgroundQuad>, Without<FluidBackgroundQuad>)>,
) {
    // Toggle with letter B key; adjust variant name if Bevy changes (currently KeyCode::KeyB in newer versions)
    if !keys.just_pressed(KeyCode::KeyB) { return; }
    let new_state = match *state { ActiveBackground::Grid => ActiveBackground::Fluid, ActiveBackground::Fluid => ActiveBackground::FluidSim, ActiveBackground::FluidSim => ActiveBackground::Grid };
    *state = new_state;
    if let Ok(mut vis_grid) = q_grid.single_mut() { *vis_grid = if *state == ActiveBackground::Grid { Visibility::Visible } else { Visibility::Hidden }; }
    if let Ok(mut vis_fluid) = q_fluid.single_mut() { *vis_fluid = if *state == ActiveBackground::Fluid { Visibility::Visible } else { Visibility::Hidden }; }
    if let Ok(mut vis_sim) = q_sim.single_mut() { *vis_sim = if *state == ActiveBackground::FluidSim { Visibility::Visible } else { Visibility::Hidden }; }
    info!("Background toggled to {:?}", *state);
}
