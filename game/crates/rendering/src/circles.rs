//! Circle (ball) flat rendering pipeline.
//!
//! Minimal placeholder: spawns a 2D mesh child (unit circle scaled by BallRadius*2) for every Ball added.
//! Future expansions:
//! - Per-ball color indexing via palette & cluster/group identity
//! - Material customization (outline, glow)
//! - GPU instancing path (single mesh + per-instance buffer) replacing many entities
//! - Golden frame hash inclusion once visuals stabilized.

use bevy::prelude::*;
#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
use bevy::sprite::{ColorMaterial, MeshMaterial2d};
#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
use bevy::prelude::Mesh2d;
#[allow(unused_imports)]
use bm_core::{Ball, BallRadius, BallCircleVisual};
#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
use bm_core::BallColorIndex;

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
use bevy::math::primitives::Circle;

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
// Resource storing shared unit circle mesh handle
#[derive(Resource)]
struct CircleMeshHandle(Handle<Mesh>);

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
#[derive(Resource, Default)]
struct NextBallColor(pub u8);

pub struct CirclesPlugin;

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
impl Plugin for CirclesPlugin {
    fn build(&self, app: &mut App) {
        // ColorMaterial Material2dPlugin is already provided upstream (e.g. via DefaultPlugins / Sprite).
        // Avoid re-adding it (Bevy panics on duplicate). Just proceed with our resources & systems.
        app.init_resource::<NextBallColor>()
            .add_systems(Startup, prepare_circle_mesh)
            .add_systems(Update, assign_ball_color_index.before(spawn_ball_circles))
            .add_systems(Update, spawn_ball_circles);
    }
}

#[cfg(any(test, feature = "headless", feature = "background_light"))]
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

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
fn prepare_circle_mesh(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
    commands.insert_resource(CircleMeshHandle(mesh));
}

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
fn assign_ball_color_index(
    mut commands: Commands,
    mut next: ResMut<NextBallColor>,
    q_new: Query<Entity, (Added<Ball>, Without<BallColorIndex>)>,
) {
    for e in &q_new {
        let idx = next.0;
        next.0 = next.0.wrapping_add(1);
        commands.entity(e).insert(BallColorIndex(idx));
    }
}

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
fn spawn_ball_circles(
    mut commands: Commands,
    circle_mesh: Option<Res<CircleMeshHandle>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_new_balls: Query<(Entity, &BallRadius, &BallColorIndex), Added<Ball>>,
    #[cfg(all(feature = "instancing", not(any(test, feature = "headless"))))]
    state: Option<Res<crate::instancing::InstancingState>>,
) {
    // If instancing feature is active AND runtime instancing enabled, skip per-entity spawn path.
    #[cfg(all(feature = "instancing", not(any(test, feature = "headless"))))]
    if let Some(st) = state {
        if st.enabled {
            return;
        }
    }
    use crate::palette::color_for_index;
    let Some(circle_mesh) = circle_mesh else { return; };
    for (entity, radius, color_index) in q_new_balls.iter() {
        let color = color_for_index(color_index.0 as usize);
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
