//! Resource definitions for the physics playground demo.

use bevy::prelude::*;

/// Tracks the state of the playground demo.
#[derive(Resource, Default)]
pub struct PlaygroundState {
    pub balls_spawned: u32,
}
