use bevy::prelude::*;

use crate::components::{Ball, Velocity};
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
        let v = **vel; // Velocity deref newtype
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
        if let Ok(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.add_systems(bevy::render::ExtractSchedule, extract_fluid_impulses);
        }
    }
}
