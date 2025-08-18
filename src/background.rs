use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;

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
    fn fragment_shader() -> ShaderRef { "shaders/bg_worldgrid.wgsl".into() }
    fn vertex_shader() -> ShaderRef { "shaders/bg_worldgrid.wgsl".into() }
}

#[derive(Component)]
struct BackgroundQuad;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<BgMaterial>::default())
            .add_systems(Startup, setup_background)
            .add_systems(Update, resize_bg_uniform);
    }
}

fn setup_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BgMaterial>>,
    windows: Query<&Window>,
) {
    let (w,h) = if let Ok(win) = windows.single() {(win.width(), win.height())} else {(800.0,600.0)};
    let mut mat = BgMaterial::default();
    mat.data.window_size = Vec2::new(w,h);
    let handle = materials.add(mat);
    let mesh = meshes.add(Mesh::from(Rectangle::new(2.0,2.0)));
    commands.spawn((
        Mesh2d::from(mesh),
        MeshMaterial2d(handle),
        Transform::from_xyz(0.0,0.0,-500.0), // very far behind
        Visibility::Visible,
        InheritedVisibility::VISIBLE,
        BackgroundQuad));
    info!("Background quad spawned");

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
