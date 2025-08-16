use bevy::prelude::*;

use crate::components::{Ball, Velocity};
use crate::config::GameConfig;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_gravity, move_balls, bounce_on_bounds));
    }
}

fn apply_gravity(time: Res<Time>, cfg: Res<GameConfig>, mut q: Query<&mut Velocity, With<Ball>>) {
    let dt = time.delta_secs();
    for mut v in &mut q {
        v.y += cfg.gravity.y * dt;
    }
}

fn move_balls(time: Res<Time>, mut q: Query<(&mut Transform, &Velocity), With<Ball>>) {
    let dt = time.delta_secs();
    for (mut tf, vel) in &mut q {
        tf.translation.x += vel.x * dt;
        tf.translation.y += vel.y * dt;
    }
}

fn bounce_on_bounds(
    mut q: Query<(&mut Transform, &mut Velocity), With<Ball>>,
    windows: Query<&Window>,
    cfg: Res<GameConfig>,
) {
    let window = windows.single().expect("primary window");
    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let restitution = cfg.bounce.restitution;

    for (mut tf, mut vel) in &mut q {
        let radius = tf.scale.x * 0.5; // diameter scaling
        let mut bounced = false;
        if tf.translation.x - radius < -half_w {
            tf.translation.x = -half_w + radius;
            vel.x = -vel.x * restitution;
            bounced = true;
        } else if tf.translation.x + radius > half_w {
            tf.translation.x = half_w - radius;
            vel.x = -vel.x * restitution;
            bounced = true;
        }
        if tf.translation.y - radius < -half_h {
            tf.translation.y = -half_h + radius;
            vel.y = -vel.y * restitution;
            bounced = true;
        } else if tf.translation.y + radius > half_h {
            tf.translation.y = half_h - radius;
            vel.y = -vel.y * restitution;
            bounced = true;
        }
        if bounced && vel.length_squared() < 1.0 {
            vel.x = 0.0;
            vel.y = 0.0;
        }
    }
}
