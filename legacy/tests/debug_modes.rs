#![cfg(feature = "debug")]
use bevy::prelude::*;
use ball_matcher::debug::{DebugState, DebugRenderMode};
use ball_matcher::debug::keys::debug_key_input_system;

#[test]
fn mode_switch_via_keys() {
    let mut app = App::new();
    // Insert only the resources we need.
    app.insert_resource(ButtonInput::<KeyCode>::default());
    app.init_resource::<DebugState>();
    app.add_systems(Update, debug_key_input_system);

    // Press key 2 to switch to RapierWireframe
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::Digit2);
    }
    app.update();
    let state = app.world().resource::<DebugState>();
    assert_eq!(state.mode, DebugRenderMode::RapierWireframe);

    // Press key 3 for MetaballHeightfield
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::Digit3);
    }
    app.update();
    let state = app.world().resource::<DebugState>();
    assert_eq!(state.mode, DebugRenderMode::MetaballHeightfield);

    // Press key 4 for MetaballColorInfo
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::Digit4);
    }
    app.update();
    let state = app.world().resource::<DebugState>();
    assert_eq!(state.mode, DebugRenderMode::MetaballColorInfo);
}
