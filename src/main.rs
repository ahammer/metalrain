use bevy::prelude::*;
use rand::Rng;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;
const BALL_COUNT: usize = 150;
const GRAVITY: f32 = -600.0; // units per second^2 (Y down)
const RESTITUTION: f32 = 0.85; // energy retained on bounce

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Ball;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Bouncing Balls".into(),
                    resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            }),
        )
        .add_systems(Startup, (setup_camera, spawn_balls))
        .add_systems(Update, (apply_gravity, move_balls, bounce_on_bounds))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_balls(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    // Circle mesh reused for all balls (unit circle scaled via Transform)
    let circle = Mesh::from(Circle { radius: 0.5 });
    let circle_handle = meshes.add(circle);
    let mut rng = rand::thread_rng();

    for _ in 0..BALL_COUNT {
        let radius = rng.gen_range(5.0..25.0);
        let x = rng.gen_range(-WINDOW_WIDTH * 0.45..WINDOW_WIDTH * 0.45);
        let y = rng.gen_range(-WINDOW_HEIGHT * 0.45..WINDOW_HEIGHT * 0.45);
        let vel = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(-50.0..350.0));

        let color = Color::srgb(
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
        );
        let material = materials.add(color);

        commands.spawn((
            Mesh2d(circle_handle.clone()),
            MeshMaterial2d(material),
            Transform::from_translation(Vec3::new(x, y, 0.0)).with_scale(Vec3::splat(radius * 2.0)),
            GlobalTransform::default(),
            Velocity(vel),
            Ball,
        ));
    }
}

fn apply_gravity(time: Res<Time>, mut q: Query<&mut Velocity, With<Ball>>) {
    let dt = time.delta_secs();
    for mut v in &mut q {
        v.y += GRAVITY * dt;
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
) {
    let window = windows.single().expect("primary window");
    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;

    for (mut tf, mut vel) in &mut q {
        let radius = tf.scale.x * 0.5; // since we scaled diameter
        let mut bounced = false;
        // X bounds
        if tf.translation.x - radius < -half_w {
            tf.translation.x = -half_w + radius;
            vel.x = -vel.x * RESTITUTION;
            bounced = true;
        } else if tf.translation.x + radius > half_w {
            tf.translation.x = half_w - radius;
            vel.x = -vel.x * RESTITUTION;
            bounced = true;
        }
        // Y bounds
        if tf.translation.y - radius < -half_h {
            tf.translation.y = -half_h + radius;
            vel.y = -vel.y * RESTITUTION;
            bounced = true;
        } else if tf.translation.y + radius > half_h {
            tf.translation.y = half_h - radius;
            vel.y = -vel.y * RESTITUTION;
            bounced = true;
        }

        if bounced {
            // Minor damping threshold cleanup
            if vel.length_squared() < 1.0 {
                vel.x = 0.0;
                vel.y = 0.0;
            }
        }
    }
}
