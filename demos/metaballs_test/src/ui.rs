use bevy::prelude::*;
use bevy::sprite::MeshMaterial2d;
use metaball_renderer::{MetaballDisplayMaterial, MetaballPresentationQuad, RuntimeSettings};

use crate::simulation::BouncyParams;

/// Marker components for UI text elements
#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct BallCountText;

#[derive(Component)]
pub struct VisualizationModeText;

#[derive(Component)]
pub struct ControlsText;

#[derive(Component)]
pub struct SettingsText;

/// Visualization mode for metaball rendering
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballVizMode {
    /// Final render with lighting and shadows
    Normal,
    /// Raw distance field values
    DistanceField,
    /// 3D normals visualization
    Normals3D,
    /// 2D gradient direction
    Gradient2D,
    /// Albedo/coverage
    Coverage,
    /// Inverse gradient length (SDF approximation)
    InverseGradient,
}

impl MetaballVizMode {
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::DistanceField,
            Self::DistanceField => Self::Normals3D,
            Self::Normals3D => Self::Gradient2D,
            Self::Gradient2D => Self::Coverage,
            Self::Coverage => Self::InverseGradient,
            Self::InverseGradient => Self::Normal,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal (Lit)",
            Self::DistanceField => "Distance Field",
            Self::Normals3D => "3D Normals",
            Self::Gradient2D => "2D Gradient",
            Self::Coverage => "Albedo Coverage",
            Self::InverseGradient => "Inverse Gradient",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Normal => "Full render: lighting, shadows, normals",
            Self::DistanceField => "Scalar field (R channel) Σ(r²/d²)",
            Self::Normals3D => "3D surface normals from height field",
            Self::Gradient2D => "Normalized gradient direction (G,B)",
            Self::Coverage => "Color albedo with field coverage",
            Self::InverseGradient => "1/|∇| for SDF approximation (A)",
        }
    }

    pub fn numpad_key(&self) -> KeyCode {
        match self {
            Self::Normal => KeyCode::Numpad1,
            Self::DistanceField => KeyCode::Numpad2,
            Self::Normals3D => KeyCode::Numpad3,
            Self::Gradient2D => KeyCode::Numpad4,
            Self::Coverage => KeyCode::Numpad5,
            Self::InverseGradient => KeyCode::Numpad6,
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::DistanceField => 1,
            Self::Normals3D => 2,
            Self::Gradient2D => 3,
            Self::Coverage => 4,
            Self::InverseGradient => 5,
        }
    }
}

impl Default for MetaballVizMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Resource, Default)]
pub struct UiState {
    pub viz_mode: MetaballVizMode,
    pub fps: f32,
    pub fps_smoothed: f32,
    pub ball_count: usize,
}

pub struct MetaballUiPlugin;

impl Plugin for MetaballUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Startup, setup_ui)
            .add_systems(
                Update,
                (
                    handle_keyboard_input,
                    update_fps_counter,
                    update_material_viz_mode,
                    update_ui_displays,
                )
                    .chain(),
            );
    }
}

fn setup_ui(mut commands: Commands) {
    // Root UI container
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
            // Top status bar
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
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                ))
                .with_children(|bar| {
                    // Title
                    bar.spawn((
                        Text::new("Metaballs Test - Shader Visualization Demo"),
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

                    // Ball count
                    bar.spawn((
                        Text::new("Balls: 0"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                        BallCountText,
                    ));
                });

            // Left panel - Controls
            parent
                .spawn((
                    Node {
                        width: Val::Px(320.0),
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        left: Val::Px(10.0),
                        top: Val::Px(60.0),
                        padding: UiRect::all(Val::Px(15.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Visualization Controls"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                        ControlsText,
                    ));
                });

            // Right panel - Current mode & settings
            parent
                .spawn((
                    Node {
                        width: Val::Px(380.0),
                        height: Val::Auto,
                        position_type: PositionType::Absolute,
                        right: Val::Px(10.0),
                        top: Val::Px(60.0),
                        padding: UiRect::all(Val::Px(15.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Current Visualization Mode"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 0.8, 1.0)),
                        VisualizationModeText,
                    ));

                    panel.spawn((
                        Text::new("Settings"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));

                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        SettingsText,
                    ));
                });
        });
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Numpad keys for direct mode selection
    for mode in [
        MetaballVizMode::Normal,
        MetaballVizMode::DistanceField,
        MetaballVizMode::Normals3D,
        MetaballVizMode::Gradient2D,
        MetaballVizMode::Coverage,
        MetaballVizMode::InverseGradient,
    ] {
        if keys.just_pressed(mode.numpad_key()) {
            ui_state.viz_mode = mode;
            info!("Switched to visualization mode: {}", mode.name());
        }
    }

    // V key to cycle through modes
    if keys.just_pressed(KeyCode::KeyV) {
        ui_state.viz_mode = ui_state.viz_mode.next();
        info!("Visualization mode: {}", ui_state.viz_mode.name());
    }

    // Escape to exit
    if keys.just_pressed(KeyCode::Escape) {
        app_exit.write(AppExit::Success);
    }
}

fn update_material_viz_mode(
    ui_state: Res<UiState>,
    query: Query<&MeshMaterial2d<MetaballDisplayMaterial>, With<MetaballPresentationQuad>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    
    for material_handle in &query {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.viz_mode = ui_state.viz_mode.as_u32();
        }
    }
}

fn update_fps_counter(time: Res<Time>, mut ui_state: ResMut<UiState>) {
    let delta = time.delta_secs();
    if delta > 0.0 {
        ui_state.fps = 1.0 / delta;
        // Exponential moving average for smoothing
        let alpha = 0.1;
        ui_state.fps_smoothed = alpha * ui_state.fps + (1.0 - alpha) * ui_state.fps_smoothed;
    }
}

fn update_ui_displays(
    ui_state: Res<UiState>,
    bouncy: Res<BouncyParams>,
    runtime: Res<RuntimeSettings>,
    ball_query: Query<&metaball_renderer::MetaBall>,
    mut fps_text: Query<
        &mut Text,
        (
            With<FpsText>,
            Without<BallCountText>,
            Without<VisualizationModeText>,
            Without<ControlsText>,
            Without<SettingsText>,
        ),
    >,
    mut ball_count_text: Query<
        &mut Text,
        (
            With<BallCountText>,
            Without<FpsText>,
            Without<VisualizationModeText>,
            Without<ControlsText>,
            Without<SettingsText>,
        ),
    >,
    mut viz_mode_text: Query<
        &mut Text,
        (
            With<VisualizationModeText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<ControlsText>,
            Without<SettingsText>,
        ),
    >,
    mut controls_text: Query<
        &mut Text,
        (
            With<ControlsText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<VisualizationModeText>,
            Without<SettingsText>,
        ),
    >,
    mut settings_text: Query<
        &mut Text,
        (
            With<SettingsText>,
            Without<FpsText>,
            Without<BallCountText>,
            Without<VisualizationModeText>,
            Without<ControlsText>,
        ),
    >,
) {
    // Update FPS
    if let Ok(mut text) = fps_text.single_mut() {
        **text = format!("FPS: {:.0}", ui_state.fps_smoothed);
    }

    // Update ball count
    let ball_count = ball_query.iter().count();
    if let Ok(mut text) = ball_count_text.single_mut() {
        **text = format!("Balls: {}", ball_count);
    }

    // Update visualization mode display
    if let Ok(mut text) = viz_mode_text.single_mut() {
        **text = format!(
            "🎨 {}\n\n{}\n\nNumpad: Direct select\nV: Cycle modes",
            ui_state.viz_mode.name(),
            ui_state.viz_mode.description()
        );
    }

    // Update controls
    if let Ok(mut text) = controls_text.single_mut() {
        **text = "\
Numpad 1: Normal (Lit)\n\
Numpad 2: Distance Field\n\
Numpad 3: 3D Normals\n\
Numpad 4: 2D Gradient\n\
Numpad 5: Coverage\n\
Numpad 6: Inverse Gradient\n\
\n\
V: Cycle Viz Modes\n\
G: Toggle Gravity\n\
C: Toggle Clustering\n\
H: Toggle Debug Vis\n\
Esc: Exit"
            .to_string();
    }

    // Update settings display
    if let Ok(mut text) = settings_text.single_mut() {
        **text = format!(
            "Gravity: {}\n\
             Clustering: {}\n\
             Balls: {}\n\
             Restitution: {:.2}",
            if bouncy.enable_gravity { "ON" } else { "OFF" },
            if runtime.clustering_enabled {
                "ON"
            } else {
                "OFF"
            },
            ball_count,
            bouncy.restitution
        );
    }
}
