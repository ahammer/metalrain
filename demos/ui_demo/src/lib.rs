use bevy::prelude::*;
use bevy_hui::prelude::*;

pub const DEMO_NAME: &str = "UI Demo (Bevy-HUI POC)";

mod simulation;
mod ui;

pub use simulation::*;
pub use ui::*;

/// Mock compositor state simulating compositor_test functionality
#[derive(Resource, Debug)]
pub struct MockCompositorState {
    // Layer visibility
    pub layer_background: bool,
    pub layer_game_world: bool,
    pub layer_metaballs: bool,
    pub layer_effects: bool,
    pub layer_ui: bool,
    
    // Effect parameters
    pub burst_interval: f32,
    pub burst_duration: f32,
    pub burst_radius: f32,
    pub burst_strength: f32,
    
    pub wall_pulse_interval: f32,
    pub wall_pulse_duration: f32,
    pub wall_pulse_distance: f32,
    pub wall_pulse_strength: f32,
    
    // Visualization mode
    pub viz_mode: VizMode,
    
    // Simulation state
    pub paused: bool,
    pub ball_count: usize,
    pub fps: f32,
    pub active_burst: bool,
    pub active_wall_pulse: bool,
    
    // Timers for effects
    pub burst_timer: f32,
    pub wall_pulse_timer: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizMode {
    Normal,
    DistanceField,
    Normals,
    RawCompute,
}

impl Default for MockCompositorState {
    fn default() -> Self {
        Self {
            layer_background: true,
            layer_game_world: true,
            layer_metaballs: true,
            layer_effects: true,
            layer_ui: true,
            
            burst_interval: 3.0,
            burst_duration: 0.6,
            burst_radius: 110.0,
            burst_strength: 1400.0,
            
            wall_pulse_interval: 10.0,
            wall_pulse_duration: 0.8,
            wall_pulse_distance: 120.0,
            wall_pulse_strength: 2200.0,
            
            viz_mode: VizMode::Normal,
            paused: false,
            ball_count: 400,
            fps: 60.0,
            active_burst: false,
            active_wall_pulse: false,
            
            burst_timer: 0.0,
            wall_pulse_timer: 0.0,
        }
    }
}

pub fn run_ui_demo() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: DEMO_NAME.to_string(),
                    resolution: (1280.0, 720.0).into(),
                    ..default()
                }),
                ..default()
            }),
            HuiPlugin,
        ))
        .init_resource::<MockCompositorState>()
        .add_systems(Startup, (setup_camera, spawn_visual_simulation, setup_ui))
        .add_systems(Update, (
            handle_keyboard_shortcuts,
            update_fps_counter,
            update_effect_timers,
            update_visual_simulation,
            update_ui_displays,
        ))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn handle_keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MockCompositorState>,
    mut app_exit: EventWriter<AppExit>,
) {
    // Layer toggles (1-5)
    if keys.just_pressed(KeyCode::Digit1) {
        state.layer_background = !state.layer_background;
        info!("Background layer: {}", state.layer_background);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        state.layer_game_world = !state.layer_game_world;
        info!("GameWorld layer: {}", state.layer_game_world);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        state.layer_metaballs = !state.layer_metaballs;
        info!("Metaballs layer: {}", state.layer_metaballs);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        state.layer_effects = !state.layer_effects;
        info!("Effects layer: {}", state.layer_effects);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        state.layer_ui = !state.layer_ui;
        info!("UI layer: {}", state.layer_ui);
    }
    
    // Effect triggers
    if keys.just_pressed(KeyCode::Space) {
        state.active_burst = true;
        state.burst_timer = state.burst_duration;
        info!("Burst force triggered!");
    }
    if keys.just_pressed(KeyCode::KeyW) {
        state.active_wall_pulse = true;
        state.wall_pulse_timer = state.wall_pulse_duration;
        info!("Wall pulse triggered!");
    }
    
    // Simulation controls
    if keys.just_pressed(KeyCode::KeyP) {
        state.paused = !state.paused;
        info!("Simulation paused: {}", state.paused);
    }
    if keys.just_pressed(KeyCode::KeyR) {
        info!("Reset simulation");
        // Will be handled in simulation module
    }
    
    // Visualization mode
    if keys.just_pressed(KeyCode::KeyV) {
        state.viz_mode = match state.viz_mode {
            VizMode::Normal => VizMode::DistanceField,
            VizMode::DistanceField => VizMode::Normals,
            VizMode::Normals => VizMode::RawCompute,
            VizMode::RawCompute => VizMode::Normal,
        };
        info!("Visualization mode: {:?}", state.viz_mode);
    }
    
    // Exit
    if keys.just_pressed(KeyCode::Escape) {
        info!("Exiting UI Demo");
        app_exit.write(AppExit::Success);
    }
}

fn update_fps_counter(
    time: Res<Time>,
    mut state: ResMut<MockCompositorState>,
) {
    let delta = time.delta_secs();
    if delta > 0.0 {
        state.fps = 1.0 / delta;
    }
}

fn update_effect_timers(
    time: Res<Time>,
    mut state: ResMut<MockCompositorState>,
) {
    let delta = time.delta_secs();
    
    if state.active_burst {
        state.burst_timer -= delta;
        if state.burst_timer <= 0.0 {
            state.active_burst = false;
        }
    }
    
    if state.active_wall_pulse {
        state.wall_pulse_timer -= delta;
        if state.wall_pulse_timer <= 0.0 {
            state.active_wall_pulse = false;
        }
    }
}
