//! Systems for input action evaluation (placeholders).
use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use super::types::*;

pub fn collect_raw_inputs() {}
pub fn resolve_gestures() {}
pub fn evaluate_bindings() {}
pub fn compute_virtual_axes() {}
pub fn finalize_states() {}

pub fn system_collect_inputs(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_evr: EventReader<MouseMotion>,
    mut wheel_evr: EventReader<MouseWheel>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    mut input_map: ResMut<InputMap>,
) {
    input_map.frame_counter += 1;
    // Clear per-frame stuff
    for st in &mut input_map.dynamic_states { match st { ActionDynamicState::Binary(b)|ActionDynamicState::Gesture(b)=> b.clear_transitions(), ActionDynamicState::Axis1(a)=> a.clear_transitions(), ActionDynamicState::Axis2(a)=> a.clear_transitions() } }

    let mut mouse_delta = Vec2::ZERO;
    for ev in motion_evr.read() { mouse_delta += ev.delta; }

    let mut wheel_v = 0.0f32; let mut wheel_h = 0.0f32;
    for ev in wheel_evr.read() { match ev.unit { bevy::input::mouse::MouseScrollUnit::Line => { wheel_v += ev.y; wheel_h += ev.x; }, bevy::input::mouse::MouseScrollUnit::Pixel => { wheel_v += ev.y * 0.1; wheel_h += ev.x * 0.1; } } }

    // Gesture pointer (single primary touch or mouse)
    let mut pointer_pos: Option<Vec2> = None;
    if touches.iter().count() > 0 { if let Some(t) = touches.iter().next() { pointer_pos = Some(Vec2::new(t.position().x, t.position().y)); } }
    else if let Ok(primary) = windows.single() { if let Some(pos) = primary.cursor_position() { // window coords -> center-origin
            let centered = Vec2::new(pos.x - primary.width() * 0.5, pos.y - primary.height()*0.5);
            pointer_pos = Some(centered);
        } }

    // Update gesture runtime
    let dt = time.delta_secs();
    let gcfg = input_map.gesture_cfg.clone();
    let mut mark_tap = false;
    {
        let grt = &mut input_map.gesture_rt;
        if let Some(ppos) = pointer_pos { if !grt.pointer_down { // detect down
            if mouse_buttons.pressed(MouseButton::Left) || touches.iter().count()>0 { grt.pointer_down = true; grt.pointer_start = ppos; grt.pointer_last = ppos; grt.time_down = 0.0; grt.max_moved = 0.0; grt.dragging = false; grt.drag_axis_value = Vec2::ZERO; grt.drag_axis_raw_delta = Vec2::ZERO; }
        } else { grt.time_down += dt; let move_delta = ppos - grt.pointer_last; grt.pointer_last = ppos; grt.max_moved = grt.max_moved.max((ppos - grt.pointer_start).length()); grt.drag_axis_raw_delta = move_delta; if !grt.dragging && (ppos - grt.pointer_start).length() >= gcfg.drag_min_move { grt.dragging = true; }
            if grt.dragging { let alpha = (1.0 - gcfg.drag_smoothing).clamp(0.0,1.0); grt.drag_axis_value = grt.drag_axis_value.lerp(grt.drag_axis_value + move_delta, alpha); }
        } }
    else { // no pointer
        grt.pointer_down = false; grt.dragging = false; }

    if grt.pointer_down && !(mouse_buttons.pressed(MouseButton::Left) || touches.iter().count()>0) { // released
        // classify tap
        if grt.time_down <= gcfg.tap_max_time && grt.max_moved <= gcfg.tap_max_move { // mark tap gesture action if present
            mark_tap = true;
        }
        grt.pointer_down = false; grt.dragging = false; }
    }

    if mark_tap { if let Some(id) = input_map.name_to_id.get("tap").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(b) = st.as_binary_mut() { b.pressed = true; b.just_pressed = true; b.just_released = true; } } } }

    // Mouse move axis2 action if present
    if let Some(id) = input_map.name_to_id.get("look").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(a2) = st.as_axis2_mut() { a2.value += mouse_delta; if mouse_delta.length_squared()>0.0 { a2.active = true; a2.delta = mouse_delta; a2.just_pressed = true; } } } }

    // Wheel vertical / horizontal actions
    if wheel_v != 0.0 { if let Some(id) = input_map.name_to_id.get("wheel_v").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(a1) = st.as_axis1_mut() { a1.value += wheel_v; a1.delta += wheel_v; a1.active = true; } } } }
    if wheel_h != 0.0 { if let Some(id) = input_map.name_to_id.get("wheel_h").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(a1) = st.as_axis1_mut() { a1.value += wheel_h; a1.delta += wheel_h; a1.active = true; } } } }

    // Placeholder for keyboard->actions mapping (to be replaced by binding evaluation)
    for (name, id) in input_map.name_to_id.clone() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let ActionDynamicState::Binary(b) = st { // Very limited sample: Space
                if name == "space" { let pressed = keyboard.pressed(KeyCode::Space); if pressed && !b.pressed { b.just_pressed = true; } if !pressed && b.pressed { b.just_released = true; } b.pressed = pressed; } } } }
}
