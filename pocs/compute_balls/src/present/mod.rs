use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::AsBindGroup;
use crate::constants::*;
use crate::compute::types::MetaballTarget;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PresentMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

impl Material2d for PresentMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef { "shaders/present_fullscreen.wgsl".into() }
}

pub struct PresentPlugin;

impl Plugin for PresentPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<PresentMaterial>::default())
            .add_systems(PostStartup, setup_present);
    }
}

fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<PresentMaterial>>,
    target: Res<MetaballTarget>,
    mut commands: Commands,
) {
    commands.spawn(Camera2d);
    let quad = Mesh::from(Rectangle::new(WIDTH as f32, HEIGHT as f32));
    let quad_handle = meshes.add(quad);
    let material_handle = materials.add(PresentMaterial { texture: target.texture.clone() });
    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::splat(DISPLAY_SCALE)),
    ));
}
