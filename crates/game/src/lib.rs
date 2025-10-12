use bevy::prelude::*;
use game_core::{BallBundle, BallSpawned, GameColor, GameCorePlugin, GameLost, GameWon};

pub struct GamePlugin;
impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCorePlugin)
            .add_systems(Startup, spawn_demo_ball)
            .add_systems(Update, (log_ball_spawned, simulate_win_condition));
    }
}

fn spawn_demo_ball(mut commands: Commands) {
    commands.spawn(BallBundle::new(
        Vec2::new(0.0, 0.0),
        16.0,
        GameColor::Yellow,
    ));
}

fn log_ball_spawned(mut reader: EventReader<BallSpawned>) {
    for _ in reader.read() {
        info!("BallSpawned event observed in GamePlugin");
    }
}

fn simulate_win_condition(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<f32>,
    mut events_won: EventWriter<GameWon>,
) {
    *timer += time.delta_secs();
    if *timer > 0.25 {
        events_won.write(GameWon);
        *timer = f32::MIN;
        commands.spawn_empty();
    }
}

#[allow(dead_code)]
fn log_end_events(mut won: EventReader<GameWon>, mut lost: EventReader<GameLost>) {
    if won.read().next().is_some() {
        info!("GameWon event received");
    }
    if lost.read().next().is_some() {
        info!("GameLost event received");
    }
}
