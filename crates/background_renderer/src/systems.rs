use bevy::{
    prelude::*,
    render::{view::RenderLayers, mesh::Mesh2d},
    sprite::MeshMaterial2d,
};

use crate::{config::{BackgroundConfig, BackgroundMode}, material::BackgroundMaterial};

#[derive(Component)]
pub struct BackgroundEntity;

pub fn setup_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    config: Res<BackgroundConfig>,
) {
    let quad = meshes.add(Rectangle::new(2.0, 2.0));
    let mat = materials.add(BackgroundMaterial::from_config(&config, 0.0));
    commands.spawn((
        Mesh2d(quad),
        MeshMaterial2d(mat),
        Transform::from_scale(Vec3::splat(1000.0)),
        RenderLayers::layer(0),
        BackgroundEntity,
        Name::new("Background"),
    ));
    info!("Spawned background with mode {:?}", config.mode);
}

pub fn update_background(
    time: Res<Time>,
    config: Res<BackgroundConfig>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    query: Query<&MeshMaterial2d<BackgroundMaterial>, With<BackgroundEntity>>,
) {
    let animated = matches!(config.mode, BackgroundMode::Animated);
    if !config.is_changed() && !animated { return; }
    let t = time.elapsed_secs();
    for handle in &query {
        if let Some(mat) = materials.get_mut(&handle.0) {
            mat.update_from_config(&config, t);
        }
    }
}

pub fn cleanup_background(mut commands: Commands, q: Query<Entity, With<BackgroundEntity>>) {
    for e in &q { commands.entity(e).despawn(); }
}
