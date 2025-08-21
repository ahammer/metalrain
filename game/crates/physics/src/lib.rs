// Phase 2 (scaffold): Physics crate placeholder.
// Goal: Provide Rapier integration plugin surface & system set hooks without real logic yet.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bm_core::{PrePhysicsSet, PostPhysicsAdjustSet};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Add Rapier (placeholder config). Real gravity / tuning will be ported from legacy later.
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0));

        // Placeholder ordering fences (future systems will be inserted into these sets).
        // Example (commented until real systems exist):
        // app.add_systems(Update, apply_radial_forces.in_set(PrePhysicsSet));
        // app.add_systems(Update, separation_adjustments.in_set(PostPhysicsAdjustSet));

        // Ensure sets exist (CorePlugin already added them; if not present this is a no-op).
        // No additional logic yet.
    }
}

// Optional future public API placeholders (kept minimal now)
pub struct PhysicsHandles {
    // Will hold handles / resources for physics configuration
    _placeholder: u8,
}

impl Default for PhysicsHandles {
    fn default() -> Self {
        Self { _placeholder: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_initializes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PhysicsPlugin);
    }
}
