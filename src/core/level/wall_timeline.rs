use bevy::prelude::*;

use super::layout::{RepeatModeDef, TransformTimelineDef};

// ---------------- Runtime Components -----------------

#[derive(Component)]
pub struct WallGroupRoot {
    pub name: String,
    pub pivot: Vec2,
}

#[derive(Component)]
pub struct WallTimeline {
    pub timeline_name: String,
    pub duration: f32,
    pub repeat: RepeatModeDef,
    pub rotation: Vec<(f32, f32)>, // (t, radians)
    pub scale: Vec<(f32, f32)>,    // (t, uniform)
    pub time: f32,
    pub forward: bool, // for ping-pong
}

impl WallTimeline {
    pub fn from_def(t: &TransformTimelineDef) -> Self {
        Self {
            timeline_name: t.name.clone(),
            duration: t.duration.max(0.001),
            repeat: t.repeat.clone(),
            rotation: t.rotation.iter().map(|k| (k.t, k.value)).collect(),
            scale: t.scale.iter().map(|k| (k.t, k.value)).collect(),
            time: 0.0,
            forward: true,
        }
    }

    fn sample(keys: &[(f32, f32)], t: f32) -> f32 {
        if keys.is_empty() {
            return 0.0;
        }
        if keys.len() == 1 {
            return keys[0].1;
        }
        // Find two surrounding keys (linear search; small lists expected)
        let mut prev = keys[0];
        for &k in &keys[1..] {
            if t < k.0 {
                // between prev and k
                let span = (k.0 - prev.0).max(1e-6);
                let a = (t - prev.0) / span;
                return prev.1 + (k.1 - prev.1) * a;
            }
            prev = k;
        }
        // After last
        prev.1
    }
}

pub struct WallTimelinePlugin;

impl Plugin for WallTimelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, advance_wall_timelines);
    }
}

fn advance_wall_timelines(
    time: Res<Time>,
    mut q: Query<(&mut WallTimeline, &mut Transform), With<WallGroupRoot>>,
) {
    let dt = time.delta_secs();
    for (mut tl, mut tf) in &mut q {
        if tl.duration <= 0.0 {
            continue;
        }
        // Advance time with repeat logic
        match tl.repeat {
            RepeatModeDef::Once => {
                tl.time = (tl.time + dt).min(tl.duration);
            }
            RepeatModeDef::Loop => {
                tl.time = (tl.time + dt) % tl.duration;
            }
            RepeatModeDef::PingPong => {
                if tl.forward {
                    tl.time += dt;
                } else {
                    tl.time -= dt;
                }
                if tl.time >= tl.duration {
                    tl.time = tl.duration;
                    tl.forward = false;
                } else if tl.time <= 0.0 {
                    tl.time = 0.0;
                    tl.forward = true;
                }
            }
        }
        let norm_t = (tl.time / tl.duration).clamp(0.0, 1.0);
        let rot = WallTimeline::sample(&tl.rotation, norm_t);
        let scl = if tl.scale.is_empty() {
            1.0
        } else {
            WallTimeline::sample(&tl.scale, norm_t).max(0.001)
        };
        tf.rotation = Quat::from_rotation_z(rot);
        tf.scale = Vec3::splat(scl);
    }
}
