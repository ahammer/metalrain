use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use game_core::{GameCorePlugin, BallBundle, GameColor};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};

const ARENA_WIDTH: f32 = 1200.0;
const ARENA_HEIGHT: f32 = 800.0;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "Physics Playground".into(), ..Default::default() }),
        ..Default::default()
    }));
    app.add_plugins(GameCorePlugin);
    app.add_plugins(GamePhysicsPlugin);
    app.add_plugins(RapierDebugRenderPlugin::default());
    app.add_systems(Startup, setup_walls);
    app.add_systems(Update, handle_spawn_input);
    app.run();
}

fn setup_walls(mut commands: Commands) {
    // Four static colliders forming a bounding box.
    let thickness = 20.0;
    let half_w = ARENA_WIDTH / 2.0;
    let half_h = ARENA_HEIGHT / 2.0;
    let walls = [
        // Floor
        (Vec2::new(0.0, -half_h), Vec2::new(half_w, thickness/2.0)),
        // Ceiling
        (Vec2::new(0.0, half_h), Vec2::new(half_w, thickness/2.0)),
        // Left
        (Vec2::new(-half_w, 0.0), Vec2::new(thickness/2.0, half_h)),
        // Right
        (Vec2::new(half_w, 0.0), Vec2::new(thickness/2.0, half_h)),
    ];
    for (center, half_extents) in walls {
        commands.spawn((
            Transform::from_translation(center.extend(0.0)),
            GlobalTransform::IDENTITY,
            RigidBody::Fixed,
            Collider::cuboid(half_extents.x, half_extents.y),
        ));
    }
}

fn handle_spawn_input(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    config: Res<PhysicsConfig>,
) {
    if !buttons.just_pressed(MouseButton::Left) { return; }
    // Spawn at origin (UI/cursor removed temporarily)
    spawn_ball(Vec2::ZERO, &mut commands, &config);
}

fn spawn_ball(position: Vec2, commands: &mut Commands, config: &PhysicsConfig) {
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(8.0..16.0);
    let color = match rng.gen_range(0..3) { 0 => GameColor::Red, 1 => GameColor::Green, _ => GameColor::Blue };
    let mut bundle = BallBundle::new(position, radius, color);

    // Initial random velocity.
    let initial_velocity = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(0.0..300.0));
    bundle.ball.velocity = initial_velocity;

    commands.spawn((
        bundle,
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity { linvel: initial_velocity, angvel: 0.0 },
        Restitution { coefficient: config.ball_restitution, combine_rule: CoefficientCombineRule::Average },
        Friction { coefficient: config.ball_friction, combine_rule: CoefficientCombineRule::Average },
        ExternalForce::default(),
        Damping { linear_damping: 0.0, angular_damping: 1.0 },
        ActiveEvents::COLLISION_EVENTS,
    ));
}

// (Velocity gizmos temporarily removed pending color API alignment for Bevy 0.16)
