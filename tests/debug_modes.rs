#![cfg(feature = "debug")]
use bevy::prelude::*;
use ball_matcher::debug::{DebugState, DebugRenderMode};
use ball_matcher::debug::keys::debug_key_input_system;
use ball_matcher::interaction::inputmap::types::*;

fn minimal_input_map() -> InputMap {
    let mut actions = Vec::new();
    let mut name_to_id = std::collections::HashMap::new();
    for (i,(name, kind)) in [
        ("DebugMode1", ActionKind::Binary),
        ("DebugMode2", ActionKind::Binary),
        ("DebugMode3", ActionKind::Binary),
        ("DebugMode4", ActionKind::Binary),
        ("ToggleOverlay", ActionKind::Binary),
    ].into_iter().enumerate() { let id = ActionId(i as u16); actions.push(ActionMeta { id, name: name.to_string(), description: String::new(), kind }); name_to_id.insert(name.to_string(), id); }
    let mut dynamic_states = Vec::new(); for _ in 0..actions.len() { dynamic_states.push(ActionDynamicState::Binary(ActionStateBinary::default())); }
    InputMap { actions, name_to_id, dynamic_states, ..Default::default() }
}

#[test]
fn mode_switch_via_keys() {
    let mut app = App::new();
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.insert_resource(minimal_input_map());
    app.init_resource::<DebugState>();
    app.add_systems(Update, debug_key_input_system);

    // Helper to mark an action just pressed
    fn mark_pressed(app: &mut App, action: &str) {
        let id_opt = { let im = app.world().resource::<InputMap>(); im.name_to_id.get(action).copied() };
        if let Some(id) = id_opt { let mut im = app.world_mut().resource_mut::<InputMap>(); if let Some(ActionDynamicState::Binary(b)) = im.dynamic_states.get_mut(id.0 as usize) { b.pressed = true; b.just_pressed = true; } }
    }

    mark_pressed(&mut app, "DebugMode2");
    app.update();
    assert_eq!(app.world().resource::<DebugState>().mode, DebugRenderMode::RapierWireframe);

    mark_pressed(&mut app, "DebugMode3");
    app.update();
    assert_eq!(app.world().resource::<DebugState>().mode, DebugRenderMode::MetaballHeightfield);

    mark_pressed(&mut app, "DebugMode4");
    app.update();
    assert_eq!(app.world().resource::<DebugState>().mode, DebugRenderMode::MetaballColorInfo);
}
