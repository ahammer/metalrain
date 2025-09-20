use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use crate::internal::{FieldTexture, AlbedoTexture};
use crate::settings::MetaballRenderSettings;
use crate::embedded_shaders;
use bevy::window::PrimaryWindow;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MetaballDisplayMaterial {
    #[texture(0)]
    texture: Handle<Image>,
    #[texture(1)]
    #[sampler(2)]
    albedo: Handle<Image>
}

impl Material2d for MetaballDisplayMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(embedded_shaders::present_handle())
    }
}

pub struct MetaballDisplayPlugin;

impl Plugin for MetaballDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<MetaballDisplayMaterial>::default()).add_systems(PostStartup, (setup_present,));
    }
}

#[derive(Component)]
struct MetaballCameraTag;

fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
    field: Res<FieldTexture>,
    albedo: Res<AlbedoTexture>,
    mut commands: Commands,
    _settings: Res<MetaballRenderSettings>,
    existing_cameras: Query<Entity, With<Camera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if existing_cameras.is_empty() {
        commands.spawn((Camera2d, MetaballCameraTag));
    }
    // Safely get the primary window; if it's not yet available, abort setup for this frame.
    let Ok(window) = windows.single() else { return; };
    let quad = Mesh::from(Rectangle::new(window.width(), window.height()));
    let quad_handle = meshes.add(quad);
    let material_handle = materials.add(MetaballDisplayMaterial {
        texture: field.0.clone(),
        albedo: albedo.0.clone()
    });
    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::splat(1.0))
    ));
}
