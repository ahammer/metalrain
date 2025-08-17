use bevy::prelude::*;

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

fn setup_ball_materials(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    // Define a palette of 6 colors (modifiable). Keep visually distinct.
    let palette: [Color; 6] = [
        Color::srgb(0.90, 0.20, 0.25), // red
        Color::srgb(0.20, 0.55, 0.90), // blue
        Color::srgb(0.95, 0.75, 0.15), // yellow
        Color::srgb(0.20, 0.80, 0.45), // green
        Color::srgb(0.65, 0.45, 0.95), // purple
        Color::srgb(0.95, 0.50, 0.15), // orange
    ];

    let mut display_handles = Vec::with_capacity(palette.len());
    for c in palette.iter().copied() {
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
