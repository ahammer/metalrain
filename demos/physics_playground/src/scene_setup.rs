//! Scene setup systems for initializing the physics playground.

use bevy::prelude::*;

use game_core::{BallBundle, GameColor};
use metaball_renderer::MetaBall;

use crate::components::{ControlsText, MousePositionText, StatsText};

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
