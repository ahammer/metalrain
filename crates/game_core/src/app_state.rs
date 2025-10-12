//! Application-level state machine controlling major app phases.
//! Separate from gameplay state (score, lives) which is tracked in GameState resource.

use bevy::prelude::*;

/// Top-level app state controlling loading and gameplay phases.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    /// Initial state: loading fonts, shaders, and essential assets.
    /// No gameplay or heavy rendering systems run here.
    #[default]
    Loading,

    /// All assets loaded; gameplay and rendering active.
    Playing,
}
