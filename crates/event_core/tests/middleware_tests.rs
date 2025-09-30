use bevy::prelude::*;
use event_core::*;

fn base_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler)
        .register_handler(handlers::TargetInteractionHandler)
        .register_middleware(KeyMappingMiddleware::with_default_gameplay());
    app
}

#[test]
fn key_mapping_converts_input() {
    let mut app = base_app();
    {
        let frame = app.world().resource::<FrameCounter>().0;
        let mut q = app.world_mut().resource_mut::<EventQueue>();
        q.enqueue(EventEnvelope::new(EventPayload::Input(InputEvent::KeyDown(KeyCode::KeyR)), EventSourceTag::Input, frame), frame);
    }
    app.update();
    // Expect ResetLevel to have been handled -> TargetInteractionHandler resets counters -> TargetCounter exists
    assert!(app.world().get_resource::<handlers::TargetCounter>().is_some());
    let journal: Vec<_> = app.world().resource::<EventQueue>().journal().collect();
    assert!(journal.iter().any(|j| matches!(j.event.payload, EventPayload::Game(GameEvent::ResetLevel))));
}

#[test]
fn filter_short_circuits() {
    let mut app = base_app();
    // Add filter that blocks PauseGame mapping by filtering input key P post‑mapping (simulate by blocking GameEvent::PauseGame)
    app.register_middleware(FilterMiddleware::new("NoPause", |env| !matches!(env.payload, EventPayload::Game(GameEvent::PauseGame))));
    // enqueue P
    let frame = app.world().resource::<FrameCounter>().0;
    app.world_mut().resource_mut::<EventQueue>().enqueue(EventEnvelope::new(EventPayload::Input(InputEvent::KeyDown(KeyCode::KeyP)), EventSourceTag::Input, frame), frame);
    app.update();
    // Ensure no PauseGame in journal
    let journal: Vec<_> = app.world().resource::<EventQueue>().journal().collect();
    assert!(!journal.iter().any(|j| matches!(j.event.payload, EventPayload::Game(GameEvent::PauseGame))));
}
