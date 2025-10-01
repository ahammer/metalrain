use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use game_core::{ArenaConfig, BallBundle, GameColor, GameCorePlugin};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use game_rendering::{GameRenderingPlugin, RenderLayer};
use metaball_renderer::{
    MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin,
};

pub const DEMO_NAME: &str = "physics_playground";

const ARENA_WIDTH: f32 = 512.0;
const ARENA_HEIGHT: f32 = 512.0;
const TEX_SIZE: UVec2 = UVec2::new(512, 512);

pub fn run_physics_playground() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../../assets".into(),
            ..default()
        }))
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(GameRenderingPlugin)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-ARENA_WIDTH * 0.5, -ARENA_HEIGHT * 0.5),
                    Vec2::new(ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5),
                ))
                .clustering_enabled(true)
                .with_presentation(true)
                .with_presentation_layer(RenderLayer::Metaballs.order() as u8),
        ))
        .add_systems(Startup, (setup_board, spawn_initial_balls))
        .run();
}

fn setup_board(mut commands: Commands) {
    commands.insert_resource(ArenaConfig {
        width: ARENA_WIDTH,
        height: ARENA_HEIGHT,
        background: GameColor::White,
    });

    commands.spawn((
        Name::new("PlayfieldBackground"),
        Sprite::from_color(
            Color::srgb(0.07, 0.08, 0.1),
            Vec2::new(ARENA_WIDTH + 40.0, ARENA_HEIGHT + 40.0),
        ),
        Transform::from_xyz(0.0, 0.0, -5.0),
        GlobalTransform::IDENTITY,
        RenderLayers::layer(RenderLayer::Background.order()),
    ));

    let thickness = 20.0;
    let wall_color = Color::srgb(0.2, 0.25, 0.35);
    let half_w = ARENA_WIDTH * 0.5;
    let half_h = ARENA_HEIGHT * 0.5;

    // Horizontal boundaries
    for (name, y) in [("Bottom", -half_h), ("Top", half_h)] {
        commands.spawn((
            Name::new(format!("Wall::{name}")),
            RigidBody::Fixed,
            Collider::cuboid(half_w, thickness * 0.5),
            Sprite::from_color(wall_color, Vec2::new(ARENA_WIDTH, thickness)),
            Transform::from_xyz(0.0, y, 0.0),
            GlobalTransform::IDENTITY,
            RenderLayers::layer(RenderLayer::GameWorld.order()),
        ));
    }

    // Vertical boundaries
    for (name, x) in [("Left", -half_w), ("Right", half_w)] {
        commands.spawn((
            Name::new(format!("Wall::{name}")),
            RigidBody::Fixed,
            Collider::cuboid(thickness * 0.5, half_h),
            Sprite::from_color(wall_color, Vec2::new(thickness, ARENA_HEIGHT)),
            Transform::from_xyz(x, 0.0, 0.0),
            GlobalTransform::IDENTITY,
            RenderLayers::layer(RenderLayer::GameWorld.order()),
        ));
    }
}

fn spawn_initial_balls(mut commands: Commands, config: Res<PhysicsConfig>) {
    let mut rng = rand::thread_rng();
    let positions = [
        Vec2::new(-160.0, 80.0),
        Vec2::new(-40.0, -40.0),
        Vec2::new(80.0, 60.0),
        Vec2::new(180.0, -90.0),
    ];
    for (idx, position) in positions.iter().enumerate() {
        let cluster = idx as i32;
        let color = match idx % 3 {
            0 => GameColor::Red,
            1 => GameColor::Green,
            _ => GameColor::Blue,
        };
        spawn_ball(*position, color, cluster, &mut commands, &config, &mut rng);
    }
}

fn spawn_ball(
    position: Vec2,
    color: GameColor,
    cluster: i32,
    commands: &mut Commands,
    config: &PhysicsConfig,
    rng: &mut impl Rng,
) {
    let radius = rng.gen_range(10.0..18.0);
    let mut bundle = BallBundle::new(position, radius, color);
    bundle.transform.translation.z = 0.05;

    let speed = rng.gen_range(80.0..180.0);
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
    bundle.ball.velocity = velocity;

    let material_color = match color {
        GameColor::Red => Color::srgb(0.92, 0.25, 0.25),
        GameColor::Green => Color::srgb(0.2, 0.85, 0.55),
        GameColor::Blue => Color::srgb(0.3, 0.45, 0.95),
        GameColor::Yellow => Color::srgb(0.95, 0.8, 0.3),
        GameColor::White => Color::srgb(0.9, 0.9, 0.95),
    };
    let linear_color = material_color.to_linear();

    commands.spawn((
        bundle,
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity {
            linvel: velocity,
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
        MetaBall {
            radius_world: radius,
        },
        MetaBallColor(linear_color),
        MetaBallCluster(cluster),
        Name::new("Ball"),
        RenderLayers::layer(RenderLayer::Metaballs.order()),
    ));
}
