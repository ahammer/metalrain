//! Systems for input action evaluation.
use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use super::types::*;

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
    for st in &mut input_map.dynamic_states { match st { ActionDynamicState::Binary(b)|ActionDynamicState::Gesture(b)=> b.clear_transitions(), ActionDynamicState::Axis1(a)=> a.clear_transitions(), ActionDynamicState::Axis2(a)=> a.clear_transitions() } }

    let mut mouse_delta = Vec2::ZERO; for ev in motion_evr.read() { mouse_delta += ev.delta; }
    let mut _wheel_v = 0.0f32; let mut _wheel_h = 0.0f32; for ev in wheel_evr.read() { match ev.unit { bevy::input::mouse::MouseScrollUnit::Line => { _wheel_v += ev.y; _wheel_h += ev.x; }, bevy::input::mouse::MouseScrollUnit::Pixel => { _wheel_v += ev.y * 0.1; _wheel_h += ev.x * 0.1; } } }

    let mut pointer_pos: Option<Vec2> = None;
    if touches.iter().count() > 0 { if let Some(t) = touches.iter().next() { pointer_pos = Some(Vec2::new(t.position().x, t.position().y)); } }
    else if let Ok(primary) = windows.single() { if let Some(pos) = primary.cursor_position() { let centered = Vec2::new(pos.x - primary.width() * 0.5, pos.y - primary.height()*0.5); pointer_pos = Some(centered); } }

    let dt = time.delta_secs();
    let gcfg = input_map.gesture_cfg.clone();
    let mut mark_tap = false;
    {
        let grt = &mut input_map.gesture_rt;
        if let Some(ppos) = pointer_pos { if !grt.pointer_down { if mouse_buttons.pressed(MouseButton::Left) || touches.iter().count()>0 { grt.pointer_down = true; grt.pointer_start = ppos; grt.pointer_last = ppos; grt.time_down = 0.0; grt.max_moved = 0.0; grt.dragging = false; grt.drag_axis_value = Vec2::ZERO; grt.drag_axis_raw_delta = Vec2::ZERO; } } else { grt.time_down += dt; let move_delta = ppos - grt.pointer_last; grt.pointer_last = ppos; grt.max_moved = grt.max_moved.max((ppos - grt.pointer_start).length()); grt.drag_axis_raw_delta = move_delta; if !grt.dragging && (ppos - grt.pointer_start).length() >= gcfg.drag_min_move { grt.dragging = true; } if grt.dragging { let alpha = (1.0 - gcfg.drag_smoothing).clamp(0.0,1.0); grt.drag_axis_value = grt.drag_axis_value.lerp(grt.drag_axis_value + move_delta, alpha); } } }
        else { grt.pointer_down = false; grt.dragging = false; }
        if grt.pointer_down && !(mouse_buttons.pressed(MouseButton::Left) || touches.iter().count()>0) { if grt.time_down <= gcfg.tap_max_time && grt.max_moved <= gcfg.tap_max_move { mark_tap = true; } grt.pointer_down = false; grt.dragging = false; }
    }

    if mark_tap { if let Some(id) = input_map.name_to_id.get("PrimaryTap").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(b) = st.as_binary_mut() { b.pressed = true; b.just_pressed = true; b.just_released = true; } } } }

    // Update drag binary + axis2
    let drag_delta = input_map.gesture_rt.drag_axis_raw_delta; // snapshot to avoid borrow conflict
    if input_map.gesture_rt.dragging { if let Some(id) = input_map.name_to_id.get("Drag").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(b) = st.as_binary_mut() { if !b.pressed { b.pressed = true; b.just_pressed = true; } } } } if let Some(id) = input_map.name_to_id.get("DragAxis").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(a2) = st.as_axis2_mut() { let d = drag_delta; a2.value += d; a2.delta = d; if d.length_squared()>0.0 { a2.active = true; if !a2.just_pressed { a2.just_pressed = true; } } } } } }
    else { if let Some(id) = input_map.name_to_id.get("Drag").copied() { if let Some(st) = input_map.dynamic_states.get_mut(id.0 as usize) { if let Some(b) = st.as_binary_mut() { if b.pressed { b.pressed = false; b.just_released = true; } } } } }

    // Compute virtual axes
    if !input_map.virtual_axes.is_empty() { let snapshot = input_map.virtual_axes.clone(); let mut vals = vec![0.0; snapshot.len()]; for (i, va) in snapshot.iter().enumerate() { let mut v = 0.0; match va.pos { RawBindingToken::Key(k) => if keyboard.pressed(k) { v += 1.0; }, RawBindingToken::MouseBtn(mb) => if mouse_buttons.pressed(mb) { v += 1.0; }, _=>{} } match va.neg { RawBindingToken::Key(k) => if keyboard.pressed(k) { v -= 1.0; }, RawBindingToken::MouseBtn(mb) => if mouse_buttons.pressed(mb) { v -= 1.0; }, _=>{} } vals[i] = (v * va.scale).clamp(-1.0,1.0); } input_map.virtual_axis_values = vals; }
}

pub fn system_evaluate_bindings(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut input_map: ResMut<InputMap>,
) {
    let dt = time.delta_secs();
    // Ensure runtime sized without simultaneous immutable+mutable borrow
    let total_bindings = input_map.bindings.len();
    if input_map.bindings_runtime.len() < total_bindings { input_map.bindings_runtime.resize(total_bindings, BindingRuntime::default()); }
    // Reset transient flags
    for brt in &mut input_map.bindings_runtime { brt.just_released = false; brt.just_pressed = false; }
    // Evaluate each binding (clone lightweight metadata to avoid borrow conflicts)
    let bindings_snapshot = input_map.bindings.clone();
    for binding in bindings_snapshot { let mut all_active = true; for token in &binding.tokens { let active = match token { RawBindingToken::Key(k) => keyboard.pressed(*k), RawBindingToken::MouseBtn(b) => mouse_buttons.pressed(*b), _ => false }; if !active { all_active = false; break; } } let rt = &mut input_map.bindings_runtime[binding.id as usize]; if all_active { if !rt.active { rt.active = true; rt.just_pressed = true; rt.hold_elapsed = 0.0; } else { rt.hold_elapsed += dt; } } else if rt.active { rt.active = false; rt.just_released = true; } }
    // Snapshot binding -> action index pairs to avoid borrow conflicts
    let bindings_index_snapshot: Vec<(ActionId, Vec<u32>)> = input_map.bindings_index.iter().map(|(k,v)| (*k, v.clone())).collect();
    for (aid, bids) in bindings_index_snapshot { for bid in bids { let (eligible, active, jp, jr) = { let rt = &input_map.bindings_runtime[bid as usize]; let binding = &input_map.bindings[bid as usize]; let meets_hold = !(binding.hold_secs > 0.0 && rt.hold_elapsed < binding.hold_secs); (meets_hold, rt.active, rt.just_pressed, rt.just_released) }; if !eligible { continue; } if let Some(state) = input_map.dynamic_states.get_mut(aid.0 as usize) { if let Some(b) = state.as_binary_mut() { if jp { b.just_pressed = true; b.pressed = true; } if jr { b.just_released = true; b.pressed = false; } if active { b.pressed = true; } } } } }
}
