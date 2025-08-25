use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::{
    config::{DragConfig, ExplosionConfig},
    GameConfig,
};
use crate::core::system::system_order::PrePhysicsSet;
use crate::interaction::cluster_pop::TapConsumed;

pub struct InputInteractionPlugin;

impl Plugin for InputInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveDrag::default()).add_systems(
            Update,
            (
                begin_or_end_drag,
                handle_tap_explosion.in_set(PrePhysicsSet).in_set(TapExplosionSet),
                apply_drag_force.in_set(PrePhysicsSet),
            ),
        );
    }
}

#[derive(Resource, Default, Debug)]
pub struct ActiveDrag {
    pub entity: Option<Entity>,
    pub started: bool,
    pub last_pos: Option<Vec2>,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TapExplosionSet;

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
        let pos = touch.position();
        return cursor_world_pos(window, camera_q, pos);
    }
    let cursor = window.cursor_position()?;
    cursor_world_pos(window, camera_q, cursor)
}

fn handle_tap_explosion(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q: Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>,
    mut active_drag: ResMut<ActiveDrag>,
    tap_consumed: Res<TapConsumed>,
    cfg: Res<GameConfig>,
) {
    let ExplosionConfig {
        enabled,
        impulse,
        radius,
        falloff_exp,
    } = cfg.interactions.explosion;
    if tap_consumed.0 {
        return;
    }
    if !enabled {
        return;
    }
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    if active_drag.started {
        active_drag.started = false;
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else {
        return;
    };
    let r2 = radius * radius;
    for (tf, ball_radius, mut vel) in q.iter_mut() {
        let pos = tf.translation.truncate();
        let d2 = pos.distance_squared(world_pos);
        if d2 > r2 {
            continue;
        }
        let d = d2.sqrt();
        let dir = if d < 1e-4 {
            Vec2::X
        } else {
            (pos - world_pos) / d
        };
        let norm = (1.0 - (d / radius))
            .clamp(0.0, 1.0)
            .powf(falloff_exp.max(0.1));
        let impulse_vec = dir * impulse * norm * (ball_radius.0 / 10.0).max(0.1);
        vel.linvel += impulse_vec;
    }
}

fn begin_or_end_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut active: ResMut<ActiveDrag>,
    q: Query<(Entity, &Transform, &BallRadius), With<Ball>>,
    cfg: Res<GameConfig>,
) {
    let drag_cfg: &DragConfig = &cfg.interactions.drag;
    if !drag_cfg.enabled {
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else {
        return;
    };
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if released {
        active.entity = None;
        active.last_pos = None;
    }
    if active.entity.is_none()
        && (buttons.just_pressed(MouseButton::Left) || touches.iter_just_pressed().next().is_some())
    {
        let mut nearest: Option<(Entity, f32)> = None;
        for (e, tf, radius) in q.iter() {
            let pos = tf.translation.truncate();
            let d2 = pos.distance_squared(world_pos);
            let grab_r = drag_cfg.grab_radius.max(radius.0);
            if d2 <= grab_r * grab_r {
                if let Some((_, best)) = &nearest {
                    if d2 < *best {
                        nearest = Some((e, d2));
                    }
                } else {
                    nearest = Some((e, d2));
                }
            }
        }
        if let Some((e, _)) = nearest {
            active.entity = Some(e);
            active.started = false;
            active.last_pos = Some(world_pos);
        }
    }
    if let (Some(_e), Some(last)) = (active.entity, active.last_pos) {
        if world_pos.distance_squared(last) > 4.0 {
            active.started = true;
        }
        active.last_pos = Some(world_pos);
    }
}

fn apply_drag_force(
    time: Res<Time>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut active: ResMut<ActiveDrag>,
    mut q: Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>,
    cfg: Res<GameConfig>,
) {
    let drag_cfg = &cfg.interactions.drag;
    if !drag_cfg.enabled {
        return;
    }
    let Some(active_entity) = active.entity else {
        return;
    };
    let Ok(window) = windows_q.single() else {
        return;
    };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else {
        return;
    };
    let dt = time.delta_secs();
    if let Ok((tf, _radius, mut vel)) = q.get_mut(active_entity) {
        let pos = tf.translation.truncate();
        let to_pointer = world_pos - pos;
        let dist = to_pointer.length();
        if dist < 1e-3 {
            return;
        }
        let dir = to_pointer / dist;
        vel.linvel += dir * drag_cfg.pull_strength * dt;
        if drag_cfg.max_speed > 0.0 {
            let speed = vel.linvel.length();
            if speed > drag_cfg.max_speed {
                vel.linvel *= drag_cfg.max_speed / speed;
            }
        }
        vel.linvel *= 0.98;
    } else {
        active.entity = None;
    }
}
