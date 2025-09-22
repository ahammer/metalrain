use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

// Logical half-extent of the square world: coordinates range roughly in [-HALF_EXTENT, HALF_EXTENT]
const HALF_EXTENT: f32 = 256.0; // mirrors metaballs demo scale
const WALL_THICKNESS: f32 = 10.0;
const NUM_BALLS: usize = 400; // reasonable default; adjust as desired
// We rely on Rapier's default gravity (approx -9.81 on Y). To exaggerate the effect, we apply a GravityScale > 1 on each ball.
const GRAVITY_SCALE: f32 = 35.0; // amplifies default gravity strength per ball

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .add_plugins(RapierDebugRenderPlugin::default()) // uncomment for collider visualization
        .add_systems(Startup, (setup_camera, spawn_walls, spawn_balls))
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Bevy 0.16 uses component-style camera spawning.
    commands.spawn((Camera2d,));
}

fn spawn_walls(mut commands: Commands) {
    // Four axis-aligned fixed walls forming a box.
    // Use large cuboids slightly beyond the half-extent to ensure containment.
    let hx = HALF_EXTENT;
    let hy = HALF_EXTENT;
    let t = WALL_THICKNESS;

    // Bottom
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(hx + t, t),
        Transform::from_translation(Vec3::new(0.0, -hy - t, 0.0)),
        GlobalTransform::default(),
        Name::new("WallBottom"),
    ));
    // Top
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(hx + t, t),
        Transform::from_translation(Vec3::new(0.0, hy + t, 0.0)),
        GlobalTransform::default(),
        Name::new("WallTop"),
    ));
    // Left
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(t, hy + t),
        Transform::from_translation(Vec3::new(-hx - t, 0.0, 0.0)),
        GlobalTransform::default(),
        Name::new("WallLeft"),
    ));
    // Right
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(t, hy + t),
        Transform::from_translation(Vec3::new(hx + t, 0.0, 0.0)),
        GlobalTransform::default(),
        Name::new("WallRight"),
    ));
}

fn spawn_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    for i in 0..NUM_BALLS {
        let radius = rng.gen_range(3.0..12.0);
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(10.0..120.0);
        let vel = Vec2::from_angle(angle) * speed;

        commands.spawn((
            RigidBody::Dynamic,
            Collider::ball(radius),
            Restitution::coefficient(0.8),
            Damping { linear_damping: 0.2, angular_damping: 0.8 },
            Velocity { linvel: vel, angvel: rng.gen_range(-5.0..5.0) },
            GravityScale(GRAVITY_SCALE),
            Ccd::disabled(),
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            GlobalTransform::default(),
            Name::new(format!("Ball#{i}")),
        ));
    }
    info!("Spawned {NUM_BALLS} balls in GameBoard_Test demo");
}
