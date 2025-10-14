//! Resource definitions for the compositor test demo.

use bevy::prelude::*;

use crate::constants::*;

/// Tracks state for the periodic burst force effect.
#[derive(Resource, Debug)]
pub struct BurstForceState {
    pub interval_timer: Timer,
    pub active_timer: Option<Timer>,
    pub center: Vec2,
}

impl Default for BurstForceState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(BURST_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
            center: Vec2::ZERO,
        }
    }
}

/// Tracks state for the periodic wall pulse force effect.
#[derive(Resource, Debug)]
pub struct WallPulseState {
    pub interval_timer: Timer,
    pub active_timer: Option<Timer>,
}

impl Default for WallPulseState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(WALL_PULSE_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
        }
    }
}

/// Compositor UI state tracking layers, effects, and visualization modes.
#[derive(Resource, Debug)]
pub struct CompositorState {
    // Layer visibility
    pub layer_background: bool,
    pub layer_game_world: bool,
    pub layer_metaballs: bool,
    pub layer_effects: bool,
    pub layer_ui: bool,

    // Simulation state
    pub paused: bool,
    pub ball_count: usize,
    pub fps: f32,

    // Manual effect triggers
    pub manual_burst_requested: bool,
    pub manual_wall_pulse_requested: bool,

    // Visualization mode
    pub viz_mode: VizMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizMode {
    Normal,
    DistanceField,
    Normals,
    RawCompute,
}

impl Default for CompositorState {
    fn default() -> Self {
        Self {
            layer_background: true,
            layer_game_world: true,
            layer_metaballs: true,
            layer_effects: true,
            layer_ui: true,
            paused: false,
            ball_count: NUM_BALLS,
            fps: 60.0,
            manual_burst_requested: false,
            manual_wall_pulse_requested: false,
            viz_mode: VizMode::Normal,
        }
    }
}
