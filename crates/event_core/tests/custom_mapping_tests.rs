use bevy::prelude::*;
use event_core::*;

#[test]
fn custom_mapping_overrides_default() {
    let mut km = KeyMappingMiddleware::empty();
    // Map R to PauseGame (instead of ResetLevel) and Space to ResetLevel for demonstration
    km.map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::PauseGame))
        .map(
            KeyCode::Space,
            KeyMappingOutput::Game(GameEvent::ResetLevel),
        );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::TargetInteractionHandler) // needs to handle ResetLevel if triggered
        .register_middleware(km);

    // Enqueue R key
    let frame = app.world().resource::<FrameCounter>().0;
    app.world_mut().resource_mut::<EventQueue>().enqueue(
        EventEnvelope::new(
            EventPayload::Input(InputEvent::KeyDown(KeyCode::KeyR)),
            EventSourceTag::Input,
            frame,
        ),
        frame,
    );
    app.update();
    // Journal should contain PauseGame not ResetLevel
    let journal: Vec<_> = app.world().resource::<EventQueue>().journal().collect();
    assert!(journal
        .iter()
        .any(|j| matches!(j.event.payload, EventPayload::Game(GameEvent::PauseGame))));
    assert!(!journal
        .iter()
        .any(|j| matches!(j.event.payload, EventPayload::Game(GameEvent::ResetLevel))));
}
