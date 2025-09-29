use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::text::TextFont;
use bevy::ui::{Node, PositionType, Val};
// Diagnostics plugin temporarily removed until version alignment confirmed.
use bevy_rapier2d::prelude::*;
use game_core::Ball;
use metaball_renderer::{
    MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin,
};
use rand::Rng;

use game_core::{BallBundle, GameColor, GameCorePlugin};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};

const ARENA_WIDTH: f32 = 512.0;
const ARENA_HEIGHT: f32 = 512.0;
const TEX_SIZE: UVec2 = UVec2::new(512, 512);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
    // (MetaballShaderSourcePlugin removed – unified asset path loading)
        .add_plugins(DefaultPlugins.set(AssetPlugin { file_path: "../../assets".into(), ..default() }))
        // .add_plugins(FrameTimeDiagnosticsPlugin) // (disabled pending version sync)
        // (No external UI plugin; using built-in Text UI)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-ARENA_WIDTH * 0.5, -ARENA_HEIGHT * 0.5),
                    Vec2::new(ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5),
                ))
                .clustering_enabled(true)
                .with_presentation(true),
        ))
        .add_systems(Startup, spawn_camera)
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(RapierDebugRenderPlugin::default())
        // .add_plugins(RapierDebugRenderPlugin::default()) // optional
        .add_systems(
            Startup,
            (setup_walls, spawn_initial_balls, spawn_config_text),
        )
        .add_systems(
            Update,
            (
                handle_spawn_input,
                handle_control_input,
                stress_test_trigger,
                update_config_text,
            ),
        )
        .run();
}

fn setup_walls(mut commands: Commands) {
    // Four static colliders forming a bounding box.
    let thickness = 20.0;
    let half_w = ARENA_WIDTH / 2.0;
    let half_h = ARENA_HEIGHT / 2.0;
    let walls = [
        // Floor
        (Vec2::new(0.0, -half_h), Vec2::new(half_w, thickness / 2.0)),
        // Ceiling
        (Vec2::new(0.0, half_h), Vec2::new(half_w, thickness / 2.0)),
        // Left
        (Vec2::new(-half_w, 0.0), Vec2::new(thickness / 2.0, half_h)),
        // Right
        (Vec2::new(half_w, 0.0), Vec2::new(thickness / 2.0, half_h)),
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

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Name::new("PhysicsPlaygroundCamera")));
}

fn handle_spawn_input(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    config: Res<PhysicsConfig>,
) {
    if !(buttons.just_pressed(MouseButton::Left) || buttons.just_pressed(MouseButton::Right)) {
        return;
    }
    let window = windows.single().ok();
    let Some(window) = window else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let (camera, cam_transform) = if let Ok(c) = cameras.single() {
        c
    } else {
        return;
    };
    if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
        if buttons.just_pressed(MouseButton::Left) {
            spawn_ball(world_pos, &mut commands, &config, 0);
        }
        if buttons.just_pressed(MouseButton::Right) {
            // Spawn slightly offset below cursor and give velocity toward cursor.
            let spawn_pos = world_pos + Vec2::new(0.0, -50.0);
            let e = spawn_ball(spawn_pos, &mut commands, &config, 1);
            // Overwrite initial velocity to point toward cursor.
            let dir = (world_pos - spawn_pos).normalize_or_zero();
            commands.entity(e).insert(Velocity {
                linvel: dir * 400.0,
                angvel: 0.0,
            });
        }
    }
}

fn handle_control_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<PhysicsConfig>,
    mut commands: Commands,
    balls: Query<Entity, With<Ball>>,
) {
    // Arrow keys adjust gravity components continuously while held.
    let mut changed = false;
    if keys.pressed(KeyCode::ArrowUp) {
        config.gravity.y += 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        config.gravity.y -= 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        config.gravity.x -= 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        config.gravity.x += 10.0;
        changed = true;
    }
    if changed {
        config.gravity = config
            .gravity
            .clamp(Vec2::splat(-1000.0), Vec2::splat(1000.0));
    }

    // +/- adjust clustering strength
    if keys.just_pressed(KeyCode::Equal) {
        config.clustering_strength = (config.clustering_strength + 10.0).min(500.0);
    }
    if keys.just_pressed(KeyCode::Minus) {
        config.clustering_strength = (config.clustering_strength - 10.0).max(0.0);
    }
    // [ ] adjust clustering radius
    if keys.just_pressed(KeyCode::BracketRight) {
        config.clustering_radius = (config.clustering_radius + 10.0).min(400.0);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        config.clustering_radius = (config.clustering_radius - 10.0).max(10.0);
    }

    if keys.just_pressed(KeyCode::KeyG) {
        if config.gravity.length_squared() > 0.0 {
            config.gravity = Vec2::ZERO;
        } else {
            config.gravity = Vec2::new(0.0, -500.0);
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        for e in &balls {
            commands.entity(e).despawn();
        }
    }
}

fn stress_test_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    balls: Query<&Transform, With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }
    let mut rng = rand::thread_rng();
    let target = 60usize;
    let current = balls.iter().len();
    if current < target {
        for i in current..target {
            let x = rng.gen_range(-ARENA_WIDTH * 0.45..ARENA_WIDTH * 0.45);
            let y = rng.gen_range(-ARENA_HEIGHT * 0.45..ARENA_HEIGHT * 0.45);
            spawn_ball(Vec2::new(x, y), &mut commands, &config, (i % 4) as i32);
        }
    }
}

fn spawn_ball(
    position: Vec2,
    commands: &mut Commands,
    config: &PhysicsConfig,
    cluster: i32,
) -> Entity {
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(8.0..16.0);
    let color = match rng.gen_range(0..3) {
        0 => GameColor::Red,
        1 => GameColor::Green,
        _ => GameColor::Blue,
    };
    let mut bundle = BallBundle::new(position, radius, color);

    // Initial random velocity.
    let initial_velocity = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(0.0..300.0));
    bundle.ball.velocity = initial_velocity;

    let entity = commands
        .spawn((
            bundle,
            RigidBody::Dynamic,
            Collider::ball(radius),
            Velocity {
                linvel: initial_velocity,
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
            MetaBallColor(LinearRgba::new(0.8, 0.2, 0.2, 1.0)),
            MetaBallCluster(cluster),
        ))
        .id();
    entity
}

// (Velocity gizmos temporarily removed pending color API alignment for Bevy 0.16)

/// Keep metaball centers in sync with physics-driven transforms.

#[derive(Component)]
struct ConfigText;

fn update_config_text(mut query: Query<&mut Text, With<ConfigText>>, config: Res<PhysicsConfig>) {
    if let Some(mut text) = query.iter_mut().next() {
        text.0 = format!(
            "Gravity: ({:.0},{:.0})  Cluster: str {:.0} rad {:.0}  Speed: min {:.0} max {:.0}\nKeys: Arrows grav  +/- strength  [ ] radius  G toggle grav  R reset  T stress spawn",
            config.gravity.x, config.gravity.y,
            config.clustering_strength, config.clustering_radius,
            config.min_ball_speed, config.max_ball_speed
        );
    }
}

/// Draw velocity vectors & optional clustering radius visualization.
#[allow(dead_code)]
fn draw_debug_gizmos(
    mut gizmos: Gizmos,
    balls: Query<(&Transform, &Velocity)>,
    config: Res<PhysicsConfig>,
) {
    let scale = 0.25; // shorten arrows for readability
    for (tr, vel) in &balls {
        let p = tr.translation.truncate();
        let v = vel.linvel * scale;
        let color = Color::WHITE;
        gizmos.line_2d(p, p + v, color);
        // Draw faint circle for clustering radius (could be heavy; sample subset)
        if balls.iter().len() <= 40 {
            // avoid overdraw spam at high counts
            gizmos.circle_2d(
                p,
                config.clustering_radius,
                Color::linear_rgba(0.5, 0.5, 0.5, 0.2),
            );
        }
    }
}

// world_to_tex & sync system removed – mapping now handled internally during packing.

fn spawn_initial_balls(mut commands: Commands, config: Res<PhysicsConfig>) {
    let mut rng = rand::thread_rng();
    for i in 0..20 {
        // seed some balls so screen isn't empty
        let x = rng.gen_range(-ARENA_WIDTH * 0.45..ARENA_WIDTH * 0.45);
        let y = rng.gen_range(-ARENA_HEIGHT * 0.45..ARENA_HEIGHT * 0.45);
        spawn_ball(Vec2::new(x, y), &mut commands, &config, (i % 4) as i32);
    }
}

fn spawn_config_text(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(4.0),
            left: Val::Px(4.0),
            ..default()
        },
        ConfigText,
    ));
}
