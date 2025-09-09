use crate::rendering::palette::palette::BASE_COLORS;
use bevy::prelude::*;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BallMaterialsInitSet;
#[derive(Resource)]
pub struct BallDisplayMaterials(pub Vec<Handle<ColorMaterial>>);
#[derive(Resource, Clone)]
pub struct BallPhysicsMaterials(pub Vec<BallPhysicsMaterial>);
#[derive(Clone, Copy, Debug)]
pub struct BallPhysicsMaterial {
    pub restitution: f32,
}
pub struct MaterialsPlugin;
impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ball_materials.in_set(BallMaterialsInitSet));
    }
}
fn setup_ball_materials(mut materials: ResMut<Assets<ColorMaterial>>, mut commands: Commands) {
    let mut display_handles = Vec::with_capacity(BASE_COLORS.len());
    for c in BASE_COLORS.iter().copied() {
        display_handles.push(materials.add(c));
    }
    let physics_defs = vec![BallPhysicsMaterial { restitution: 0.85 }; display_handles.len()];
    commands.insert_resource(BallDisplayMaterials(display_handles));
    commands.insert_resource(BallPhysicsMaterials(physics_defs));
}
#[derive(Component, Debug, Copy, Clone)]
pub struct BallMaterialIndex(pub usize);
/// Optional shape index for SDF atlas based rendering.
/// When SDF shapes are enabled this u16 value will be packed with the color group id
/// into the metaball storage buffer lane (see schema docs). For now it is stored as
/// its own component to keep spawning logic simple; packing occurs during the GPU
/// buffer build pass.
#[derive(Component, Debug, Copy, Clone, Default)]
pub struct BallShapeIndex(pub u16);
