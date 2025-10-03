use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::window::PrimaryWindow;
use rand::prelude::*;

use game_core::{Ball, BallBundle, GameColor, GameCorePlugin, Wall};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use metaball_renderer::{MetaBall, MetaballRenderSettings, MetaballRendererPlugin};
use game_rendering::{GameCamera, GameRenderingPlugin, RenderLayer};
use event_core::EventCorePlugin;
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
        // Core game components (MUST BE FIRST)
        .add_plugins(GameCorePlugin)
        // Physics (includes RapierPhysicsPlugin internally)
        .add_plugins(GamePhysicsPlugin)
        // Metaball rendering with proper world bounds
        .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings {
            texture_size: bevy::math::UVec2::new(1024, 1024),
            world_bounds: bevy::math::Rect::from_center_size(
                Vec2::ZERO,
                Vec2::splat(ARENA_HALF_EXTENT * 2.0 + 100.0),
            ),
            enable_clustering: true,
            present_via_quad: true,
            presentation_layer: Some(RenderLayer::Metaballs.order() as u8),
        }))
        // Multi-layer compositor
        .add_plugins(GameRenderingPlugin)
        // Background
        .add_plugins(BackgroundRendererPlugin)
        .insert_resource(BackgroundConfig {
            mode: BackgroundMode::LinearGradient,
            primary_color: bevy::color::LinearRgba::rgb(0.05, 0.05, 0.15),
            secondary_color: bevy::color::LinearRgba::rgb(0.1, 0.1, 0.2),
            angle: 0.25 * std::f32::consts::PI,
            animation_speed: 0.5,
            radial_center: Vec2::new(0.5, 0.5),
            radial_radius: 0.75,
        })
        // Widget rendering (walls, targets, etc.)
        .add_plugins(WidgetRendererPlugin)
        // Event system
        .add_plugins(EventCorePlugin::default())
        // Local state
        .init_resource::<PlaygroundState>()
        .add_systems(Startup, (setup_camera, setup_arena, setup_ui, spawn_test_balls))
        .add_systems(Update, (
            exit_on_escape,
            spawn_ball_on_click,
            reset_on_key,
            pause_on_key,
            adjust_physics_with_keys,
            update_stats_text,
        ))
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
        let radius = 20.0;
        
        // Just spawn with BallBundle and MetaBall
        // GamePhysicsPlugin will automatically add RigidBody, Collider, Velocity
        commands.spawn((
            BallBundle::new(pos, radius, color),
            MetaBall { radius_world: radius },
            Name::new("TestBall"),
        ));
    }
    
    info!("Spawned {} test balls", test_positions.len());
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
    
    // Bottom wall - WidgetRendererPlugin will add Sprite automatically
    commands.spawn((
        Wall {
            start: Vec2::new(-half, -half),
            end: Vec2::new(half, -half),
            thickness,
            color: Color::srgb(0.3, 0.3, 0.4),
        },
        Transform::from_xyz(0.0, -half, 0.0),
        bevy_rapier2d::prelude::RigidBody::Fixed,
        bevy_rapier2d::prelude::Collider::cuboid(half, thickness / 2.0),
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
        Transform::from_xyz(0.0, half, 0.0),
        bevy_rapier2d::prelude::RigidBody::Fixed,
        bevy_rapier2d::prelude::Collider::cuboid(half, thickness / 2.0),
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
        Transform::from_xyz(-half, 0.0, 0.0),
        bevy_rapier2d::prelude::RigidBody::Fixed,
        bevy_rapier2d::prelude::Collider::cuboid(thickness / 2.0, half),
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
        Transform::from_xyz(half, 0.0, 0.0),
        bevy_rapier2d::prelude::RigidBody::Fixed,
        bevy_rapier2d::prelude::Collider::cuboid(thickness / 2.0, half),
        Name::new("RightWall"),
    ));
    
    info!("Arena setup complete");
}

fn spawn_ball_on_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut playground_state: ResMut<PlaygroundState>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }
    
    let Ok(window) = windows.single() else {
        error!("No primary window found");
        return;
    };
    
    let Some(cursor_pos) = window.cursor_position() else { 
        warn!("No cursor position");
        return; 
    };
    
    let Ok((camera, camera_transform)) = camera_q.single() else { 
        warn!("No camera found");
        return; 
    };
    
    // Convert screen position to world position
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else { 
        warn!("Failed to convert viewport to world");
        return; 
    };
    
    // Random parameters
    let mut rng = rand::thread_rng();
    let colors = [GameColor::Red, GameColor::Blue, GameColor::Green, GameColor::Yellow, GameColor::White];
    let color = *colors.choose(&mut rng).unwrap();
    let radius = rng.gen_range(15.0..30.0);
    
    // Spawn ball - GamePhysicsPlugin will add RigidBody, Collider, Velocity automatically
    commands.spawn((
        BallBundle::new(world_pos, radius, color),
        MetaBall { radius_world: radius },
        Name::new("Ball"),
    ));
    
    playground_state.balls_spawned += 1;
    
    info!("Spawned ball #{} at {:?}", playground_state.balls_spawned, world_pos);
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
        
        info!("Reset simulation - despawned {} balls", balls.iter().count());
    }
}

fn pause_on_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut rapier_config: Query<&mut bevy_rapier2d::prelude::RapierConfiguration>,
) {
    if keys.just_pressed(KeyCode::KeyP) {
        if let Ok(mut config) = rapier_config.single_mut() {
            config.physics_pipeline_active = !config.physics_pipeline_active;
            info!("Physics paused: {}", !config.physics_pipeline_active);
        }
    }
}

fn setup_ui(mut commands: Commands) {
    // Root container
    commands.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    }).with_children(|parent| {
        // Top stats bar
        parent.spawn((
            Node {
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Stats Loading..."),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                StatsText,
            ));
        });
        
        // Bottom controls bar
        parent.spawn((
            Node {
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Controls: Left Click=Spawn | R=Reset | P=Pause | Arrows=Gravity | +/-=Clustering"),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                ControlsText,
            ));
        });
    });
}

fn adjust_physics_with_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut physics_config: ResMut<PhysicsConfig>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let speed = 500.0;
    
    let mut changed = false;
    
    // Gravity adjustments
    if keys.pressed(KeyCode::ArrowUp) {
        physics_config.gravity.y += speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        physics_config.gravity.y -= speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        physics_config.gravity.x -= speed * delta;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        physics_config.gravity.x += speed * delta;
        changed = true;
    }
    
    // Clustering strength
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        physics_config.clustering_strength = (physics_config.clustering_strength + 100.0 * delta).min(500.0);
        changed = true;
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        physics_config.clustering_strength = (physics_config.clustering_strength - 100.0 * delta).max(0.0);
        changed = true;
    }
    
    if changed {
        info!("Physics - Gravity: ({:.0}, {:.0}), Clustering: {:.0}", 
            physics_config.gravity.x, physics_config.gravity.y, physics_config.clustering_strength);
    }
}

fn update_stats_text(
    mut text_query: Query<&mut Text, With<StatsText>>,
    diagnostics: Res<DiagnosticsStore>,
    balls: Query<&Ball>,
    _playground_state: Res<PlaygroundState>,
    physics_config: Res<PhysicsConfig>,
) {
    let Ok(mut text) = text_query.single_mut() else { return; };
    
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    
    let ball_count = balls.iter().count();
    
    **text = format!(
        "FPS: {:.1} | Balls: {} | Gravity: ({:.0}, {:.0}) | Clustering: {:.0}",
        fps,
        ball_count,
        physics_config.gravity.x,
        physics_config.gravity.y,
        physics_config.clustering_strength,
    );
}
