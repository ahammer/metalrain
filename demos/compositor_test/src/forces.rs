//! Force application systems for burst and wall pulse effects.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

use crate::constants::*;
use crate::resources::{BurstForceState, CompositorState, WallPulseState};

/// Updates the burst force timer and activates new bursts.
pub fn update_burst_force_state(time: Res<Time>, mut state: ResMut<BurstForceState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() {
            state.active_timer = None;
        }
    }
    if state.interval_timer.just_finished() {
        let mut rng = thread_rng();
        let margin = BURST_RADIUS * 0.5;
        let x = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        let y = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        state.center = Vec2::new(x, y);
        state.active_timer = Some(Timer::from_seconds(BURST_ACTIVE_SECONDS, TimerMode::Once));
        info!("Burst force activated at ({x:.1},{y:.1})");
    }
}

/// Applies radial burst forces to nearby rigid bodies.
pub fn apply_burst_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<BurstForceState>,
) {
    let Some(active) = state.active_timer.as_ref() else {
        return;
    };
    if active.finished() {
        return;
    }
    let center = state.center;
    let r2 = BURST_RADIUS * BURST_RADIUS;
    for (tr, mut force) in &mut q {
        let pos = tr.translation.truncate();
        let to_ball = pos - center;
        let dist2 = to_ball.length_squared();
        if dist2 > r2 || dist2 < 1.0 {
            continue;
        }
        let dist = dist2.sqrt();
        let falloff = 1.0 - (dist / BURST_RADIUS);
        let dir = to_ball / dist;
        force.force += dir * BURST_STRENGTH * falloff;
    }
}

/// Updates the wall pulse timer and activates new pulses.
pub fn update_wall_pulse_state(time: Res<Time>, mut state: ResMut<WallPulseState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() {
            state.active_timer = None;
        }
    }
    if state.interval_timer.just_finished() {
        state.active_timer = Some(Timer::from_seconds(
            WALL_PULSE_ACTIVE_SECONDS,
            TimerMode::Once,
        ));
        info!("Wall repulsion pulse active");
    }
}

/// Applies repulsive forces from walls when pulse is active.
pub fn apply_wall_pulse_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<WallPulseState>,
) {
    let Some(active) = state.active_timer.as_ref() else {
        return;
    };
    if active.finished() {
        return;
    }
    let max_dist = WALL_PULSE_DISTANCE;
    for (tr, mut force) in &mut q {
        let p = tr.translation.truncate();
        let mut accum = Vec2::ZERO;

        let left_d = (p.x - (-HALF_EXTENT)).max(0.0);
        if left_d < max_dist {
            let f = 1.0 - left_d / max_dist;
            accum.x += f;
        }
        let right_d = (HALF_EXTENT - p.x).max(0.0);
        if right_d < max_dist {
            let f = 1.0 - right_d / max_dist;
            accum.x -= f;
        }
        let bottom_d = (p.y - (-HALF_EXTENT)).max(0.0);
        if bottom_d < max_dist {
            let f = 1.0 - bottom_d / max_dist;
            accum.y += f;
        }
        let top_d = (HALF_EXTENT - p.y).max(0.0);
        if top_d < max_dist {
            let f = 1.0 - top_d / max_dist;
            accum.y -= f;
        }
        if accum.length_squared() > 0.0001 {
            let magnitude = accum.length();
            let dir = accum / magnitude;
            force.force += dir * WALL_PULSE_STRENGTH * magnitude;
        }
    }
}

/// Handle manual effect triggers from the UI/keyboard shortcuts.
pub fn handle_manual_effect_triggers(
    mut compositor_state: ResMut<CompositorState>,
    mut burst_state: ResMut<BurstForceState>,
    mut wall_pulse_state: ResMut<WallPulseState>,
) {
    // Handle manual burst trigger
    if compositor_state.manual_burst_requested {
        compositor_state.manual_burst_requested = false;

        let mut rng = thread_rng();
        let margin = BURST_RADIUS * 0.5;
        let x = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        let y = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        burst_state.center = Vec2::new(x, y);
        burst_state.active_timer = Some(Timer::from_seconds(BURST_ACTIVE_SECONDS, TimerMode::Once));
        info!("Manual burst force activated at ({x:.1},{y:.1})");
    }

    // Handle manual wall pulse trigger
    if compositor_state.manual_wall_pulse_requested {
        compositor_state.manual_wall_pulse_requested = false;
        wall_pulse_state.active_timer = Some(Timer::from_seconds(
            WALL_PULSE_ACTIVE_SECONDS,
            TimerMode::Once,
        ));
        info!("Manual wall pulse activated");
    }
}
