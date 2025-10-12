# game_core

Foundational ECS types (components, resources, events, and bundles) for the metalrain game architecture.

## Description

This crate provides the core game primitives that define the game's domain model. It contains minimal logic and focuses on type definitions that are shared across multiple game systems and crates. By centralizing these definitions, it ensures consistency and enables clean separation of concerns between game logic, physics, and rendering.

## Purpose

**For Users:**

- Defines the fundamental game entities (balls, paddles, targets, walls, hazards)
- Provides the event system for game state changes
- Establishes the core game state and configuration

**For Downstream Developers:**

- Single source of truth for all game entity types
- Shared component definitions prevent duplication and version conflicts
- Clean interfaces for spawning and manipulating game entities
- Event-driven architecture for loose coupling between systems
- Provides pre-configured bundles for common entity patterns

## Key API Components

### Plugin

- **`GameCorePlugin`** - Registers all events and initializes core resources

### Components

#### Gameplay Entities

- **`Ball`** - Movable ball entity
  - `velocity: Vec2` - Current velocity (synced from physics)
  - `radius: f32` - Ball radius in world units
  - `color: GameColor` - High-level color enum

- **`Paddle`** - Player or AI-controlled paddle
  - `half_extents: Vec2` - Half-width and half-height
  - `move_speed: f32` - Maximum movement speed
  - `control: PaddleControl` - Control mode (Player/FollowCursor/Static)

- **`Target`** - Destructible target entity
  - `health: u8` - Current health
  - `max_health: u8` - Maximum health
  - `radius: f32` - Target radius
  - `color: Color` - Visual color
  - `state: TargetState` - Animation state (Idle/Hit/Destroying)

- **`Wall`** - Static collision boundary
  - `start: Vec2` - Starting point
  - `end: Vec2` - Ending point
  - `thickness: f32` - Wall thickness
  - `color: Color` - Visual color

- **`Hazard`** - Environmental hazard (e.g., pits)
  - `bounds: Rect` - Hazard area
  - `hazard_type: HazardType` - Type of hazard (currently only Pit)

- **`SpawnPoint`** - Ball spawn location
  - `radius: f32` - Spawn area radius
  - `active: bool` - Whether spawning is active
  - `cooldown: f32` - Cooldown duration between spawns
  - `timer: f32` - Current cooldown timer

- **`Selected`** - Marker component for selected entities (used by editor/tools)

### Bundles

- **`BallBundle`** - Complete bundle for spawning a ball with Transform and GlobalTransform
- **`WallBundle`** - Complete bundle for spawning a wall
- **`TargetBundle`** - Complete bundle for spawning a target
- **`PaddleBundle`** - Complete bundle for spawning a paddle
- **`HazardBundle`** - Complete bundle for spawning a hazard
- **`SpawnPointBundle`** - Complete bundle for spawning a spawn point

### Events

- **`BallSpawned`** - Fired when a ball is created
  - `entity: Entity` - The spawned ball entity
  - `position: Vec2` - Initial spawn position

- **`TargetDestroyed`** - Fired when a target is destroyed
  - `entity: Entity` - The destroyed target entity
  - `position: Vec2` - Position where target was destroyed

- **`GameWon`** - Fired when win condition is met
- **`GameLost`** - Fired when lose condition is met

- **`SpawnBallEvent`** - Command event to request ball spawn
  - `position: Vec2` - Desired spawn position
  - `velocity: Vec2` - Initial velocity
  - `radius: f32` - Ball radius
  - `color: GameColor` - Ball color

### Resources

- **`GameState`** - Top-level game state
  - `score: u32` - Current score
  - `balls_remaining: u32` - Balls left to spawn
  - `is_paused: bool` - Pause state

- **`ArenaConfig`** - Arena/level configuration
  - `bounds: Rect` - Playable area bounds
  - `spawn_cooldown: f32` - Default spawn cooldown time

### Types

- **`GameColor`** - High-level color enum (Red, Green, Blue, Yellow, White)
- **`TargetState`** - Target animation state (Idle, Hit, Destroying)
- **`PaddleControl`** - Paddle control mode (Player, FollowCursor, Static)
- **`HazardType`** - Hazard type enumeration (Pit)

## Usage Example

```rust
use bevy::prelude::*;
use game_core::*;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(GameCorePlugin)
        .add_systems(Startup, spawn_game_entities)
        .add_systems(Update, handle_events)
        .run();
}

fn spawn_game_entities(mut commands: Commands) {
    // Spawn a ball using the bundle
    commands.spawn(BallBundle::new(
        Vec2::new(0.0, 100.0),
        16.0,
        GameColor::Red,
    ));
    
    // Spawn a paddle
    commands.spawn(PaddleBundle::new(
        Vec2::new(0.0, -200.0),
        Paddle {
            half_extents: Vec2::new(60.0, 10.0),
            move_speed: 400.0,
            control: PaddleControl::Player,
        },
    ));
    
    // Spawn a target
    commands.spawn(TargetBundle::new(
        Vec2::new(100.0, 150.0),
        Target::new(3, 20.0, Color::srgb(0.2, 0.8, 0.3)),
    ));
    
    // Spawn walls to form arena boundaries
    commands.spawn(WallBundle::new(
        Vec2::new(-400.0, -300.0),
        Vec2::new(-400.0, 300.0),
        10.0,
        Color::WHITE,
    ));
}

fn handle_events(
    mut ball_spawned: EventReader<BallSpawned>,
    mut target_destroyed: EventReader<TargetDestroyed>,
    mut game_won: EventReader<GameWon>,
) {
    for event in ball_spawned.read() {
        info!("Ball spawned at: {:?}", event.position);
    }
    
    for event in target_destroyed.read() {
        info!("Target destroyed at: {:?}", event.position);
    }
    
    if game_won.read().next().is_some() {
        info!("Player won the game!");
    }
}

// Requesting a ball spawn programmatically
fn request_ball_spawn(mut spawn_events: EventWriter<SpawnBallEvent>) {
    spawn_events.send(SpawnBallEvent {
        position: Vec2::new(0.0, 200.0),
        velocity: Vec2::new(100.0, -50.0),
        radius: 12.0,
        color: GameColor::Blue,
    });
}
```

## Dependencies

- `bevy` - Core ECS and math functionality
- `serde` - Serialization support for configurations

## Architecture Notes

This crate intentionally contains no business logic or system implementations. It serves as a contract between different parts of the game architecture:

- **game_physics** reads components and writes physics data
- **widget_renderer** reads components and creates visual representations  
- **game_rendering** uses components for render layer assignment
- **game** crate orchestrates systems using these types

The separation allows each system to be developed, tested, and maintained independently while sharing a common domain model.
