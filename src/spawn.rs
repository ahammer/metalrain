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

    // For omni-directional spawn we interpret configured ranges as window interior size fallback.
    // Spawn positions picked on rectangle perimeter (random edge) then give velocity aiming toward center with noise.
    for _ in 0..c.count {
        let radius = rng.gen_range(c.radius_range.min..c.radius_range.max);
        // Choose an edge: 0=left,1=right,2=bottom,3=top
        let edge = rng.gen_range(0..4);
        let (x, y) = match edge {
            0 => { // left
                let y = rng.gen_range(c.y_range.min..c.y_range.max);
                (c.x_range.min, y)
            }
            1 => { // right
                let y = rng.gen_range(c.y_range.min..c.y_range.max);
                (c.x_range.max, y)
            }
            2 => { // bottom
                let x = rng.gen_range(c.x_range.min..c.x_range.max);
                (x, c.y_range.min)
            }
            _ => { // top
                let x = rng.gen_range(c.x_range.min..c.x_range.max);
                (x, c.y_range.max)
            }
        };
        // Direction toward center (0,0)
        let to_center = Vec2::new(-x, -y).normalize_or_zero();
        // Add random variation scaled by configured velocity ranges.
        let jitter = Vec2::new(
            rng.gen_range(c.vel_x_range.min..c.vel_x_range.max),
            rng.gen_range(c.vel_y_range.min..c.vel_y_range.max),
        );
        let base_speed = jitter.length();
        let vel = to_center * base_speed + jitter * 0.25; // mostly inward with some randomness
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
