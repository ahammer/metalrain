use bevy::prelude::*;
use game_core::{BallBundle, GameColor, BallSpawned, TargetDestroyed, GameWon, GameLost, GameCorePlugin};
use game::GamePlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GameCorePlugin, GamePlugin))
        .add_systems(Startup, (spawn_ball, spawn_target))
        .add_systems(Update, (emit_events_once, observe_events, exit_after_demo))
        .insert_resource(DemoFrameCounter(0))
        .run();
}

#[derive(Resource)]
struct DemoFrameCounter(u32);

#[derive(Component)]
struct DemoTarget;

fn spawn_ball(mut commands: Commands) {
    commands.spawn(BallBundle::new(Vec2::new(10.0, 20.0), 12.0, GameColor::Red));
}

fn spawn_target(mut commands: Commands) {
    commands.spawn((DemoTarget, Name::new("DemoTarget")));
}

fn emit_events_once(
    mut did: Local<bool>,
    query: Query<Entity, With<DemoTarget>>,
    mut spawn_writer: EventWriter<BallSpawned>,
    mut target_writer: EventWriter<TargetDestroyed>,
    mut win_writer: EventWriter<GameWon>,
) {
    if *did { return; }
    if let Ok(entity) = query.single() {
        // Emit a cluster of events to validate propagation across plugin boundaries.
        spawn_writer.write(BallSpawned(entity, Ball { velocity: Vec2::ZERO, radius: 12.0, color: GameColor::Green }));
        target_writer.write(TargetDestroyed(entity, game_core::Target { health: 0, color: None }));
        win_writer.write(GameWon);
        *did = true;
    }
}

use game_core::Ball; // keep after functions that reference to satisfy ordering

fn observe_events(
    mut spawned: EventReader<BallSpawned>,
    mut destroyed: EventReader<TargetDestroyed>,
    mut won: EventReader<GameWon>,
    mut lost: EventReader<GameLost>,
) {
    for _ in spawned.read() { info!("Observed BallSpawned in architecture_test demo"); }
    for _ in destroyed.read() { info!("Observed TargetDestroyed in architecture_test demo"); }
    if won.read().next().is_some() { info!("Observed GameWon in architecture_test demo"); }
    if lost.read().next().is_some() { info!("Observed GameLost in architecture_test demo"); }
}

fn exit_after_demo(mut counter: ResMut<DemoFrameCounter>, mut exit: EventWriter<AppExit>) {
    counter.0 += 1;
    if counter.0 > 5 { // run a few frames then exit
        exit.write(AppExit::Success);
    }
}
