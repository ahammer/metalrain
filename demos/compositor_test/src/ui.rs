//! UI setup and update systems for the compositor test demo.

use bevy::prelude::*;

use crate::resources::{BurstForceState, CompositorState, WallPulseState};

/// Sets up the UI overlay using Bevy's built-in UI system.
/// This provides a control panel and status displays for the compositor demo.
pub fn setup_ui(mut commands: Commands) {

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
                        Text::new("Compositor Test - Layered Rendering Demo"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // FPS counter
                    bar.spawn((
                        Text::new("FPS: 60"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.0, 1.0, 0.0)),
                        FpsText,
                    ));

                    // Ball counter
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
                        Text::new(
                            "1-3: Toggle Layers\n\
                             Space: Manual Burst\n\
                             W: Manual Wall Pulse\n\
                             P: Pause Simulation\n\
                             V: Cycle Viz Mode\n\
                             Esc: Exit",
                        ),
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

            // Effect status panel on right
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
                        Text::new("Effect Status"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Effect parameters
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        EffectParametersText,
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
pub struct EffectParametersText;

#[derive(Component)]
pub struct ActiveEffectsText;

/// System to update UI text displays based on current state.
pub fn update_ui_displays(
    state: Res<CompositorState>,
    burst_state: Res<BurstForceState>,
    wall_pulse_state: Res<WallPulseState>,
    mut fps_query: Query<
        &mut Text,
        (
            With<FpsText>,
            Without<BallCountText>,
            Without<LayerStatusText>,
            Without<EffectParametersText>,
            Without<ActiveEffectsText>,
        ),
    >,
    mut ball_query: Query<
        &mut Text,
        (
            With<BallCountText>,
            Without<FpsText>,
            Without<LayerStatusText>,
            Without<EffectParametersText>,
            Without<ActiveEffectsText>,
        ),
    >,
    mut layer_query: Query<
        &mut Text,
        (
            With<LayerStatusText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<EffectParametersText>,
            Without<ActiveEffectsText>,
        ),
    >,
    mut param_query: Query<
        &mut Text,
        (
            With<EffectParametersText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<LayerStatusText>,
            Without<ActiveEffectsText>,
        ),
    >,
    mut effects_query: Query<
        &mut Text,
        (
            With<ActiveEffectsText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<LayerStatusText>,
            Without<EffectParametersText>,
        ),
    >,
) {
    // Update FPS (using smoothed value)
    if let Ok(mut text) = fps_query.single_mut() {
        **text = format!("FPS: {:.0}", state.fps_smoothed);
    }

    // Update ball count
    if let Ok(mut text) = ball_query.single_mut() {
        **text = format!("Balls: {}", state.ball_count);
    }

    // Update layer status with clear enabled/disabled indicators
    if let Ok(mut text) = layer_query.single_mut() {
        let mut status = String::from("Layers:\n");
        status.push_str(&format!(
            "  [{}] Background\n",
            if state.layer_background { "ON " } else { "OFF" }
        ));
        status.push_str(&format!(
            "  [{}] GameWorld\n",
            if state.layer_game_world { "ON " } else { "OFF" }
        ));
        status.push_str(&format!(
            "  [{}] Metaballs\n",
            if state.layer_metaballs { "ON " } else { "OFF" }
        ));
        // Effects & UI layers removed
        **text = status;
    }

    // Update effect parameters
    if let Ok(mut text) = param_query.single_mut() {
        let burst_active = burst_state.active_timer.is_some();
        let wall_pulse_active = wall_pulse_state.active_timer.is_some();

        let params = format!(
            "Burst Force:\n\
               Auto Interval: {:.1}s\n\
               Duration: {:.1}s\n\
               Status: {}\n\n\
             Wall Pulse:\n\
               Auto Interval: {:.1}s\n\
               Duration: {:.1}s\n\
               Status: {}\n\n\
             Viz Mode: {:?}\n\
             Paused: {}",
            burst_state.interval_timer.duration().as_secs_f32(),
            burst_state
                .active_timer
                .as_ref()
                .map(|t| t.duration().as_secs_f32())
                .unwrap_or(0.6),
            if burst_active { "🔥 ACTIVE" } else { "Idle" },
            wall_pulse_state.interval_timer.duration().as_secs_f32(),
            wall_pulse_state
                .active_timer
                .as_ref()
                .map(|t| t.duration().as_secs_f32())
                .unwrap_or(0.8),
            if wall_pulse_active {
                "🌊 ACTIVE"
            } else {
                "Idle"
            },
            state.viz_mode,
            state.paused,
        );
        **text = params;
    }

    // Update active effects
    if let Ok(mut text) = effects_query.single_mut() {
        let mut effects = String::from("Active Effects:\n");

        if burst_state.active_timer.is_some() {
            effects.push_str("  🔥 BURST FORCE\n");
        }
        if wall_pulse_state.active_timer.is_some() {
            effects.push_str("  🌊 WALL PULSE\n");
        }
        if state.manual_burst_requested {
            effects.push_str("  ⚡ Manual Burst Queued\n");
        }
        if state.manual_wall_pulse_requested {
            effects.push_str("  ⚡ Manual Pulse Queued\n");
        }

        if burst_state.active_timer.is_none()
            && wall_pulse_state.active_timer.is_none()
            && !state.manual_burst_requested
            && !state.manual_wall_pulse_requested
        {
            effects.push_str("  None");
        }

        **text = effects;
    }
}

/// Handle keyboard shortcuts for layer toggles, effects, and controls.
pub fn handle_keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CompositorState>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Layer toggles (1-5)
    if keys.just_pressed(KeyCode::Digit1) {
        state.layer_background = !state.layer_background;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        state.layer_game_world = !state.layer_game_world;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        state.layer_metaballs = !state.layer_metaballs;
    }
    // Layers 4 & 5 removed

    // Manual effect triggers
    if keys.just_pressed(KeyCode::Space) {
        state.manual_burst_requested = true;
    }
    if keys.just_pressed(KeyCode::KeyW) {
        state.manual_wall_pulse_requested = true;
    }

    // Simulation controls
    if keys.just_pressed(KeyCode::KeyP) {
        state.paused = !state.paused;
    }

    // Visualization mode
    if keys.just_pressed(KeyCode::KeyV) {
        use crate::resources::VizMode;
        state.viz_mode = match state.viz_mode {
            VizMode::Normal => VizMode::DistanceField,
            VizMode::DistanceField => VizMode::Normals,
            VizMode::Normals => VizMode::RawCompute,
            VizMode::RawCompute => VizMode::Normal,
        };
    }

    // Exit
    if keys.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
    }
}

/// Update FPS counter from frame time with exponential moving average smoothing.
pub fn update_fps_counter(time: Res<Time>, mut state: ResMut<CompositorState>) {
    let delta = time.delta_secs();
    if delta > 0.0 {
        state.fps = 1.0 / delta;

        // Apply exponential moving average for smooth FPS display
        // Alpha of 0.1 means we smooth over roughly 10 frames
        let alpha = 0.1;
        state.fps_smoothed = alpha * state.fps + (1.0 - alpha) * state.fps_smoothed;
    }
}
