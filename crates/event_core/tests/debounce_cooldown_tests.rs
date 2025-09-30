use bevy::prelude::*; use event_core::*;

#[test]
fn debounce_blocks_duplicates_same_frame() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler)
        .register_middleware(DebounceMiddleware::new(0));
    let frame = app.world().resource::<FrameCounter>().0;
    for _ in 0..5 { app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall{}, EventSourceTag::Test, frame); }
    app.update();
    // Only first should pass
    assert_eq!(app.world().resource::<handlers::BallCounter>().balls, 1);
}

#[test]
fn cooldown_blocks_until_frame_passes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler)
        .register_middleware(CooldownMiddleware::new(2));
    let mut frame = app.world().resource::<FrameCounter>().0;
    // enqueue each frame for 5 frames
    for _ in 0..5 {
        app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall{}, EventSourceTag::Test, frame);
        app.update();
        frame += 1;
    }
    // With cooldown 2, frames allowed at 0,3 (approx) => 2 spawns
    assert!( (2..=3).contains(&app.world().resource::<handlers::BallCounter>().balls));
}
