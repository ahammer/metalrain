# game

High-level integration crate that assembles core gameplay systems into a cohesive game experience.

## Description

This crate serves as the main game orchestration layer, combining `game_core`, `metaball_renderer`, and other systems to create the complete game. It provides the `GamePlugin` which sets up all necessary sub-plugins and implements high-level game logic like win/lose conditions, level progression, and game state management.

Currently in early development, the crate provides basic plugin integration and placeholder game logic that will be expanded with full gameplay systems.

## Purpose

**For Users:**

- Provides the complete game experience
- Manages game state and progression
- Coordinates between different game systems
- Implements win/lose conditions

**For Downstream Developers:**

- Single plugin for complete game setup
- Integration point for all game subsystems
- Reference implementation for combining metalrain crates
- Foundation for game-specific logic and content

## Key API Components

### Plugin

- **`GamePlugin`** - Main game plugin that orchestrates all subsystems
  - Adds `GameCorePlugin` for foundational types
  - Sets up basic game entities
  - Implements placeholder game logic

### Systems

Current systems (placeholders for full implementation):

- `spawn_demo_ball` - Creates an initial demo ball (Startup)
- `log_ball_spawned` - Logs ball spawn events for debugging
- `simulate_win_condition` - Placeholder win condition timer

## Usage Example

### Basic Game Setup

```rust
use bevy::prelude::*;
use game::GamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GamePlugin)
        .run();
}
```

### Extended Setup with Additional Plugins

```rust
use bevy::prelude::*;
use game::GamePlugin;
use game_physics::GamePhysicsPlugin;
use game_rendering::GameRenderingPlugin;
use widget_renderer::WidgetRendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            GamePlugin,
            GamePhysicsPlugin,
            GameRenderingPlugin,
            WidgetRendererPlugin,
        ))
        .add_systems(Startup, customize_game)
        .run();
}

fn customize_game(mut game_state: ResMut<GameState>) {
    game_state.balls_remaining = 10;
    game_state.score = 0;
}
```

### Listening to Game Events

```rust
use bevy::prelude::*;
use game_core::{GameWon, GameLost, BallSpawned};

fn handle_game_events(
    mut won_events: EventReader<GameWon>,
    mut lost_events: EventReader<GameLost>,
    mut spawn_events: EventReader<BallSpawned>,
) {
    // Handle win condition
    if won_events.read().next().is_some() {
        info!("Player won the game!");
        // Trigger victory screen, save high score, etc.
    }
    
    // Handle lose condition
    if lost_events.read().next().is_some() {
        info!("Game over!");
        // Trigger game over screen, offer retry, etc.
    }
    
    // Handle ball spawning
    for event in spawn_events.read() {
        info!("Ball spawned at {:?}", event.position);
    }
}

// Add to app
app.add_systems(Update, handle_game_events);
```

## Current Implementation

The current implementation is minimal and serves as a foundation:

```rust
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCorePlugin)
            .add_systems(Startup, spawn_demo_ball)
            .add_systems(Update, (log_ball_spawned, simulate_win_condition));
    }
}
```

### Demo Ball Spawning

A single yellow ball is spawned at the origin as a basic test entity:

```rust
fn spawn_demo_ball(mut commands: Commands) {
    commands.spawn(BallBundle::new(
        Vec2::new(0.0, 0.0),
        16.0,
        GameColor::Yellow,
    ));
}
```

### Placeholder Win Condition

Currently simulates a win after 0.25 seconds for testing event flow:

```rust
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
    }
}
```

## Dependencies

- `bevy` - Core engine functionality
- `game_core` - Core game types and events
- `metaball_renderer` - Visual metaball rendering (future integration)

## Roadmap

### Near Term

- Integrate `game_physics` for realistic ball physics
- Add level loading and target spawning systems
- Implement real win/lose condition logic
- Add score tracking and progression
- Spawn points and ball lifecycle management

### Medium Term

- Multiple level support with progression
- Difficulty scaling and dynamic spawn rates
- Combo system and score multipliers
- Power-ups and special ball types
- Pause menu and game state management

### Long Term

- Save/load system for progress persistence
- Procedural level generation
- Achievement system
- Replay system integration with `event_core`
- Customizable game modes

## Architecture Notes

The `game` crate is intentionally high-level and orchestration-focused. It should:

- Delegate low-level systems to specialized crates
- Coordinate between subsystems
- Implement game-specific logic not appropriate for reusable crates
- Serve as the integration point for all game features

Avoid putting generic, reusable functionality here - push it down to appropriate specialized crates like `game_core`, `game_physics`, or `widget_renderer`.

## Testing

As game logic is implemented, tests should cover:

- Win/lose condition triggering
- Score calculation and progression
- Level transition logic
- Game state management

Run tests with:

```bash
cargo test -p game
```

## Integration Examples

See the `demos/` directory for complete integration examples showing how to combine the game crate with rendering, physics, and input systems.
