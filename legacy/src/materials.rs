use crate::palette::BASE_COLORS;
use bevy::prelude::*; // added

// System set to ensure material palette is initialized before other Startup systems that depend on it.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BallMaterialsInitSet;

/// Resource containing the visual color materials used for balls.
#[derive(Resource)]
pub struct BallDisplayMaterials(pub Vec<Handle<ColorMaterial>>);

/// Resource containing physics restitution / friction pairs per variant.
#[derive(Resource, Clone)]
pub struct BallPhysicsMaterials(pub Vec<BallPhysicsMaterial>);

/// Simple physics material definition (can be extended later with friction, density, etc.).
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
    // Use centralized palette
    let mut display_handles = Vec::with_capacity(BASE_COLORS.len());
    for c in BASE_COLORS.iter().copied() {
        display_handles.push(materials.add(c));
    }

    // Physics material list mirrors palette length; same restitution/friction for now (can vary later).
    let physics_defs = vec![BallPhysicsMaterial { restitution: 0.85 }; display_handles.len()];

    commands.insert_resource(BallDisplayMaterials(display_handles));
    commands.insert_resource(BallPhysicsMaterials(physics_defs));
}

/// Component storing the index into BallDisplayMaterials / BallPhysicsMaterials arrays.
#[derive(Component, Debug, Copy, Clone)]
#[allow(dead_code)] // reserved for future classification / queries
pub struct BallMaterialIndex(pub usize);
