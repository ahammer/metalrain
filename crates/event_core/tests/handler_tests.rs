use bevy::prelude::*; use event_core::*;

#[test]
fn spawn_and_loss_flow() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EventCorePlugin::default())
        .register_handler(handlers::BallLifecycleHandler);
    // enqueue spawn then loss across frames
    let frame0 = app.world().resource::<FrameCounter>().0;
    app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::SpawnBall{}, EventSourceTag::Test, frame0);
    app.update();
    assert_eq!(app.world().resource::<handlers::BallCounter>().balls, 1);
    let frame1 = app.world().resource::<FrameCounter>().0;
    app.world_mut().resource_mut::<EventQueue>().enqueue_game(GameEvent::BallLostToHazard{}, EventSourceTag::Test, frame1);
    app.update();
    assert_eq!(app.world().resource::<handlers::BallCounter>().balls, 0);
}
