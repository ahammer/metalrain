use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;
use metaball_renderer::{MetaBall, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin, MetaballShaderSourcePlugin, MetaBallCluster};

use game_core::{GameCorePlugin, BallBundle, GameColor};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};

const ARENA_WIDTH: f32 = 800.0;
const ARENA_HEIGHT: f32 = 600.0;
const TEX_SIZE: UVec2 = UVec2::new(512,512);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(MetaballShaderSourcePlugin) // must precede DefaultPlugins for custom source
        .add_plugins(DefaultPlugins)
    .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings { present: true, texture_size: TEX_SIZE, enable_clustering: true }))
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        // .add_plugins(RapierDebugRenderPlugin::default()) // optional
        .add_systems(Startup, (setup_walls, spawn_initial_balls))
        .add_systems(Update, (handle_spawn_input, sync_balls_to_metaballs))
        .run();
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
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    config: Res<PhysicsConfig>,
) {
    if !buttons.just_pressed(MouseButton::Left) { return; }
    let window = windows.single().ok();
    let Some(window) = window else { return; };
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let (camera, cam_transform) = if let Ok(c) = cameras.single() { c } else { return };
    if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
        spawn_ball(world_pos, &mut commands, &config, 0);
    }
}

fn spawn_ball(position: Vec2, commands: &mut Commands, config: &PhysicsConfig, cluster: i32) -> Entity {
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(8.0..16.0);
    let color = match rng.gen_range(0..3) { 0 => GameColor::Red, 1 => GameColor::Green, _ => GameColor::Blue };
    let mut bundle = BallBundle::new(position, radius, color);

    // Initial random velocity.
    let initial_velocity = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(0.0..300.0));
    bundle.ball.velocity = initial_velocity;

    let entity = commands.spawn((
        bundle,
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity { linvel: initial_velocity, angvel: 0.0 },
        Restitution { coefficient: config.ball_restitution, combine_rule: CoefficientCombineRule::Average },
        Friction { coefficient: config.ball_friction, combine_rule: CoefficientCombineRule::Average },
        ExternalForce::default(),
        Damping { linear_damping: 0.0, angular_damping: 1.0 },
        ActiveEvents::COLLISION_EVENTS,
        MetaBall { center: world_to_tex(position), radius },
        MetaBallColor(LinearRgba::new(0.8, 0.2, 0.2, 1.0)),
        MetaBallCluster(cluster),
    )).id();
    entity
}

// (Velocity gizmos temporarily removed pending color API alignment for Bevy 0.16)

/// Keep metaball centers in sync with physics-driven transforms.
fn sync_balls_to_metaballs(mut query: Query<(&Transform, &mut MetaBall)>) {
    for (tr, mut mb) in &mut query { mb.center = world_to_tex(tr.translation.truncate()); }
}

fn world_to_tex(p: Vec2) -> Vec2 {
    Vec2::new(
        ((p.x + ARENA_WIDTH * 0.5) / ARENA_WIDTH) * TEX_SIZE.x as f32,
        ((p.y + ARENA_HEIGHT * 0.5) / ARENA_HEIGHT) * TEX_SIZE.y as f32,
    )
}

fn spawn_initial_balls(mut commands: Commands, config: Res<PhysicsConfig>) {
    let mut rng = rand::thread_rng();
    for i in 0..20 { // seed some balls so screen isn't empty
        let x = rng.gen_range(-ARENA_WIDTH*0.45..ARENA_WIDTH*0.45);
        let y = rng.gen_range(-ARENA_HEIGHT*0.45..ARENA_HEIGHT*0.45);
        spawn_ball(Vec2::new(x,y), &mut commands, &config, (i % 4) as i32);
    }
}
