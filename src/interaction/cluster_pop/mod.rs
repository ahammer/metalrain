use bevy::prelude::*;
use bevy::sprite::MeshMaterial2d;
use bevy_rapier2d::prelude::{Collider, Velocity};

use crate::core::components::{Ball, BallCircleVisual, BallRadius};
use crate::core::config::GameConfig;
use crate::core::system::system_order::PrePhysicsSet;
use crate::physics::clustering::cluster::{Clusters, BallClusterIndex};

/// Event emitted when a qualifying cluster transitions into the paddle lifecycle
#[derive(Event, Debug, Clone)]
pub struct ClusterPopped {
    pub color_index: usize,
    pub ball_count: usize,
    pub total_area: f32,
    pub centroid: Vec2,
}

/// Lifecycle freeze behavior
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreezeMode {
    ZeroVelEachFrame,
    Kinematic,
    Fixed,
}
impl FreezeMode {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => FreezeMode::Kinematic,
            2 => FreezeMode::Fixed,
            _ => FreezeMode::ZeroVelEachFrame,
        }
    }
}

#[derive(Component, Debug)]
pub struct PaddleLifecycle {
    pub elapsed: f32,
    pub grow_duration: f32,
    pub hold_duration: f32,
    pub shrink_duration: f32,
    pub peak_scale: f32,
    pub freeze_mode: FreezeMode,
    pub base_radius: f32,
    pub fade_alpha: bool,
    pub fade_curve: u32,
    pub collider_scale_curve: u32,
    pub alpha_base: f32, // sentinel < 0 until captured
}
impl PaddleLifecycle {
    #[inline]
    pub fn total(&self) -> f32 {
        self.grow_duration + self.hold_duration + self.shrink_duration
    }
}

type ChildVisualTuple<'a> = (
    &'a mut Transform,
    Option<&'a MeshMaterial2d<ColorMaterial>>,
    Option<&'a BallCircleVisual>,
);

pub struct ClusterPopPlugin;

impl Plugin for ClusterPopPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ClusterPopped>()
            .add_systems(
                Update,
                handle_tap_cluster_pop
                    .in_set(PrePhysicsSet),
            )
            // Run after tap selection, still inside PrePhysicsSet so collider size & velocity freeze
            // are applied before the physics step.
            .add_systems(
                Update,
                update_paddle_lifecycle
                    .after(handle_tap_cluster_pop)
                    .in_set(PrePhysicsSet),
            );
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

const DIST_EPS: f32 = 1e-4; // distance squared epsilon for tie-breaking

/// Ball-first cluster picking. Returns (ball_entity, cluster_index, ball_radius, distance_squared_to_ball)
pub fn pick_ball_cluster<'a, I>(
    world_pos: Vec2,
    clusters: &Clusters,
    cluster_index: &BallClusterIndex,
    iter: I,
    cp: &crate::core::config::ClusterPopConfig,
) -> Option<(Entity, usize, f32, f32)>
where I: IntoIterator<Item = (Entity, &'a Transform, &'a BallRadius, Option<&'a PaddleLifecycle>)>
{
    let mut best_ball: Option<(Entity, usize, f32, f32, f32)> = None; // (entity, cluster_idx, d2, radius, cluster_centroid_d2)
    for (entity, tf, radius, lifecycle) in iter.into_iter() {
        if lifecycle.is_some() { continue; }
        let cluster_idx = match cluster_index.0.get(&entity) { Some(i) => *i, None => continue };
        let pos = tf.translation.truncate();
        let delta = world_pos - pos;
        if !delta.x.is_finite() || !delta.y.is_finite() { continue; }
        let d2 = delta.length_squared();
        if d2.is_nan() { continue; }
        let base_pick = cp.ball_pick_radius.max(0.0);
        let eff_r = if cp.ball_pick_radius_scale_with_ball { base_pick.max(radius.0) } else { base_pick };
        if d2 > eff_r * eff_r { continue; }
        let cluster = match clusters.0.get(cluster_idx) { Some(c) => c, None => continue };
        let centroid_d2 = cluster.centroid.distance_squared(world_pos);
        let radius_val = radius.0;
        let replace = match best_ball {
            None => true,
            Some((best_entity, bci, bd2, br, bcent_d2)) => {
                if d2 + DIST_EPS < bd2 { true }
                else if (d2 - bd2).abs() <= DIST_EPS {
                    if cp.prefer_larger_radius_on_tie && (radius_val > br + 1e-6) { true }
                    else if (radius_val - br).abs() <= 1e-6 {
                        let bcl = &clusters.0[bci];
                        if cluster.entities.len() > bcl.entities.len() { true }
                        else if cluster.entities.len() == bcl.entities.len() {
                            if centroid_d2 + DIST_EPS < bcent_d2 { true }
                            else if (centroid_d2 - bcent_d2).abs() <= DIST_EPS { entity.index() < best_entity.index() } else { false }
                        } else { false }
                    } else { false }
                } else { false }
            }
        };
        if replace { best_ball = Some((entity, cluster_idx, d2, radius_val, centroid_d2)); }
    }
    best_ball.map(|(e, ci, d2, r, _)| (e, ci, r, d2))
}

fn handle_tap_cluster_pop(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    clusters: Res<Clusters>,
    cluster_index: Res<BallClusterIndex>,
    mut q: Query<(Entity, &Transform, &BallRadius, &mut Velocity, Option<&PaddleLifecycle>), With<Ball>>,
    mut ew: EventWriter<ClusterPopped>,
    mut commands: Commands,
    cfg: Res<GameConfig>,
) {
    let cp = &cfg.interactions.cluster_pop;
    if !cp.enabled {
        return;
    }
    // Tap release detection
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else {
        return;
    };

    let iter = q.iter().map(|(e,t,r,_v,l)| (e,t,r,l));
    let Some((_ball_entity, cluster_idx, chosen_radius, _d2)) = pick_ball_cluster(world_pos, &clusters, &cluster_index, iter, cp) else {
        #[cfg(feature = "debug")]
        {
            info!("cluster_pop: no ball hit");
        }
        return;
    };
    let cluster = &clusters.0[cluster_idx];

    let ball_count = cluster.entities.len();
    let total_area = cluster.total_area;
    if ball_count < cp.min_ball_count {
        return;
    }
    if cp.min_total_area > 0.0 && total_area < cp.min_total_area {
        return;
    }

    for e in cluster.entities.iter() {
        if let Ok((entity, _tf, radius, mut vel, existing)) = q.get_mut(*e) {
            if existing.is_some() {
                continue;
            }
            // Freeze initial motion deterministically
            vel.linvel = Vec2::ZERO;
            vel.angvel = 0.0;
            commands.entity(entity).insert(PaddleLifecycle {
                elapsed: 0.0,
                grow_duration: cp.grow_duration.max(0.01),
                hold_duration: cp.hold_duration.max(0.0),
                shrink_duration: cp.shrink_duration.max(0.05),
                peak_scale: cp.peak_scale.max(0.1),
                freeze_mode: FreezeMode::from_code(cp.freeze_mode),
                base_radius: radius.0,
                fade_alpha: cp.fade_alpha,
                fade_curve: cp.fade_curve,
                collider_scale_curve: cp.collider_scale_curve,
                alpha_base: -1.0,
            });
        }
    }

    ew.write(ClusterPopped {
        color_index: cluster.color_index,
        ball_count,
        total_area,
        centroid: cluster.centroid,
    });

    #[cfg(feature = "debug")]
    {
        info!(
            "ClusterPopped color={} count={} area={:.1} centroid=({:.1},{:.1}) peak_scale={:.2} chosen_ball_radius={:.2}",
            cluster.color_index,
            ball_count,
            total_area,
            cluster.centroid.x,
            cluster.centroid.y,
            cp.peak_scale,
            chosen_radius
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifecyclePhase {
    Grow,
    Hold,
    Shrink,
}

fn apply_curve(mode: u32, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    match mode {
        0 => x,                                 // linear
        1 => x * x * (3.0 - 2.0 * x),           // smoothstep
        2 => 1.0 - (1.0 - x).powi(3),           // ease-out cubic
        _ => x,
    }
}

fn update_paddle_lifecycle(
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &mut PaddleLifecycle,
        &mut BallRadius,
        Option<&Children>,
        &mut Velocity,
        &mut Collider,
    )>,
    mut q_child_vis: Query<ChildVisualTuple>,
) {
    if q.is_empty() {
        return;
    }
    let dt = time.delta_secs();

    for (entity, mut plc, mut radius_comp, children_opt, mut vel, mut collider) in q.iter_mut() {
        let prev_elapsed = plc.elapsed;
        plc.elapsed += dt;
        let total = plc.total();

        // Determine phase & local_t
        let (phase, local_t) = if plc.elapsed < plc.grow_duration {
            (LifecyclePhase::Grow, plc.elapsed / plc.grow_duration.max(f32::EPSILON))
        } else if plc.elapsed < plc.grow_duration + plc.hold_duration {
            (LifecyclePhase::Hold, 0.0)
        } else {
            let base = plc.grow_duration + plc.hold_duration;
            let shrink_elapsed = (plc.elapsed - base).max(0.0);
            (
                LifecyclePhase::Shrink,
                shrink_elapsed / plc.shrink_duration.max(f32::EPSILON),
            )
        };

        let scale_t = apply_curve(plc.collider_scale_curve, local_t);
        let factor = match phase {
            LifecyclePhase::Grow => 1.0 + (plc.peak_scale - 1.0) * scale_t,
            LifecyclePhase::Hold => plc.peak_scale,
            LifecyclePhase::Shrink => plc.peak_scale * (1.0 - scale_t),
        }
        .max(0.0);

        // Velocity freezing (simple deterministic approach)
        vel.linvel = Vec2::ZERO;
        vel.angvel = 0.0;

        // Update collider & logical BallRadius (metaball rendering reads BallRadius)
        let new_r = plc.base_radius * factor;
        if new_r.is_finite() {
            radius_comp.0 = new_r;
            *collider = Collider::ball(new_r);
        }

        // Update visuals (child transform scale, alpha fade)
        if let Some(children) = children_opt {
            for child in children.iter() {
                if let Ok((mut tf, maybe_mat, _marker)) = q_child_vis.get_mut(child) {
                    tf.scale = Vec3::splat(plc.base_radius * 2.0 * factor);
                    if plc.fade_alpha {
                        if let Some(mesh_mat) = maybe_mat {
                            if let Some(mat) = materials.get_mut(&mesh_mat.0) {
                                if plc.alpha_base < 0.0 {
                                    plc.alpha_base = mat.color.to_srgba().alpha;
                                }
                                if matches!(phase, LifecyclePhase::Shrink) {
                                    let fade_t = apply_curve(plc.fade_curve, local_t);
                                    let new_alpha = plc.alpha_base * (1.0 - fade_t);
                                    let c = mat.color.to_srgba();
                                    mat.color = Color::srgba(
                                        c.red,
                                        c.green,
                                        c.blue,
                                        new_alpha.clamp(0.0, plc.alpha_base),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Completion (ensure at least one frame at final shrink factor)
        if plc.elapsed >= total {
            commands.entity(entity).despawn();
        } else if prev_elapsed < total && plc.elapsed > total {
            // Edge case: large dt overshoot; still despawn now.
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rapier2d::prelude::{Collider, Velocity};

    #[test]
    fn scale_progress_basic() {
        let plc = PaddleLifecycle {
            elapsed: 0.0,
            grow_duration: 0.25,
            hold_duration: 0.1,
            shrink_duration: 0.4,
            peak_scale: 1.8,
            freeze_mode: FreezeMode::ZeroVelEachFrame,
            base_radius: 10.0,
            fade_alpha: true,
            fade_curve: 1,
            collider_scale_curve: 1,
            alpha_base: -1.0,
        };
        assert!((plc.total() - 0.75).abs() < 1e-5);
        // Mid grow expected > 1.0
        let half_grow_factor = {
            let local_t = 0.5;
            let curve = apply_curve(plc.collider_scale_curve, local_t);
            1.0 + (plc.peak_scale - 1.0) * curve
        };
        assert!(half_grow_factor > 1.3);
    }

    #[test]
    fn ball_radius_updates_during_lifecycle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Provide required resources for system params
        app.init_resource::<Assets<ColorMaterial>>();
        // Insert system under test
        app.add_systems(Update, update_paddle_lifecycle);

        // Spawn a ball mid-grow (elapsed = 50% of grow)
        let base_radius = 10.0;
        let grow_duration = 0.2;
        app.world_mut().spawn((
            Ball,
            PaddleLifecycle {
                elapsed: grow_duration * 0.5, // 50% through grow phase
                grow_duration,
                hold_duration: 0.0,
                shrink_duration: 0.2,
                peak_scale: 1.8,
                freeze_mode: FreezeMode::ZeroVelEachFrame,
                base_radius,
                fade_alpha: true,
                fade_curve: 1,
                collider_scale_curve: 1,
                alpha_base: -1.0,
            },
            BallRadius(base_radius),
            Velocity::zero(),
            Collider::ball(base_radius),
        ));

        app.update(); // run one frame (dt defaults to 0, system applies factor based on existing elapsed)

        // Verify BallRadius increased (metaball renderer will now see enlarged radius)
        let mut query = app.world_mut().query::<(&PaddleLifecycle, &BallRadius)>();
        for (plc, r) in query.iter(app.world()) {
            // Expected factor at 50% grow with smoothstep(0.5)=0.5: 1 + (1.8-1)*0.5 = 1.4
            assert!(r.0 > base_radius * 1.1 && r.0 <= plc.base_radius * plc.peak_scale + 0.01);
        }
    }
}
