use bevy::prelude::*;
use rand::Rng;

use crate::MockCompositorState;

#[derive(Component)]
pub struct Ball {
    pub velocity: Vec2,
    pub radius: f32,
}

pub fn spawn_visual_simulation(
    mut commands: Commands,
    state: Res<MockCompositorState>,
) {
    let mut rng = rand::thread_rng();
    let window_width = 1280.0;
    let window_height = 720.0;
    
    // Spawn colored balls
    for _ in 0..state.ball_count {
        let x = rng.gen_range(-window_width/2.0 + 50.0..window_width/2.0 - 50.0);
        let y = rng.gen_range(-window_height/2.0 + 100.0..window_height/2.0 - 50.0);
        let vx = rng.gen_range(-200.0..200.0);
        let vy = rng.gen_range(-200.0..200.0);
        let radius = rng.gen_range(8.0..20.0);
        
        // Random color
        let hue = rng.gen_range(0.0..360.0);
        let color = Color::hsl(hue, 0.8, 0.6);
        
        commands.spawn((
            Ball {
                velocity: Vec2::new(vx, vy),
                radius,
            },
            Sprite {
                color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
            Transform::from_xyz(x, y, 0.0),
        ));
    }
    
    info!("Spawned {} balls for visual simulation", state.ball_count);
}

pub fn update_visual_simulation(
    time: Res<Time>,
    state: Res<MockCompositorState>,
    mut query: Query<(&mut Transform, &mut Ball, &mut Sprite)>,
) {
    if state.paused {
        return;
    }
    
    // Hide balls if GameWorld layer is off
    let visibility = state.layer_game_world;
    
    let window_width = 1280.0;
    let window_height = 720.0;
    let delta = time.delta_secs();
    
    let center = Vec2::ZERO;
    
    for (mut transform, mut ball, mut sprite) in query.iter_mut() {
        // Update visibility based on layer
        sprite.color.set_alpha(if visibility { 1.0 } else { 0.0 });
        
        if !visibility {
            continue;
        }
        
        let pos = transform.translation.truncate();
        
        // Apply burst force if active
        if state.active_burst {
            let to_ball = pos - center;
            let dist = to_ball.length();
            if dist < state.burst_radius && dist > 0.1 {
                let force = (to_ball / dist) * state.burst_strength;
                ball.velocity += force * delta;
            }
        }
        
        // Apply wall pulse if active
        if state.active_wall_pulse {
            let left_dist = (pos.x + window_width/2.0).abs();
            let right_dist = (window_width/2.0 - pos.x).abs();
            let top_dist = (window_height/2.0 - pos.y).abs();
            let bottom_dist = (pos.y + window_height/2.0).abs();
            
            let min_dist = left_dist.min(right_dist).min(top_dist).min(bottom_dist);
            
            if min_dist < state.wall_pulse_distance {
                // Push toward center
                let to_center = center - pos;
                if to_center.length() > 0.1 {
                    let force = to_center.normalize() * state.wall_pulse_strength;
                    ball.velocity += force * delta;
                }
            }
        }
        
        // Update position
        transform.translation.x += ball.velocity.x * delta;
        transform.translation.y += ball.velocity.y * delta;
        
        // Bounce off walls
        let half_width = window_width / 2.0;
        let half_height = window_height / 2.0;
        
        if transform.translation.x - ball.radius < -half_width {
            transform.translation.x = -half_width + ball.radius;
            ball.velocity.x = ball.velocity.x.abs();
        }
        if transform.translation.x + ball.radius > half_width {
            transform.translation.x = half_width - ball.radius;
            ball.velocity.x = -ball.velocity.x.abs();
        }
        if transform.translation.y - ball.radius < -half_height {
            transform.translation.y = -half_height + ball.radius;
            ball.velocity.y = ball.velocity.y.abs();
        }
        if transform.translation.y + ball.radius > half_height {
            transform.translation.y = half_height - ball.radius;
            ball.velocity.y = -ball.velocity.y.abs();
        }
        
        // Apply damping
        ball.velocity *= 0.99;
    }
}
