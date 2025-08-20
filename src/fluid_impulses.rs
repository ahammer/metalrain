use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::components::Ball;
use bevy_rapier2d::prelude::Velocity; // Rapier velocity component
use crate::config::GameConfig;

/// Different semantic kinds of fluid impulses; extended later for directional / dye / swirl.
#[derive(Debug, Clone, Copy)]
pub enum FluidImpulseKind {
    /// Simple radial swirl (tangential) derived from velocity.
    SwirlFromVelocity,
    /// Pure directional push using the ball's velocity vector.
    DirectionalVelocity,
}

/// A single high-level impulse request produced in main world prior to extraction.
#[derive(Debug, Clone, Copy)]
pub struct FluidImpulse {
    pub position: Vec2,   // World position (will be remapped to grid in shader/compute later)
    pub radius: f32,
    pub strength: f32,
    pub kind: FluidImpulseKind,
    pub dir: Vec2, // optional direction; zero for swirl
}

/// Queue of impulses accumulated during a frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct FluidImpulseQueue(pub Vec<FluidImpulse>);

impl FluidImpulseQueue {
    pub fn clear(&mut self) { self.0.clear(); }
    pub fn push(&mut self, imp: FluidImpulse) { self.0.push(imp); }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}

/// System: collect per-ball wake impulses (prototype logic).
/// This is intentionally conservative; we'll refine thresholds once shader support lands.
pub fn collect_ball_wake_impulses(
    mut queue: ResMut<FluidImpulseQueue>,
    q_balls: Query<(&Transform, &Velocity), With<Ball>>,
    cfg: Option<Res<GameConfig>>,
) {
    // Config additions for wake currently absent; use heuristic constants.
    let min_speed = cfg.as_ref().map(|c| c.fluid_sim.force_strength * 0.05).unwrap_or(5.0); // reuse force_strength as proxy
    let radius_scale = 2.0_f32; // arbitrary scaling factor
    let strength_scale = 0.4_f32;
    for (tf, vel) in &q_balls {
        let v = Vec2::new(vel.linvel.x, vel.linvel.y);
        let speed = v.length();
        if speed < min_speed { continue; }
        let pos = tf.translation.truncate();
        queue.push(FluidImpulse {
            position: pos,
            radius: (speed * radius_scale).clamp(8.0, 96.0),
            strength: speed * strength_scale,
            kind: FluidImpulseKind::SwirlFromVelocity,
            dir: v, // raw velocity; swirl shader will turn into tangential
        });
    }
}

/// Extraction step: copy queue into render world (simple clone). Later phases will pack into GPU buffer.
pub fn extract_fluid_impulses(mut commands: Commands, src: bevy::render::Extract<Res<FluidImpulseQueue>>) {
    commands.insert_resource(src.clone());
}

/// Utility system to clear queue after extraction (run end of frame in main world) to avoid accumulation.
pub fn clear_fluid_impulse_queue(mut queue: ResMut<FluidImpulseQueue>) { queue.clear(); }

/// Plugin wiring for Phase 2 (cpu-only, no shader usage yet).
pub struct FluidImpulsesPlugin;
impl Plugin for FluidImpulsesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidImpulseQueue>()
            .add_systems(Update, collect_ball_wake_impulses)
            .add_systems(PostUpdate, clear_fluid_impulse_queue.after(collect_ball_wake_impulses));
        // Add extraction system into render app
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(bevy::render::ExtractSchedule, extract_fluid_impulses);
        }
    }
}

// ------------------------------------------------------------------------------------
// Phase 4 (GPU multi-impulse) groundwork: GPU-ready packed impulse representation.
// This is intentionally not yet wired to any buffers or shaders; we just establish
// a stable, size/alignment-friendly layout so later steps can create a storage buffer
// without churn.
// Layout rationale (repr C): 32 bytes total, 16-byte multiple for predictable strides.
// Fields:
//   pos (vec2)          : 8 bytes
//   radius (f32)        : 4
//   strength (f32)      : 4   -> first 16-byte block
//   dir (vec2)          : 8
//   kind (u32)          : 4
//   _pad (u32)          : 4   -> second 16-byte block (32 total)
// Future extension room: replace _pad with color index / flags without enlarging struct.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuImpulse {
    pub pos: [f32; 2],
    pub radius: f32,
    pub strength: f32,
    pub dir: [f32; 2],
    pub kind: u32,
    pub _pad: u32,
}

/// Maximum number of impulses the GPU path will process per frame (tunable).
/// Chosen to balance buffer size vs. typical ball counts; 256 * 32B = 8KB.
pub const MAX_GPU_IMPULSES: usize = 256;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gpu_impulse_layout() {
        assert_eq!(std::mem::size_of::<GpuImpulse>(), 32, "GpuImpulse must remain 32 bytes");
        assert_eq!(std::mem::align_of::<GpuImpulse>(), 4, "Expected 4-byte alignment (repr C scalar alignment)");
    }
}
