use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, Damping, Velocity};
use bevy::sprite::MeshMaterial2d;

use rand::Rng;

use crate::core::components::{Ball, BallCircleVisual, BallRadius};
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

/// Component representing a ball that is currently in the popping fade-out phase.
#[derive(Component, Debug)]
pub struct PoppingBall {
    pub elapsed: f32,
    pub duration: f32,
    pub start_radius: f32,
    pub end_scale: f32,
    pub fade_alpha: bool,
    pub collider_shrink: bool,
    pub collider_min_scale: f32,
    pub base_alpha: f32,
    pub added_damping: f32,
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
            // Run after impulses applied but still in Update; does not need to be before physics since
            // scaling is purely visual unless collider shrinking is enabled (then we do it early each frame).
            .add_systems(Update, update_popping_balls.after(PrePhysicsSet))
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
    mut q: Query<(&Transform, &BallRadius, &mut Velocity, Option<&mut Damping>), With<Ball>>,
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
                    && dist_centroid < bcl.centroid.distance(world_pos)
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

    // Apply outward impulse and configure fade / popping component
    let base_impulse = cfg.interactions.explosion.impulse;
    let magnitude_base =
        base_impulse * cp.impulse_scale.max(0.0) * (1.0 + cp.outward_bonus.max(0.0));
    let mut r = rand::thread_rng();

    for e in cluster.entities.iter() {
        if let Ok((tf, radius, mut vel, damping_opt)) = q.get_mut(*e) {
            let pos = tf.translation.truncate();
            let mut dir = pos - cluster.centroid;
            let len = dir.length();
            if len > 1e-4 {
                dir /= len;
            } else {
                dir = Vec2::X; // deterministic
            }
            let mag = magnitude_base * (radius.0 / 10.0).max(0.1);
            vel.linvel += dir * mag;

            // Spin jitter -> modify angular velocity field
            if cp.spin_jitter > 0.0 {
                let jitter = r.gen_range(-cp.spin_jitter..cp.spin_jitter);
                vel.angvel += jitter;
            }

            let mut added_damping = 0.0;
            if cp.fade_enabled && cp.velocity_damping > 0.0 {
                if let Some(mut d) = damping_opt {
                    d.linear_damping += cp.velocity_damping;
                    added_damping = cp.velocity_damping;
                } else {
                    // Insert a new damping component
                    commands
                        .entity(*e)
                        .insert(Damping {
                            linear_damping: cp.velocity_damping,
                            angular_damping: 0.0,
                        });
                    added_damping = cp.velocity_damping;
                }
            }

            if cp.fade_enabled {
                commands.entity(*e).insert(PoppingBall {
                    elapsed: 0.0,
                    duration: cp
                        .fade_duration
                        .max(0.05)
                        .min(if cp.fade_enabled { cp.fade_duration.max(0.05) } else { 0.05 }),
                    start_radius: radius.0,
                    end_scale: cp.fade_scale_end.clamp(0.0, 1.0),
                    fade_alpha: cp.fade_alpha,
                    collider_shrink: cp.collider_shrink,
                    collider_min_scale: cp.collider_min_scale.clamp(0.0, 1.0),
                    base_alpha: -1.0, // sentinel; will capture original alpha on first update
                    added_damping,
                });
            } else {
                // Legacy immediate despawn path (respect existing despawn_delay if set)
                if cp.despawn_delay <= 0.0 {
                    commands.entity(*e).despawn();
                } else {
                    // Minimal timer-based fallback: reuse PoppingBall with duration=despawn_delay but no scaling if fade disabled.
                    commands.entity(*e).insert(PoppingBall {
                        elapsed: 0.0,
                        duration: cp.despawn_delay,
                        start_radius: radius.0,
                        end_scale: 1.0,
                        fade_alpha: false,
                        collider_shrink: false,
                        collider_min_scale: 1.0,
                        base_alpha: -1.0,
                        added_damping: 0.0,
                    });
                }
            }
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
            "ClusterPopped color={} count={} area={:.1} centroid=({:.1},{:.1}) fade={}",
            cluster.color_index,
            ball_count,
            total_area,
            cluster.centroid.x,
            cluster.centroid.y,
            cp.fade_enabled
        );
    }
}

fn update_popping_balls(
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut PoppingBall, &BallRadius, Option<&Children>)>,
    mut q_child_vis: Query<(
        &mut Transform,
        Option<&MeshMaterial2d<ColorMaterial>>,
        Option<&BallCircleVisual>,
    )>,
    mut q_collider: Query<&mut Collider>,
) {
    if q.is_empty() {
        return;
    }
    let dt = time.delta_secs();

    for (entity, mut popping, radius, children_opt) in q.iter_mut() {
        popping.elapsed += dt;
        let t_raw = (popping.elapsed / popping.duration).clamp(0.0, 1.0);
        // Smoothstep easing
        let t = t_raw * t_raw * (3.0 - 2.0 * t_raw);
        let scale_factor = 1.0 + (popping.end_scale - 1.0) * t;

        // Adjust visuals (child holding mesh & material)
        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((mut tf, maybe_mat, _marker)) = q_child_vis.get_mut(child) {
                    // Original child scale = radius * 2 (diameter). Recompute each frame relative to base.
                    tf.scale = Vec3::splat(radius.0 * 2.0 * scale_factor.max(0.0));

                    // Alpha fade
                    if let Some(mesh_mat) = maybe_mat {
                        if popping.fade_alpha {
                            if let Some(mat) = materials.get_mut(&mesh_mat.0) {
                                // Capture base alpha first time (assumes linear RGBA)
                                if popping.base_alpha < 0.0 {
                                    let s = mat.color.to_srgba();
                                    popping.base_alpha = s.alpha;
                                }
                                let new_alpha = popping.base_alpha * (1.0 - t);
                                let s = mat.color.to_srgba();
                                mat.color = Color::srgba(s.red, s.green, s.blue, new_alpha);
                            }
                        }
                    }
                }
            }
        }

        // Collider shrink
        if popping.collider_shrink {
            if let Ok(mut col) = q_collider.get_mut(entity) {
                // Determine target collider radius (clamp to collider_min_scale)
                let phys_scale_target =
                    (popping.end_scale.max(popping.collider_min_scale)).clamp(0.0, 1.0);
                let phys_scale = 1.0 + (phys_scale_target - 1.0) * t;
                let new_r = radius.0 * phys_scale.max(0.0);
                // Replace only if significantly changed to avoid churn
                // Collider::ball stores radius internally; simplest is to overwrite each frame
                *col = Collider::ball(new_r);
            }
        }

        // Completion
        if popping.elapsed >= popping.duration {
            commands.entity(entity).despawn();
        }
    }
}

fn reset_tap_consumed(mut tap: ResMut<TapConsumed>) {
    tap.0 = false;
}
