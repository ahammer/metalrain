use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::components::{Ball, BallRadius};
use crate::materials::{BallDisplayMaterials, BallPhysicsMaterials, BallMaterialIndex};
use crate::config::GameConfig;

pub struct BallSpawnPlugin;

#[derive(Resource, Clone)]
pub struct CircleMesh(pub Handle<Mesh>);

impl Plugin for BallSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_balls);
    }
}

fn spawn_balls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GameConfig>,
    display_palette: Option<Res<BallDisplayMaterials>>,
    physics_palette: Option<Res<BallPhysicsMaterials>>,
) {
    // Create shared unit circle mesh once (radius 0.5 so scale = diameter)
    let circle_handle = meshes.add(Mesh::from(Circle { radius: 0.5 }));
    commands.insert_resource(CircleMesh(circle_handle.clone()));
    let mut rng = rand::thread_rng();
    let c = &cfg.balls;

    for _ in 0..c.count {
        let radius = rng.gen_range(c.radius_range.min..c.radius_range.max);
        let x = rng.gen_range(c.x_range.min..c.x_range.max);
        let y = rng.gen_range(c.y_range.min..c.y_range.max);
        let vel = Vec2::new(
            rng.gen_range(c.vel_x_range.min..c.vel_x_range.max),
            rng.gen_range(c.vel_y_range.min..c.vel_y_range.max),
        );
        // Choose material variant index
        let variant_idx = if let Some(p) = &display_palette { rng.gen_range(0..p.0.len()) } else { 0 };
        let (material, restitution) = if let (Some(disp), Some(phys)) = (&display_palette, &physics_palette) {
            (disp.0[variant_idx].clone(), phys.0[variant_idx].restitution)
        } else {
            // Fallback to random color if palettes missing (should not happen after MaterialsPlugin loads)
            let color = Color::srgb(
                rng.gen::<f32>() * 0.9 + 0.1,
                rng.gen::<f32>() * 0.9 + 0.1,
                rng.gen::<f32>() * 0.9 + 0.1,
            );
            (materials.add(color), cfg.bounce.restitution)
        };
        spawn_ball_entity(
            &mut commands,
            &circle_handle,
            Vec3::new(x, y, 0.0),
            vel,
            radius,
            material,
            restitution,
            variant_idx,
            cfg.draw_circles,
        );
    }
}

/// Reusable helper to spawn a single ball.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ball_entity(
    commands: &mut Commands,
    circle_mesh: &Handle<Mesh>,
    translation: Vec3,
    linear_vel: Vec2,
    radius: f32,
    material: Handle<ColorMaterial>,
    restitution: f32,
    variant_idx: usize,
    draw_circles: bool,
) {
    commands
        .spawn((
            Transform::from_translation(translation),
            GlobalTransform::default(),
            VisibilityBundle::default(),
            RigidBody::Dynamic,
            Collider::ball(radius),
            Velocity::linear(linear_vel),
            Restitution::coefficient(restitution),
            Damping { linear_damping: 0.0, angular_damping: 0.0 },
            ActiveEvents::COLLISION_EVENTS,
            Ball,
            BallRadius(radius),
            BallMaterialIndex(variant_idx),
        ))
        .with_children(|parent| {
            if draw_circles {
                parent.spawn(bevy::sprite::MaterialMesh2dBundle {
                    mesh: circle_mesh.clone().into(),
                    material,
                    transform: Transform::from_scale(Vec3::splat(radius * 2.0)),
                    ..default()
                });
            }
        });
}
