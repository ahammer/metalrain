use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use game_core::{ArenaConfig, Ball, Paddle, PaddleControl};

use crate::PhysicsConfig;
use std::collections::HashMap;

pub fn apply_clustering_forces(
    mut query: Query<(&Transform, &mut ExternalForce), With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    if query.is_empty() {
        return;
    }
    if !config.optimize_clustering {
        let positions: Vec<Vec2> = query
            .iter()
            .map(|(t, _)| t.translation.truncate())
            .collect();
        for (i, (transform, mut ext_force)) in query.iter_mut().enumerate() {
            let my_pos = transform.translation.truncate();
            let mut cluster_force = Vec2::ZERO;
            for (j, other_pos) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let delta = *other_pos - my_pos;
                let distance = delta.length();
                if distance > 0.0 && distance < config.clustering_radius {
                    let direction = delta / distance;
                    let strength =
                        (1.0 - distance / config.clustering_radius) * config.clustering_strength;
                    cluster_force += direction * strength;
                }
            }
            ext_force.force = cluster_force;
        }
        return;
    }

    let cell_size = config.clustering_radius.max(1.0);
    let mut positions: Vec<Vec2> = Vec::with_capacity(query.iter().len());
    for (t, _) in query.iter() {
        positions.push(t.translation.truncate());
    }

    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, pos) in positions.iter().enumerate() {
        let cell = (
            (pos.x / cell_size).floor() as i32,
            (pos.y / cell_size).floor() as i32,
        );
        grid.entry(cell).or_default().push(i);
    }

    let mut idx = 0usize;
    for (transform, mut ext_force) in query.iter_mut() {
        let my_pos = transform.translation.truncate();
        let my_cell = (
            (my_pos.x / cell_size).floor() as i32,
            (my_pos.y / cell_size).floor() as i32,
        );
        let mut cluster_force = Vec2::ZERO;
        for ox in -1..=1 {
            for oy in -1..=1 {
                if let Some(indices) = grid.get(&(my_cell.0 + ox, my_cell.1 + oy)) {
                    for &j in indices {
                        if j == idx {
                            continue;
                        }
                        let other_pos = positions[j];
                        let delta = other_pos - my_pos;
                        let distance = delta.length();
                        if distance > 0.0 && distance < config.clustering_radius {
                            let direction = delta / distance;
                            let strength = (1.0 - distance / config.clustering_radius)
                                * config.clustering_strength;
                            cluster_force += direction * strength;
                        }
                    }
                }
            }
        }
        ext_force.force = cluster_force;
        idx += 1;
    }
}

pub fn clamp_velocities(
    mut vel_query: Query<&mut Velocity, With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    for mut vel in vel_query.iter_mut() {
        let lin = vel.linvel.length();
        if lin > config.max_ball_speed {
            vel.linvel = vel.linvel.normalize_or_zero() * config.max_ball_speed;
        }
        if lin > 0.0 && lin < config.min_ball_speed * 0.5 {
            vel.linvel = vel.linvel.normalize_or_zero() * config.min_ball_speed * 0.5;
        }
    }
}

pub fn sync_physics_to_balls(mut query: Query<(&Velocity, &mut Ball)>) {
    for (vel, mut ball) in query.iter_mut() {
        ball.velocity = vel.linvel;
    }
}

pub fn apply_config_gravity(
    mut query: Query<&mut ExternalForce, With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    for mut force in &mut query {
        force.force += config.gravity;
    }
}

pub fn handle_collision_events(
    mut collisions: EventReader<CollisionEvent>,
    _balls: Query<(), With<Ball>>,
) {
    for ev in collisions.read() {
        if let CollisionEvent::Started(_, _, _) = ev {}
    }
}

pub fn spawn_physics_for_new_balls(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    mut q: Query<(Entity, &mut Ball, &Transform), Without<RigidBody>>,
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for (e, mut ball, transform) in &mut q {
        let radius = ball.radius.max(1.0);
        if ball.velocity == Vec2::ZERO {
            ball.velocity = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(0.0..300.0));
        }
        commands.entity(e).insert((
            RigidBody::Dynamic,
            Collider::ball(radius),
            Velocity {
                linvel: ball.velocity,
                angvel: 0.0,
            },
            Restitution {
                coefficient: config.ball_restitution,
                combine_rule: CoefficientCombineRule::Average,
            },
            Friction {
                coefficient: config.ball_friction,
                combine_rule: CoefficientCombineRule::Average,
            },
            ExternalForce::default(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 1.0,
            },
            ActiveEvents::COLLISION_EVENTS,
        ));
        let _ = transform;
    }
}
pub fn attach_paddle_kinematic_physics(
    mut commands: Commands,
    paddles: Query<(Entity, &Paddle), Added<Paddle>>,
) {
    for (e, paddle) in &paddles {
        commands.entity(e).insert((
            RigidBody::KinematicVelocityBased,
            Collider::cuboid(paddle.half_extents.x, paddle.half_extents.y),
            Velocity::zero(),
            Restitution {
                coefficient: 1.1,
                combine_rule: CoefficientCombineRule::Average,
            },
            Friction {
                coefficient: 0.2,
                combine_rule: CoefficientCombineRule::Min,
            },
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

pub fn drive_paddle_velocity(
    keys: Res<ButtonInput<KeyCode>>,
    mut paddles: Query<(&Paddle, &mut Velocity), With<RigidBody>>,
) {
    for (paddle, mut vel) in &mut paddles {
        if !matches!(paddle.control, PaddleControl::Player) {
            continue;
        }
        let mut dir = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }
        vel.linvel = dir * paddle.move_speed;
    }
}

pub fn clamp_paddle_positions(
    arena: Option<Res<ArenaConfig>>,
    mut q: Query<(&Paddle, &mut Transform, &mut Velocity)>,
) {
    let Some(arena) = arena else {
        return;
    };
    for (paddle, mut tf, mut vel) in &mut q {
        let half_w = arena.width * 0.5 - paddle.half_extents.x;
        let half_h = arena.height * 0.5 - paddle.half_extents.y;
        let mut clamped = false;
        let mut pos = tf.translation;
        if pos.x < -half_w {
            pos.x = -half_w;
            vel.linvel.x = vel.linvel.x.max(0.0);
            clamped = true;
        }
        if pos.x > half_w {
            pos.x = half_w;
            vel.linvel.x = vel.linvel.x.min(0.0);
            clamped = true;
        }
        if pos.y < -half_h {
            pos.y = -half_h;
            vel.linvel.y = vel.linvel.y.max(0.0);
            clamped = true;
        }
        if pos.y > half_h {
            pos.y = half_h;
            vel.linvel.y = vel.linvel.y.min(0.0);
            clamped = true;
        }
        if clamped {
            tf.translation = pos;
        }
    }
}
