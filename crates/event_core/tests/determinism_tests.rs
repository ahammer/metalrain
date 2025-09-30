use bevy::prelude::*; use event_core::*;

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler)
        .register_middleware(DebounceMiddleware::new(0));
    app
}

#[test]
fn deterministic_journal_order() {
    let mut a1 = build_app();
    let mut a2 = build_app();
    // enqueue identical sequence across several frames
    for f in 0..5u64 {
        a1.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall{}, EventSourceTag::Test, f);
        a2.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall{}, EventSourceTag::Test, f);
        a1.update(); a2.update();
    }
    let j1: Vec<String> = a1.world().resource::<EventQueue>().journal().map(|e| format!("{:?}", e.event.payload)).collect();
    let j2: Vec<String> = a2.world().resource::<EventQueue>().journal().map(|e| format!("{:?}", e.event.payload)).collect();
    assert_eq!(j1, j2);
}
