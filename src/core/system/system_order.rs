//! Central system ordering labels to make update sequence explicit.
//! Stages (high-level):
//! 1. PrePhysics (custom forces / manual velocity edits before Rapier)
//! 2. Rapier (handled by plugin)
//! 3. PostPhysicsAdjust (lightweight post-physics corrections)
//! 4. Rendering (implicit)
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PrePhysicsSet; // forces applied before physics simulation step

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct PostPhysicsAdjustSet; // lightweight corrections after physics
