//! Scene setup systems for initializing the physics playground.

use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider as RapierCollider, RigidBody as RapierRigidBody};

use game_core::{BallBundle, GameColor, Wall};
use game_rendering::GameCamera;
use metaball_renderer::MetaBall;

use crate::components::{ControlsText, MousePositionText, StatsText};
use crate::constants::*;

/// Spawns the main 2D camera.
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, GameCamera::default()));
}

/// Sets up the arena boundaries with physics-enabled walls.
pub fn setup_arena(mut commands: Commands) {
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
            Wall {
                start,
                end,
                thickness,
                color: Color::srgb(0.3, 0.3, 0.4),
            },
            Transform {
                translation: center.extend(0.0),
                rotation: Quat::from_rotation_z(angle),
                ..Default::default()
            },
            GlobalTransform::IDENTITY,
            RapierRigidBody::Fixed,
            RapierCollider::cuboid(half_along, half_across),
            Name::new(format!(
                "Wall({:.0},{:.0})->({:.0},{:.0})",
                start.x, start.y, end.x, end.y
            )),
        ));
    }

    spawn_wall(
        &mut commands,
        Vec2::new(-half, -half),
        Vec2::new(half, -half),
        WALL_THICKNESS,
        phys_pad,
    );
    spawn_wall(
        &mut commands,
        Vec2::new(-half, half),
        Vec2::new(half, half),
        WALL_THICKNESS,
        phys_pad,
    );
    spawn_wall(
        &mut commands,
        Vec2::new(-half, -half),
        Vec2::new(-half, half),
        WALL_THICKNESS,
        phys_pad,
    );
    spawn_wall(
        &mut commands,
        Vec2::new(half, -half),
        Vec2::new(half, half),
        WALL_THICKNESS,
        phys_pad,
    );

    info!("Arena setup complete");
}

/// Spawns initial test balls in the arena.
pub fn spawn_test_balls(mut commands: Commands) {
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
            MetaBall {
                radius_world: radius,
            },
            Name::new("TestBall"),
        ));
    }

    info!("Spawned {} test balls", test_positions.len());
}

/// Sets up the UI overlay with stats and controls.
pub fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|root| {
            root.spawn((
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
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    StatsText,
                ));
                bar.spawn((
                    Text::new("Mouse: ---, ---"),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    MousePositionText,
                ));
            });

            root.spawn((
                Node {
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            ))
            .with_children(|c| {
                c.spawn((
                    Text::new(
                        "Controls: Left Click=Spawn | R=Reset | P=Pause | Arrows=Gravity | +/-=Clustering",
                    ),
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
