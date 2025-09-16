use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::AsBindGroup;
use crate::constants::*;
use crate::compute::types::{MetaballTarget, MetaballAlbedoTarget};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MetaballDisplayMaterial {
    #[texture(0)]
    texture: Handle<Image>,
    #[texture(1)]
    #[sampler(2)]
    albedo: Handle<Image>,
}

impl Material2d for MetaballDisplayMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef { "shaders/present_fullscreen.wgsl".into() }
}

pub struct MetaballDisplayPlugin;

impl Plugin for MetaballDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<MetaballDisplayMaterial>::default())
            .add_systems(PostStartup, setup_present);
    }
}

fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
    target: Res<MetaballTarget>,
    albedo: Res<MetaballAlbedoTarget>,
    mut commands: Commands,
) {
    commands.spawn(Camera2d);
    let quad = Mesh::from(Rectangle::new(WIDTH as f32, HEIGHT as f32));
    let quad_handle = meshes.add(quad);

    let material_handle = materials.add(MetaballDisplayMaterial { texture: target.texture.clone(), albedo: albedo.texture.clone() });
    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::splat(1.0)),
    ));
}
