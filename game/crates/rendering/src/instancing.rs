//! GPU instancing scaffold (feature `instancing`).
//!
//! Goal (future):
//! - Replace per-ball spawned Mesh2d child entities with a single batched draw
//!   using a per-instance buffer (positions, radii, color indices).
//! - Greatly reduce entity / archetype churn for large ball counts.
//!
//! Current (no-op) scaffold:
//! - Provides `InstancingState` resource placeholder to evolve without touching
//!   the public `RenderingPlugin` surface.
//! - Plugin inserted only when feature `instancing` is enabled AND we are not in
//!   headless/test builds (those use minimal placeholder rendering).
//!
//! Planned evolution steps (tracked in plan.md):
//! 1. Collect newly spawned `Ball` components into a compact Vec.
//! 2. Maintain GPU buffer (write-on-change) with per-instance data.
//! 3. Custom `Material2d` (or wgpu pipeline) drawing single circle mesh N instanced times.
//! 4. Remove legacy `spawn_ball_circles` system path when stable & benchmarked.

use bevy::prelude::*;

use bm_core::{Ball, BallRadius, BallColorIndex};
use bevy::prelude::GlobalTransform;

/// Internal placeholder state for future batched buffers.
#[derive(Clone, Copy, Debug)]
pub struct BallInstance {
    pub pos: Vec2,
    pub radius: f32,
    pub color_index: u8,
}

#[derive(Resource, Debug)]
pub struct InstancingState {
    /// Count of tracked balls (for early smoke instrumentation).
    pub tracked: usize,
    /// Toggle to temporarily fall back to per-entity path (debug).
    pub enabled: bool,
    /// Collected per-frame instance data (rebuilt each Update for now).
    pub instances: Vec<BallInstance>,
}

impl Default for InstancingState {
    fn default() -> Self {
        Self {
            tracked: 0,
            enabled: true,
            instances: Vec::new(),
        }
    }
}

#[cfg(feature = "instancing")]
pub struct InstancingPlugin;

#[cfg(feature = "instancing")]
impl Plugin for InstancingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InstancingState>()
            .add_systems(Update, track_balls);
    }
}

#[cfg(feature = "instancing")]
fn track_balls(
    mut state: ResMut<InstancingState>,
    q_balls: Query<(&GlobalTransform, &BallRadius, Option<&BallColorIndex>), With<Ball>>,
) {
    if !state.enabled {
        // When disabled we intentionally leave instances Vec empty so fallback path can render per-entity.
        state.tracked = 0;
        state.instances.clear();
        return;
    }
    state.instances.clear();
    for (gt, radius, color_idx) in q_balls.iter() {
        let translation = gt.translation();
        state.instances.push(BallInstance {
            pos: Vec2::new(translation.x, translation.y),
            radius: radius.0,
            color_index: color_idx.map(|c| c.0).unwrap_or(0),
        });
    }
    state.tracked = state.instances.len();
}

#[cfg(test)]
mod tests {
    use super::*;
    // Note: This test only validates resource default when feature is compiled.
    #[test]
    fn state_default() {
        let mut app = App::new();
        app.init_resource::<InstancingState>();
        let state = app.world().get_resource::<InstancingState>().unwrap();
        assert!(state.enabled); // enabled defaults to true
    }
}
