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
