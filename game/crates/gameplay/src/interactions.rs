// Phase 5: Input interactions (drag & explosion) ported from legacy/input_interaction.rs
// Adaptations:
// - Uses bm_core::GameConfigRes wrapper (config lives in pure-data crate).
// - System ordering: explosion + drag force placed in bm_core::PrePhysicsSet.
// - Graceful early returns if no window/camera (other crates own window/camera creation).
// - Resource ActiveDrag kept internal to gameplay crate (pub(crate)) until external API need arises.

use bevy::prelude::*;
use bevy_rapier2d::prelude::Velocity;
use bm_core::{Ball, BallRadius, GameConfigRes, PrePhysicsSet};

/// Tracks the currently dragged ball (if any) and whether movement threshold was exceeded.
#[derive(Resource, Default, Debug)]
pub(crate) struct ActiveDrag {
    pub(crate) entity: Option<Entity>,
    pub(crate) started: bool,
    pub(crate) last_pos: Option<Vec2>,
}

/// Convert a window cursor position (top-left origin, logical coordinates) to world coordinates.
fn cursor_world_pos(
    camera_q: &Query<(&Camera, &GlobalTransform)>,
    screen_pos: Vec2,
) -> Option<Vec2> {
    let (camera, cam_tf) = camera_q.iter().next()?; // assume single active camera
    camera.viewport_to_world_2d(cam_tf, screen_pos).ok()
}

/// Unified pointer (first touch if present, else mouse) world position.
fn primary_pointer_world_pos(
    window: &Window,
    touches: &Touches,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    if let Some(touch) = touches.iter().next() {
        return cursor_world_pos(camera_q, touch.position());
    }
    let cursor = window.cursor_position()?;
    cursor_world_pos(camera_q, cursor)
}

/// System: Begin a drag on press (nearest ball within grab radius) and mark started when moved.
pub(crate) fn begin_or_end_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut active: ResMut<ActiveDrag>,
    q: Query<(Entity, &Transform, &BallRadius), With<Ball>>,
    cfg: Res<GameConfigRes>,
) {
    let drag_cfg = &cfg.0.interactions.drag;
    if !drag_cfg.enabled {
        return;
    }
    let Some(window) = windows_q.iter().next() else { return; };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else { return; };

    // Release -> end drag (keep started flag for explosion suppression inspection this frame).
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if released {
        active.entity = None;
        active.last_pos = None;
    }

    // Begin drag on just pressed if none active.
    if active.entity.is_none()
        && (buttons.just_pressed(MouseButton::Left) || touches.iter_just_pressed().next().is_some())
    {
        let mut nearest: Option<(Entity, f32)> = None;
        for (e, tf, radius) in q.iter() {
            let pos = tf.translation.truncate();
            let d2 = pos.distance_squared(world_pos);
            let grab_r = drag_cfg.grab_radius.max(radius.0);
            if d2 <= grab_r * grab_r {
                match nearest {
                    Some((_, best_d2)) if d2 >= best_d2 => {}
                    _ => nearest = Some((e, d2)),
                }
            }
        }
        if let Some((e, _)) = nearest {
            active.entity = Some(e);
            active.started = false;
            active.last_pos = Some(world_pos);
        }
    }

    // Detect movement threshold to mark drag as "started".
    if let (Some(_), Some(last)) = (active.entity, active.last_pos) {
        if world_pos.distance_squared(last) > 4.0 {
            // ~2 units movement
            active.started = true;
        }
        active.last_pos = Some(world_pos);
    }
}

/// System: Explosion impulse occurs on release if drag did not meaningfully start.
pub(crate) fn handle_tap_explosion(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q: Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>,
    mut active_drag: ResMut<ActiveDrag>,
    cfg: Res<GameConfigRes>,
) {
    let ex_cfg = &cfg.0.interactions.explosion;
    if !ex_cfg.enabled {
        return;
    }
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    if active_drag.started {
        // Suppress explosion if a drag occurred; reset for next gesture.
        active_drag.started = false;
        return;
    }
    let Some(window) = windows_q.iter().next() else { return; };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else { return; };

    let r2 = ex_cfg.radius * ex_cfg.radius;
    for (tf, ball_radius, mut vel) in q.iter_mut() {
        let pos = tf.translation.truncate();
        let d2 = pos.distance_squared(world_pos);
        if d2 > r2 {
            continue;
        }
        let d = d2.sqrt();
        let dir = if d < 1e-4 { Vec2::X } else { (pos - world_pos) / d }; // outward
        let norm = (1.0 - (d / ex_cfg.radius))
            .clamp(0.0, 1.0)
            .powf(ex_cfg.falloff_exp.max(0.1));
        let impulse_vec =
            dir * ex_cfg.impulse * norm * (ball_radius.0 / 10.0).max(0.1);
        vel.linvel += impulse_vec;
    }
}

/// System: Continuous pull toward pointer while dragging.
pub(crate) fn apply_drag_force(
    time: Res<Time>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut active: ResMut<ActiveDrag>,
    mut q: Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>,
    cfg: Res<GameConfigRes>,
) {
    let drag_cfg = &cfg.0.interactions.drag;
    if !drag_cfg.enabled {
        return;
    }
    let Some(active_entity) = active.entity else { return; };
    let Some(window) = windows_q.iter().next() else { return; };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else { return; };
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
        // Mild damping for stability (legacy retained).
        vel.linvel *= 0.98;
    } else {
        // Entity despawned mid-drag.
        active.entity = None;
    }
}

/// Helper to register interaction systems (invoked by GameplayPlugin).
pub(crate) fn add_interaction_systems(app: &mut App) {
    app.init_resource::<ActiveDrag>();
    app.add_systems(
        Update,
        (
            begin_or_end_drag,
            handle_tap_explosion.in_set(PrePhysicsSet),
            apply_drag_force.in_set(PrePhysicsSet),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bm_core::{CorePlugin, GameConfigRes};

    #[test]
    fn active_drag_resource_added() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(CorePlugin);
        app.insert_resource(GameConfigRes(Default::default()));
        add_interaction_systems(&mut app);
        assert!(app.world().get_resource::<ActiveDrag>().is_some(), "ActiveDrag not inserted");
    }
}
