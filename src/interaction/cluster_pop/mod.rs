use bevy::prelude::*;
use bevy_rapier2d::prelude::Velocity;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::GameConfig;
use crate::core::system::system_order::PrePhysicsSet;
use crate::interaction::input::input_interaction::{ActiveDrag, TapExplosionSet};
use crate::physics::clustering::cluster::Clusters;

/// Event emitted when a qualifying cluster is popped (cleared)
#[derive(Event, Debug, Clone)]
pub struct ClusterPopped {
    pub color_index: usize,
    pub ball_count: usize,
    pub total_area: f32,
    pub centroid: Vec2,
}

/// Transient flag (per-frame) to suppress the generic tap explosion when a cluster pop occurred.
#[derive(Resource, Default, Debug)]
pub struct TapConsumed(pub bool);

/// Component for delayed despawn after a pop (optional fade hook).
#[derive(Component, Debug)]
pub struct PopFade {
    pub remaining: f32,
}

pub struct ClusterPopPlugin;

impl Plugin for ClusterPopPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TapConsumed>()
            .add_event::<ClusterPopped>()
            .add_systems(
                Update,
                handle_tap_cluster_pop
                    .in_set(PrePhysicsSet)
                    .before(TapExplosionSet),
            )
            .add_systems(Update, update_pop_fade)
            .add_systems(PostUpdate, reset_tap_consumed);
    }
}

fn cursor_world_pos(
    _window: &Window,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
    screen_pos: Vec2,
) -> Option<Vec2> {
    let (camera, cam_tf) = camera_q.iter().next()?;
    camera.viewport_to_world_2d(cam_tf, screen_pos).ok()
}

fn primary_pointer_world_pos(
    window: &Window,
    touches: &Touches,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    if let Some(touch) = touches.iter().next() {
        return cursor_world_pos(window, camera_q, touch.position());
    }
    let cursor = window.cursor_position()?;
    cursor_world_pos(window, camera_q, cursor)
}

fn handle_tap_cluster_pop(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    clusters: Res<Clusters>,
    mut q: Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>,
    mut tap_consumed: ResMut<TapConsumed>,
    active_drag: Res<ActiveDrag>,
    mut ew: EventWriter<ClusterPopped>,
    mut commands: Commands,
    cfg: Res<GameConfig>,
) {
    let cp = &cfg.interactions.cluster_pop;
    if !cp.enabled {
        return;
    }
    // Pointer/tap release detection
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    // Do not trigger on drag release
    if active_drag.started {
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else {
        return;
    };

    // Candidate cluster selection
    let mut best: Option<usize> = None;
    for (i, cl) in clusters.0.iter().enumerate() {
        // AABB padded hit
        let min = cl.min - Vec2::splat(cp.aabb_pad.max(0.0));
        let max = cl.max + Vec2::splat(cp.aabb_pad.max(0.0));
        let inside_aabb = world_pos.x >= min.x
            && world_pos.x <= max.x
            && world_pos.y >= min.y
            && world_pos.y <= max.y;
        let dist_centroid = cl.centroid.distance(world_pos);
        let within_radius = dist_centroid <= cp.tap_radius.max(0.0);
        if inside_aabb || within_radius {
            if let Some(bi) = best {
                let bcl = &clusters.0[bi];
                // Prefer largest ball count; tie-break by smaller centroid distance
                let better = if cl.entities.len() > bcl.entities.len() {
                    true
                } else if cl.entities.len() == bcl.entities.len()
                    && dist_centroid
                        < bcl.centroid.distance(world_pos)
                {
                    true
                } else {
                    false
                };
                if better {
                    best = Some(i);
                }
            } else {
                best = Some(i);
            }
        }
    }
    let Some(idx) = best else {
        // No candidate cluster; allow normal explosion
        return;
    };
    let cluster = &clusters.0[idx];

    let ball_count = cluster.entities.len();
    let total_area = cluster.total_area;
    // Threshold evaluation
    if ball_count < cp.min_ball_count {
        return;
    }
    if cp.min_total_area > 0.0 && total_area < cp.min_total_area {
        return;
    }

    // Apply outward impulse + schedule despawn
    let base_impulse = cfg.interactions.explosion.impulse;
    let magnitude_base = base_impulse * cp.impulse_scale.max(0.0);
    if magnitude_base > 0.0 || cp.outward_bonus > 0.0 {
        for e in cluster.entities.iter() {
            if let Ok((tf, radius, mut vel)) = q.get_mut(*e) {
                let pos = tf.translation.truncate();
                let mut dir = pos - cluster.centroid;
                let len = dir.length();
                if len > 1e-4 {
                    dir /= len;
                } else {
                    dir = Vec2::X; // deterministic
                }
                // Interpret outward_bonus as additive % (bonus 0.6 => 1.6x)
                let mag = magnitude_base
                    * (1.0 + cp.outward_bonus.max(0.0))
                    * (radius.0 / 10.0).max(0.1);
                vel.linvel += dir * mag;
            }
        }
    }

    let despawn_delay = cp.despawn_delay.max(0.0);
    for e in cluster.entities.iter() {
        if despawn_delay == 0.0 {
            commands.entity(*e).despawn();
        } else {
            commands.entity(*e).insert(PopFade {
                remaining: despawn_delay,
            });
        }
    }

    ew.write(ClusterPopped {
        color_index: cluster.color_index,
        ball_count,
        total_area,
        centroid: cluster.centroid,
    });

    tap_consumed.0 = true;

    #[cfg(feature = "debug")]
    {
        info!(
            "ClusterPopped color={} count={} area={:.1} centroid=({:.1},{:.1})",
            cluster.color_index,
            ball_count,
            total_area,
            cluster.centroid.x,
            cluster.centroid.y
        );
    }
}

fn update_pop_fade(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut PopFade)>,
) {
    if q.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    for (e, mut fade) in q.iter_mut() {
        fade.remaining -= dt;
        if fade.remaining <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn reset_tap_consumed(mut tap: ResMut<TapConsumed>) {
    tap.0 = false;
}
