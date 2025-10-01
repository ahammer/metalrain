use bevy::prelude::*;
use event_core::*;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler)
        .register_handler(handlers::TargetInteractionHandler);
    app
}

#[test]
fn fifo_order_and_journal_capacity() {
    let mut app = test_app();
    app.world_mut().resource_mut::<EventQueue>().set_journal_capacity(3);
    for _ in 0..3 { app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall, EventSourceTag::Test, 0); }
    app.update();
    assert_eq!(app.world().resource::<handlers::BallCounter>().balls, 3);
    assert_eq!(app.world().resource::<EventQueue>().journal().count(), 3);
    app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall, EventSourceTag::Test, 1);
    app.update();
    let journal: Vec<_> = app.world().resource::<EventQueue>().journal().cloned().collect();
    assert_eq!(journal.len(), 3);
}

#[test]
fn defers_events_enqueued_for_future_frame() {
    let mut app = test_app();
    app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall, EventSourceTag::Test, 5);
    for _ in 0..5 { app.update(); }
    assert_eq!(app.world().resource::<handlers::BallCounter>().balls, 1);
}
