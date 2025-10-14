use bevy::prelude::*;

use crate::MockCompositorState;

pub fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // For this POC, we'll use a simple overlay UI with text displays
    // Since Bevy-HUI doesn't have built-in checkboxes/sliders,
    // we'll create a control panel using buttons and text

    info!("Setting up UI (Note: Templates not yet created)");

    // TODO: Once we understand HUI better, we'll spawn proper templates
    // For now, let's use Bevy's built-in UI as a fallback to demonstrate
    // the concept while we evaluate HUI

    spawn_fallback_ui(&mut commands, &asset_server);
}

/// Fallback UI using Bevy's built-in UI system
/// This demonstrates the intended functionality while we evaluate HUI
fn spawn_fallback_ui(commands: &mut Commands, _asset_server: &Res<AssetServer>) {
    // Root UI node
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            // Status bar at top
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(40.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                ))
                .with_children(|bar| {
                    // Title
                    bar.spawn((
                        Text::new("UI Demo - Bevy-HUI POC"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // FPS counter - will be updated
                    bar.spawn((
                        Text::new("FPS: 60"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.0, 1.0, 0.0)),
                        FpsText,
                    ));

                    // Ball counter - will be updated
                    bar.spawn((
                        Text::new("Balls: 400"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        BallCountText,
                    ));
                });

            // Control panel on left
            parent
                .spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        left: Val::Px(10.0),
                        top: Val::Px(60.0),
                        padding: UiRect::all(Val::Px(15.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                    BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("Controls (Keyboard)"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Instructions
                    panel.spawn((
                        Text::new("1-5: Toggle Layers\nSpace: Burst Force\nW: Wall Pulse\nP: Pause\nR: Reset\nV: Cycle Viz Mode\nEsc: Exit"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));

                    // Layer status
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.9, 1.0)),
                        LayerStatusText,
                    ));
                });

            // Parameter panel on right
            parent
                .spawn((
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        right: Val::Px(10.0),
                        top: Val::Px(60.0),
                        padding: UiRect::all(Val::Px(15.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
                    BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                ))
                .with_children(|panel| {
                    // Title
                    panel.spawn((
                        Text::new("Effect Parameters"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Parameters display
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ParametersText,
                    ));

                    // Active effects
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.5, 0.0)),
                        ActiveEffectsText,
                    ));
                });
        });
}

// Marker components for text elements that need updating
#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct BallCountText;

#[derive(Component)]
pub struct LayerStatusText;

#[derive(Component)]
pub struct ParametersText;

#[derive(Component)]
pub struct ActiveEffectsText;

// System to update UI displays
pub fn update_ui_displays(
    state: Res<MockCompositorState>,
    mut fps_query: Query<&mut Text, (With<FpsText>, Without<BallCountText>, Without<LayerStatusText>, Without<ParametersText>, Without<ActiveEffectsText>)>,
    mut ball_query: Query<&mut Text, (With<BallCountText>, Without<FpsText>, Without<LayerStatusText>, Without<ParametersText>, Without<ActiveEffectsText>)>,
    mut layer_query: Query<&mut Text, (With<LayerStatusText>, Without<FpsText>, Without<BallCountText>, Without<ParametersText>, Without<ActiveEffectsText>)>,
    mut param_query: Query<&mut Text, (With<ParametersText>, Without<FpsText>, Without<BallCountText>, Without<LayerStatusText>, Without<ActiveEffectsText>)>,
    mut effects_query: Query<&mut Text, (With<ActiveEffectsText>, Without<FpsText>, Without<BallCountText>, Without<LayerStatusText>, Without<ParametersText>)>,
) {
    // Update FPS
    if let Ok(mut text) = fps_query.single_mut() {
        **text = format!("FPS: {:.0}", state.fps);
    }

    // Update ball count
    if let Ok(mut text) = ball_query.single_mut() {
        **text = format!("Balls: {}", state.ball_count);
    }

    // Update layer status
    if let Ok(mut text) = layer_query.single_mut() {
        let mut status = String::from("Layers:\n");
        status.push_str(&format!("  BG: {}\n", if state.layer_background { "✓" } else { "✗" }));
        status.push_str(&format!("  World: {}\n", if state.layer_game_world { "✓" } else { "✗" }));
        status.push_str(&format!("  Metaballs: {}\n", if state.layer_metaballs { "✓" } else { "✗" }));
        status.push_str(&format!("  Effects: {}\n", if state.layer_effects { "✓" } else { "✗" }));
        status.push_str(&format!("  UI: {}", if state.layer_ui { "✓" } else { "✗" }));
        **text = status;
    }

    // Update parameters
    if let Ok(mut text) = param_query.single_mut() {
        let params = format!(
            "Burst:\n  Interval: {:.1}s\n  Duration: {:.1}s\n  Radius: {:.0}\n  Strength: {:.0}\n\n\
             Wall Pulse:\n  Interval: {:.1}s\n  Duration: {:.1}s\n  Distance: {:.0}\n  Strength: {:.0}\n\n\
             Viz Mode: {:?}\n\
             Paused: {}",
            state.burst_interval,
            state.burst_duration,
            state.burst_radius,
            state.burst_strength,
            state.wall_pulse_interval,
            state.wall_pulse_duration,
            state.wall_pulse_distance,
            state.wall_pulse_strength,
            state.viz_mode,
            state.paused,
        );
        **text = params;
    }

    // Update active effects
    if let Ok(mut text) = effects_query.single_mut() {
        let mut effects = String::from("Active Effects:\n");
        if state.active_burst {
            effects.push_str("  🔥 BURST FORCE\n");
        }
        if state.active_wall_pulse {
            effects.push_str("  🌊 WALL PULSE\n");
        }
        if !state.active_burst && !state.active_wall_pulse {
            effects.push_str("  None");
        }
        **text = effects;
    }
}
