use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::components::Ball;
use crate::config::GameConfig;

pub struct BallSpawnPlugin;

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
) {
    let circle = Mesh::from(Circle { radius: 0.5 });
    let circle_handle = meshes.add(circle);
    let mut rng = rand::thread_rng();
    let c = &cfg.balls;

    for _ in 0..c.count {
    let base_radius = rng.gen_range(c.radius_range.min..c.radius_range.max);
    let radius = base_radius * 2.0; // doubled size
        let x = rng.gen_range(c.x_range.min..c.x_range.max);
        let y = rng.gen_range(c.y_range.min..c.y_range.max);
        let vel = Vec2::new(
            rng.gen_range(c.vel_x_range.min..c.vel_x_range.max),
            rng.gen_range(c.vel_y_range.min..c.vel_y_range.max),
        );
        let color = Color::srgb(
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
        );
        let material = materials.add(color);

        commands
            .spawn((
                Transform::from_translation(Vec3::new(x, y, 0.0)),
                GlobalTransform::default(),
                RigidBody::Dynamic,
                Collider::ball(radius),
                Velocity::linear(vel),
                Restitution::coefficient(cfg.bounce.restitution),
                Damping { linear_damping: 0.0, angular_damping: 0.0 },
                ActiveEvents::COLLISION_EVENTS,
                Ball,
            ))
            .with_children(|parent| {
                parent.spawn(bevy::sprite::MaterialMesh2dBundle {
                    mesh: circle_handle.clone().into(),
                    material: material.clone(),
                    transform: Transform::from_scale(Vec3::splat(radius * 2.0)),
                    ..default()
                });
            });
    }
}
