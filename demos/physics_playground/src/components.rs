//! Component definitions for the physics playground demo.

use bevy::prelude::*;

/// Marker component for the stats text in the UI.
#[derive(Component)]
pub struct StatsText;

/// Marker component for the controls text in the UI.
#[derive(Component)]
pub struct ControlsText;

/// Marker component for the mouse position text in the UI.
#[derive(Component)]
pub struct MousePositionText;
