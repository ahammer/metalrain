//! Circle (ball) flat rendering pipeline.
//!
//! Minimal placeholder: spawns a 2D mesh child (unit circle scaled by BallRadius*2) for every Ball added.
//! Future expansions:
//! - Per-ball color indexing via palette & cluster/group identity
//! - Material customization (outline, glow)
//! - GPU instancing path (single mesh + per-instance buffer) replacing many entities
//! - Golden frame hash inclusion once visuals stabilized.

use bevy::prelude::*;
#[cfg(not(any(test, feature = "headless")))]
use bevy::sprite::{ColorMaterial, MeshMaterial2d, Material2dPlugin};
#[cfg(not(any(test, feature = "headless")))]
use bevy::prelude::Mesh2d;
use bm_core::{Ball, BallRadius, BallCircleVisual};

#[cfg(not(any(test, feature = "headless")))]
use bevy::math::primitives::Circle;

#[cfg(not(any(test, feature = "headless")))]
// Resource storing shared unit circle mesh handle
#[derive(Resource)]
struct CircleMeshHandle(Handle<Mesh>);

pub struct CirclesPlugin;

#[cfg(not(any(test, feature = "headless")))]
impl Plugin for CirclesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<ColorMaterial>::default())
            .add_systems(Startup, prepare_circle_mesh)
            .add_systems(Update, spawn_ball_circles);
    }
}

#[cfg(any(test, feature = "headless"))]
impl Plugin for CirclesPlugin {
    fn build(&self, app: &mut App) {
        // Test variant: no render / materials; just spawn marker child for each Added<Ball>
        app.add_systems(Update, |mut commands: Commands, q_new: Query<Entity, Added<Ball>>| {
            for e in &q_new {
                let child = commands.spawn((BallCircleVisual,)).id();
                commands.entity(e).add_child(child);
            }
        });
    }
}

#[cfg(not(any(test, feature = "headless")))]
fn prepare_circle_mesh(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
    commands.insert_resource(CircleMeshHandle(mesh));
}

#[cfg(not(any(test, feature = "headless")))]
fn spawn_ball_circles(
    mut commands: Commands,
    circle_mesh: Option<Res<CircleMeshHandle>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_new_balls: Query<(Entity, &BallRadius), Added<Ball>>,
) {
    let Some(circle_mesh) = circle_mesh else { return; };
    for (entity, radius) in q_new_balls.iter() {
        let color = Color::srgb(0.95, 0.95, 1.0); // placeholder; will use palette indexing later
        let mat_handle = materials.add(ColorMaterial::from(color));
        // Child entity holding visual
        let child = commands
            .spawn((
                Mesh2d::from(circle_mesh.0.clone()),
                MeshMaterial2d(mat_handle),
                Transform::from_scale(Vec3::splat(radius.0 * 2.0)),
                Visibility::Visible,
                InheritedVisibility::VISIBLE,
                BallCircleVisual,
            ))
            .id();
        commands.entity(entity).add_child(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_spawn_for_new_ball() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CirclesPlugin);

        // Insert a Ball entity
        app.world_mut()
            .spawn((Ball, BallRadius(5.0)));

        app.update(); // run systems

        // Expect a BallCircleVisual somewhere
        let world = app.world_mut();
        let mut q = world.query::<&BallCircleVisual>();
        assert_eq!(q.iter(world).count(), 1, "expected one BallCircleVisual spawned");
    }
}
