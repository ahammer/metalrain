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
    pub position: Vec2,   // Grid-space position (0..grid_w, 0..grid_h)
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
    windows: Query<&Window>,
) {
    // IMPORTANT: Clear at start (after last frame's extraction) so impulses remain populated for ExtractSchedule.
    queue.clear();
    // Derive fluid grid resolution from config (avoids coupling on FluidSimSettings type here)
    let (grid_w, grid_h) = cfg.as_ref().map(|c| (c.fluid_sim.width as f32, c.fluid_sim.height as f32)).unwrap_or((256.0, 256.0));
    let Ok(window) = windows.single() else { return; };
    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let inv_w = if window.width() != 0.0 { 1.0 / window.width() } else { 0.0 };
    let inv_h = if window.height() != 0.0 { 1.0 / window.height() } else { 0.0 };

    // Config-driven constants (Phase 4 externalization)
    let (min_speed_factor, radius_scale_world, mut strength_scale, world_r_min, world_r_max, debug_mul) = if let Some(c) = &cfg {
        (
            c.fluid_sim.impulse_min_speed_factor,
            c.fluid_sim.impulse_radius_scale,
        c.fluid_sim.impulse_strength_scale,
            c.fluid_sim.impulse_radius_world_min,
            c.fluid_sim.impulse_radius_world_max,
        c.fluid_sim.impulse_debug_strength_mul,
        )
    } else { (0.05, 2.0, 0.4, 8.0, 96.0, 1.0) };
    strength_scale *= debug_mul.max(0.0);
    let min_speed = cfg.as_ref().map(|c| c.fluid_sim.force_strength * min_speed_factor).unwrap_or(5.0);
    let mut emitted = 0usize;
    for (tf, vel) in &q_balls {
        let v = Vec2::new(vel.linvel.x, vel.linvel.y);
        let speed = v.length();
        if speed < min_speed { continue; }
        let world = tf.translation.truncate();
        // Map world (-half_w..half_w) to grid (0..grid_w), similarly for y
        let gx = ((world.x + half_w) * inv_w) * grid_w;
        let gy = ((world.y + half_h) * inv_h) * grid_h;
        if gx < 0.0 || gx >= grid_w || gy < 0.0 || gy >= grid_h { continue; } // cull off-screen
        // Scale radius from world to grid using average of x/y scales
        let world_radius = (speed * radius_scale_world).clamp(world_r_min, world_r_max); // world units
        let sx = grid_w * inv_w; let sy = grid_h * inv_h; let s_avg = 0.5 * (sx + sy);
        let grid_radius = (world_radius * s_avg).clamp(2.0, grid_w.max(grid_h));
        queue.push(FluidImpulse {
            position: Vec2::new(gx, gy),
            radius: grid_radius,
            strength: speed * strength_scale,
            kind: FluidImpulseKind::SwirlFromVelocity,
            dir: v,
        });
        emitted += 1;
    }
    if emitted > 0 {
    debug!(emitted, total_queue = queue.len(), min_speed, radius_scale_world, strength_scale, debug_mul, "Collected ball wake impulses");
    }
}

/// Extraction step: copy queue into render world (simple clone). Later phases will pack into GPU buffer.
pub fn extract_fluid_impulses(mut commands: Commands, src: bevy::render::Extract<Res<FluidImpulseQueue>>) {
    commands.insert_resource(src.clone());
}

/// Utility system to clear queue after extraction (run end of frame in main world) to avoid accumulation.
pub fn clear_fluid_impulse_queue(mut _queue: ResMut<FluidImpulseQueue>) { /* no-op: clearing now done at collection start */ }

/// Plugin wiring for Phase 2 (cpu-only, no shader usage yet).
pub struct FluidImpulsesPlugin;
impl Plugin for FluidImpulsesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidImpulseQueue>()
            .add_systems(Update, collect_ball_wake_impulses);
        app.add_systems(Update, inject_debug_test_impulse);
        // Add extraction system into render app
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(bevy::render::ExtractSchedule, extract_fluid_impulses);
        }
    }
}

/// Inject a strong central impulse periodically for debugging visibility when enabled in config.
fn inject_debug_test_impulse(
    mut queue: ResMut<FluidImpulseQueue>,
    cfg: Option<Res<GameConfig>>,
    time: Res<Time>,
) {
    let Some(cfg) = cfg else { return; };
    if !cfg.fluid_sim.impulse_debug_test_enabled { return; }
    // Every ~0.5s add a pulse at center of grid
    let t = time.elapsed_secs();
    if (t * 2.0).fract() < 0.02 { // narrow window to avoid flooding
        let gx = cfg.fluid_sim.width as f32 * 0.5;
        let gy = cfg.fluid_sim.height as f32 * 0.5;
        queue.push(FluidImpulse { position: Vec2::new(gx, gy), radius: (cfg.fluid_sim.width.min(cfg.fluid_sim.height) as f32) * 0.15, strength: cfg.fluid_sim.force_strength * 4.0, kind: FluidImpulseKind::DirectionalVelocity, dir: Vec2::X });
        debug!("Injected debug test impulse");
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
