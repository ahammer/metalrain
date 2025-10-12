//! Resource definitions for the compositor test demo.

use bevy::prelude::*;
use game_rendering::BlendMode;
use std::collections::VecDeque;

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

/// Controls visibility of the performance overlay.
#[derive(Resource, Debug)]
pub struct PerformanceOverlayState {
    pub visible: bool,
}

impl Default for PerformanceOverlayState {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Tracks performance statistics over time.
#[derive(Resource, Debug, Default)]
pub struct PerformanceStats {
    pub frames: u64,
    pub last_sample_time: f32,
    pub recent: VecDeque<(f32, f32)>,
}

/// Caches HUD state to avoid unnecessary rebuilds.
#[derive(Resource, Debug, Default, Clone)]
pub struct LayerHudCache {
    pub last_enabled: [bool; 5],
    pub last_blends: [BlendMode; 5],
    pub last_exposure: f32,
    pub last_boundary_debug: bool,
    pub last_camera_scale: f32,
    pub last_text: String,
}

/// Simple frame counter for periodic logging.
#[derive(Resource, Debug, Default)]
pub struct FrameCounter {
    pub frame: u64,
}
