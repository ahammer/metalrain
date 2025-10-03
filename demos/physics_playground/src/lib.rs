use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

use game_core::{Ball, BallBundle, GameColor, GameState, Wall};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use metaball_renderer::{MetaBall, MetaballRenderSettings, MetaballRendererPlugin};
use game_rendering::{BlendMode, CompositorSettings, GameCamera, GameRenderingPlugin, LayerBlendState, RenderLayer};
use event_core::{EventCorePlugin, EventFlowSet};
use widget_renderer::WidgetRendererPlugin;
use background_renderer::{BackgroundRendererPlugin, BackgroundConfig, BackgroundMode};
use game_assets::GameAssetsPlugin;

pub const DEMO_NAME: &str = "physics_playground";

// Arena configuration constants
const ARENA_HALF_EXTENT: f32 = 400.0;
const WALL_THICKNESS: f32 = 20.0;

// Local game state resource for pause tracking
#[derive(Resource, Default)]
struct PlaygroundState {
    is_paused: bool,
    balls_spawned: u32,
}

// UI marker components
#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct ControlsText;

pub fn run_physics_playground() {
    App::new()
        .insert_resource(game_rendering::RenderSurfaceSettings {
            base_resolution: bevy::math::UVec2::new(1280, 720),
            ..Default::default()
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: "../../assets".into(),
            ..Default::default()
        }))
        .add_plugins(game_assets::GameAssetsPlugin::default())
        .add_plugins(GameRenderingPlugin)
        .add_plugins(BackgroundRendererPlugin)
        .insert_resource(BackgroundConfig {
            mode: BackgroundMode::LinearGradient,
            primary_color: LinearRgba::rgb(0.05, 0.05, 0.15),
            secondary_color: LinearRgba::rgb(0.1, 0.1, 0.2),
            angle: 0.25 * std::f32::consts::PI,
            animation_speed: 0.5,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.75,
        })
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .init_resource::<PlaygroundState>()
        .add_systems(Startup, (setup_camera, setup_arena, setup_ui, spawn_test_balls))
        .add_systems(Update, (exit_on_escape, spawn_ball_on_click, reset_on_key, update_stats_text))
        .run();
}

fn spawn_test_balls(mut commands: Commands) {
    // Spawn a few test balls so there's something to see immediately
    let test_positions = [
        Vec2::new(-100.0, 100.0),
        Vec2::new(0.0, 150.0),
        Vec2::new(100.0, 100.0),
    ];
    
    let test_colors = [GameColor::Red, GameColor::Blue, GameColor::Green];
    
    for (i, &pos) in test_positions.iter().enumerate() {
        let color = test_colors[i];
        let sprite_color = match color {
            GameColor::Red => Color::srgb(1.0, 0.3, 0.3),
            GameColor::Blue => Color::srgb(0.3, 0.4, 1.0),
            GameColor::Green => Color::srgb(0.3, 1.0, 0.4),
            GameColor::Yellow => Color::srgb(1.0, 1.0, 0.3),
            GameColor::White => Color::srgb(0.9, 0.9, 0.9),
        };
        
        let radius = 20.0;
        
        commands.spawn((
            BallBundle::new(pos, radius, color),
            Sprite {
                color: sprite_color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..default()
            },
            RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
            RigidBody::Dynamic,
            Collider::ball(radius),
            Velocity {
                linvel: Vec2::new(0.0, -50.0),
                angvel: 0.0,
            },
            Restitution::coefficient(0.7),
            MetaBall { radius_world: radius },
            Name::new("TestBall"),
        ));
    }
    
    info!("Spawned 3 test balls");
}

fn exit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        GameCamera::default(),
    ));
}

fn setup_arena(mut commands: Commands) {
    let half = ARENA_HALF_EXTENT;
    let thickness = WALL_THICKNESS;
    
    // Bottom wall
    commands.spawn((
        Wall {
            start: Vec2::new(-half, -half),
            end: Vec2::new(half, -half),
            thickness,
            color: Color::srgb(0.3, 0.3, 0.4),
        },
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.4),
            custom_size: Some(Vec2::new(half * 2.0, thickness)),
            ..default()
        },
        RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
        Transform::from_xyz(0.0, -half, 0.0),
        GlobalTransform::IDENTITY,
        RigidBody::Fixed,
        Collider::cuboid(half, thickness / 2.0),
        Name::new("BottomWall"),
    ));
    
    // Top wall
    commands.spawn((
        Wall {
            start: Vec2::new(-half, half),
            end: Vec2::new(half, half),
            thickness,
            color: Color::srgb(0.3, 0.3, 0.4),
        },
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.4),
            custom_size: Some(Vec2::new(half * 2.0, thickness)),
            ..default()
        },
        RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
        Transform::from_xyz(0.0, half, 0.0),
        GlobalTransform::IDENTITY,
        RigidBody::Fixed,
        Collider::cuboid(half, thickness / 2.0),
        Name::new("TopWall"),
    ));
    
    // Left wall
    commands.spawn((
        Wall {
            start: Vec2::new(-half, -half),
            end: Vec2::new(-half, half),
            thickness,
            color: Color::srgb(0.3, 0.3, 0.4),
        },
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.4),
            custom_size: Some(Vec2::new(thickness, half * 2.0)),
            ..default()
        },
        RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
        Transform::from_xyz(-half, 0.0, 0.0),
        GlobalTransform::IDENTITY,
        RigidBody::Fixed,
        Collider::cuboid(thickness / 2.0, half),
        Name::new("LeftWall"),
    ));
    
    // Right wall
    commands.spawn((
        Wall {
            start: Vec2::new(half, -half),
            end: Vec2::new(half, half),
            thickness,
            color: Color::srgb(0.3, 0.3, 0.4),
        },
        Sprite {
            color: Color::srgb(0.3, 0.3, 0.4),
            custom_size: Some(Vec2::new(thickness, half * 2.0)),
            ..default()
        },
        RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
        Transform::from_xyz(half, 0.0, 0.0),
        GlobalTransform::IDENTITY,
        RigidBody::Fixed,
        Collider::cuboid(thickness / 2.0, half),
        Name::new("RightWall"),
    ));
}

fn setup_initial_state(mut commands: Commands) {
    // Insert playground-specific state
    commands.insert_resource(PlaygroundState {
        is_paused: false,
        balls_spawned: 0,
    });
}

fn spawn_ball_on_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }
    
    let Ok(window) = windows.single() else { return; };
    let Some(cursor_pos) = window.cursor_position() else { return; };
    let Ok((camera, camera_transform)) = camera_q.single() else { return; };
    
    // Convert screen position to world position
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { return; };
    
    // Random velocity
    let mut rng = rand::thread_rng();
    let velocity = Vec2::new(
        rng.gen_range(-200.0..200.0),
        rng.gen_range(-200.0..200.0),
    );
    
    // Random color
    let colors = [GameColor::Red, GameColor::Blue, GameColor::Green, GameColor::Yellow, GameColor::White];
    let color = *colors.choose(&mut rng).unwrap();
    
    let radius = rng.gen_range(15.0..30.0);
    
    // Convert GameColor to Color for sprite
    let sprite_color = match color {
        GameColor::Red => Color::srgb(1.0, 0.3, 0.3),
        GameColor::Blue => Color::srgb(0.3, 0.4, 1.0),
        GameColor::Green => Color::srgb(0.3, 1.0, 0.4),
        GameColor::Yellow => Color::srgb(1.0, 1.0, 0.3),
        GameColor::White => Color::srgb(0.9, 0.9, 0.9),
    };
    
    // Spawn ball with physics and metaball components
    commands.spawn((
        BallBundle::new(world_pos, radius, color),
        Sprite {
            color: sprite_color,
            custom_size: Some(Vec2::splat(radius * 2.0)),
            ..default()
        },
        RenderLayers::layer(game_rendering::RenderLayer::GameWorld.order()),
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity {
            linvel: velocity,
            angvel: 0.0,
        },
        Restitution::coefficient(0.7),
        MetaBall { radius_world: radius },
        Name::new("Ball"),
    ));
    
    info!("Spawned ball at {:?} with velocity {:?}", world_pos, velocity);
}

fn reset_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    balls: Query<Entity, With<Ball>>,
    mut playground_state: ResMut<PlaygroundState>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        // Despawn all balls
        for entity in &balls {
            commands.entity(entity).despawn();
        }
        
        // Reset playground state
        playground_state.balls_spawned = 0;
        
        info!("Reset simulation");
    }
}

fn pause_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut playground_state: ResMut<PlaygroundState>,
    mut rapier_config: Query<&mut RapierConfiguration>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        playground_state.is_paused = !playground_state.is_paused;
        
        if let Ok(mut config) = rapier_config.single_mut() {
            config.physics_pipeline_active = !playground_state.is_paused;
        }
        
        info!("Pause: {}", playground_state.is_paused);
    }
}

fn setup_ui(mut commands: Commands) {
    // Stats text in top-left
    commands.spawn((
        Text::new("Stats"),
        TextColor(Color::WHITE),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatsText,
    ));
    
    // Controls text in bottom-left
    commands.spawn((
        Text::new("Controls:\nLeft Click: Spawn Ball\nR: Reset\nP: Pause\nArrows: Adjust Gravity\n+/-: Clustering Strength"),
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ControlsText,
    ));
}

fn adjust_physics_with_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut physics_config: ResMut<PhysicsConfig>,
) {
    // Gravity adjustments
    if keys.pressed(KeyCode::ArrowUp) {
        physics_config.gravity.y += 10.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        physics_config.gravity.y -= 10.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        physics_config.gravity.x -= 10.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        physics_config.gravity.x += 10.0;
    }
    
    // Clustering strength
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        physics_config.clustering_strength = (physics_config.clustering_strength + 5.0).min(500.0);
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        physics_config.clustering_strength = (physics_config.clustering_strength - 5.0).max(0.0);
    }
}

fn update_stats_text(
    mut text_query: Query<&mut Text, With<StatsText>>,
    diagnostics: Res<DiagnosticsStore>,
    balls: Query<&Ball>,
    playground_state: Res<PlaygroundState>,
) {
    let Ok(mut text) = text_query.single_mut() else { return; };
    
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    
    **text = format!(
        "FPS: {:.1}\nBalls: {}\nSpawned: {}",
        fps,
        balls.iter().count(),
        playground_state.balls_spawned,
    );
}
