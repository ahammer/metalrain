use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::window::PrimaryWindow;
use rand::prelude::*;

use game_core::{Ball, BallBundle, GameColor, GameCorePlugin, Wall};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use bevy_rapier2d::prelude::{Ccd, RigidBody as RapierRigidBody, Collider as RapierCollider};
use bevy_rapier2d::prelude::RapierDebugRenderPlugin;
use metaball_renderer::{MetaBall, MetaballRenderSettings, MetaballRendererPlugin};
use game_rendering::{GameCamera, GameRenderingPlugin, RenderLayer};
use event_core::EventCorePlugin;
use widget_renderer::WidgetRendererPlugin;
use background_renderer::{BackgroundRendererPlugin, BackgroundConfig, BackgroundMode};

pub const DEMO_NAME: &str = "physics_playground";

const ARENA_HALF_EXTENT: f32 = 400.0;
const WALL_THICKNESS: f32 = 20.0;

#[derive(Resource, Default)]
struct PlaygroundState {
    balls_spawned: u32,
}

#[derive(Component)]
struct StatsText;

#[derive(Component)]
struct ControlsText;

#[derive(Component)]
struct MousePositionText;

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
        .add_plugins(GameCorePlugin)
    .add_plugins(GamePhysicsPlugin)
    .add_plugins(RapierDebugRenderPlugin::default())
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
        .add_plugins(GameRenderingPlugin)
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
        .add_plugins(WidgetRendererPlugin)
        .add_plugins(EventCorePlugin::default())
        .init_resource::<PlaygroundState>()
    .add_systems(Startup, (setup_camera, setup_arena, setup_ui, spawn_test_balls))
        .add_systems(Update, (
            exit_on_escape,
            spawn_ball_on_click,
            reset_on_key,
            pause_on_key,
            adjust_physics_with_keys,
            update_stats_text,
            update_mouse_position_text,
            enable_ccd_for_balls,
        ))
        .run();
}

fn spawn_test_balls(mut commands: Commands) {
    let test_positions = [
        Vec2::new(-150.0, 220.0),
        Vec2::new(0.0, 260.0),
        Vec2::new(150.0, 240.0),
    ];

    let test_colors = [GameColor::Red, GameColor::Blue, GameColor::Green];

    for (i, &pos) in test_positions.iter().enumerate() {
        let color = test_colors[i];
        let radius = 20.0;

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
    commands.spawn((Camera2d, GameCamera::default()));
}

fn setup_arena(mut commands: Commands) {
    let half = ARENA_HALF_EXTENT;
    let phys_pad = 8.0_f32;

    fn spawn_wall(commands: &mut Commands, start: Vec2, end: Vec2, thickness: f32, phys_pad: f32) {
        let delta = end - start;
        let length = delta.length().max(1.0);
        let center = (start + end) * 0.5;
        let angle = delta.y.atan2(delta.x);
        let half_along = length * 0.5;
        let half_across = thickness * 0.5 + phys_pad;
        commands.spawn((
            Wall { start, end, thickness, color: Color::srgb(0.3, 0.3, 0.4) },
            Transform {
                translation: center.extend(0.0),
                rotation: Quat::from_rotation_z(angle),
                ..Default::default()
            },
            GlobalTransform::IDENTITY,
            RapierRigidBody::Fixed,
            RapierCollider::cuboid(half_along, half_across),
            Name::new(format!("Wall({:.0},{:.0})->({:.0},{:.0})", start.x, start.y, end.x, end.y)),
        ));
    }

    spawn_wall(&mut commands, Vec2::new(-half, -half), Vec2::new(half, -half), WALL_THICKNESS, phys_pad);
    spawn_wall(&mut commands, Vec2::new(-half, half), Vec2::new(half, half), WALL_THICKNESS, phys_pad);
    spawn_wall(&mut commands, Vec2::new(-half, -half), Vec2::new(-half, half), WALL_THICKNESS, phys_pad);
    spawn_wall(&mut commands, Vec2::new(half, -half), Vec2::new(half, half), WALL_THICKNESS, phys_pad);

    info!("Arena setup complete");
}

fn spawn_ball_on_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
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

    trace!("Mouse click window pos = ({:.1},{:.1})", cursor_pos.x, cursor_pos.y);

    let Ok((camera, camera_transform)) = camera_q.single() else {
        warn!("No GameCamera found (spawn_ball_on_click)");
        return;
    };

    let world_pos = match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(p) => p,
        Err(_) => match camera.viewport_to_world(camera_transform, cursor_pos) {
            Ok(ray) => ray.origin.truncate(),
            Err(e) => {
                warn!("Failed viewport->world conversion: {e:?}");
                return;
            }
        },
    };

    trace!("World click pos = ({:.1},{:.1})", world_pos.x, world_pos.y);

    let mut rng = rand::thread_rng();
    let colors = [GameColor::Red, GameColor::Blue, GameColor::Green, GameColor::Yellow, GameColor::White];
    let color = *colors.choose(&mut rng).unwrap();
    let radius = rng.gen_range(15.0..30.0);

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
        for entity in &balls {
            commands.entity(entity).despawn();
        }

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
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|root| {
            root
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Text::new("Stats Loading..."),
                        TextColor(Color::WHITE),
                        TextFont { font_size: 20.0, ..default() },
                        StatsText,
                    ));
                    bar.spawn((
                        Text::new("Mouse: ---, ---"),
                        TextColor(Color::WHITE),
                        TextFont { font_size: 20.0, ..default() },
                        MousePositionText,
                    ));
                });

            root
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                ))
                .with_children(|c| {
                    c.spawn((
                        Text::new("Controls: Left Click=Spawn | R=Reset | P=Pause | Arrows=Gravity | +/-=Clustering"),
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        TextFont { font_size: 16.0, ..default() },
                        ControlsText,
                    ));
                });
        });
}

fn enable_ccd_for_balls(mut commands: Commands, q: Query<(Entity, &RapierRigidBody), (With<Ball>, Without<Ccd>)>) {
    for (e, body) in &q {
        if matches!(body, RapierRigidBody::Dynamic) { commands.entity(e).insert(Ccd::enabled()); }
    }
}

fn adjust_physics_with_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut physics_config: ResMut<PhysicsConfig>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();
    let speed = 500.0;

    let mut changed = false;

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
        "FPS: {:03} | Balls: {} | Gravity: ({:.0}, {:.0}) | Clustering: {:.0}",
        fps as u32,
        ball_count,
        physics_config.gravity.x,
        physics_config.gravity.y,
        physics_config.clustering_strength,
    );
}

fn update_mouse_position_text(
    mut text_query: Query<&mut Text, With<MousePositionText>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
) {
    let Ok(mut text) = text_query.single_mut() else { return; };

    let Ok(window) = windows.single() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        **text = "Mouse: ---, ---".to_string();
        return;
    };
    let world_pos = match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
        Ok(p) => p,
        Err(_) => match camera.viewport_to_world(camera_transform, cursor_pos) {
            Ok(ray) => ray.origin.truncate(),
            Err(_) => {
                **text = "Mouse: ---, ---".to_string();
                return;
            }
        },
    };
    **text = format!("Mouse: {:.0}, {:.0}", world_pos.x, world_pos.y);
}
