# game_physics

Rapier2D physics integration providing realistic ball physics, paddle kinematics, and clustering forces for the metalrain game.

## Description

This crate integrates the Rapier2D physics engine with the game's ECS architecture. It handles automatic physics body creation for game entities, applies custom forces (like clustering), enforces velocity constraints, and synchronizes physics state back to game components. The system provides runtime-configurable physics parameters for tuning gameplay feel.

## Purpose

**For Users:**

- Provides realistic ball physics with proper collision response
- Enables dynamic gameplay through configurable physics parameters
- Implements ball clustering behavior for visual appeal
- Ensures smooth, predictable paddle control

**For Downstream Developers:**

- Automatic physics body and collider creation for game entities
- Clean separation between physics and game logic
- Runtime-tunable parameters without code changes
- Handles synchronization between Rapier and game components
- Provides foundation for advanced physics behaviors

## Key API Components

### Plugin

- **`GamePhysicsPlugin`** - Main plugin that sets up Rapier and all physics systems

### Resources

- **`PhysicsConfig`** - Runtime-configurable physics parameters
  - `gravity: Vec2` - Global gravity vector (default: `Vec2::new(0.0, -980.0)`)
  - `min_ball_speed: f32` - Minimum ball velocity magnitude
  - `max_ball_speed: f32` - Maximum ball velocity magnitude
  - `clustering_strength: f32` - Force multiplier for ball clustering
  - `clustering_radius: f32` - Distance within which balls attract
  - `paddle_speed: f32` - Paddle movement speed
  - `paddle_bounds: Rect` - Paddle movement constraints

### Systems

The plugin registers the following systems in the `Update` schedule:

- `attach_paddle_kinematic_physics` - Creates kinematic rigid bodies for paddles
- `spawn_physics_for_new_balls` - Attaches physics bodies to newly spawned balls
- `drive_paddle_velocity` - Converts input to paddle velocity
- `apply_clustering_forces` - Applies attractive forces between nearby balls
- `apply_config_gravity` - Applies configured gravity to all balls
- `sync_physics_to_balls` - Syncs Rapier velocity back to `Ball` component
- `clamp_velocities` - Enforces min/max speed limits
- `clamp_paddle_positions` - Keeps paddles within bounds
- `handle_collision_events` - Processes Rapier collision events

## Usage Example

```rust
use bevy::prelude::*;
use game_core::{GameCorePlugin, BallBundle, PaddleBundle, GameColor};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_systems(Startup, (setup_physics, spawn_entities))
        .run();
}

fn setup_physics(mut config: ResMut<PhysicsConfig>) {
    // Customize physics parameters
    config.gravity = Vec2::new(0.0, -500.0); // Lighter gravity
    config.max_ball_speed = 800.0;
    config.clustering_strength = 150.0;
    config.clustering_radius = 100.0;
}

fn spawn_entities(mut commands: Commands) {
    // Spawn a ball - physics will be automatically attached
    commands.spawn(BallBundle::new(
        Vec2::new(0.0, 200.0),
        16.0,
        GameColor::Blue,
    ));
    
    // Spawn a paddle - will become kinematic automatically
    commands.spawn(PaddleBundle::new(
        Vec2::new(0.0, -250.0),
        Default::default(),
    ));
}

// Runtime physics tuning example
fn tune_physics(
    input: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<PhysicsConfig>,
) {
    if input.just_pressed(KeyCode::KeyG) {
        // Toggle gravity
        config.gravity.y *= -1.0;
    }
    
    if input.pressed(KeyCode::Equal) {
        // Increase clustering
        config.clustering_strength += 10.0;
    }
    
    if input.pressed(KeyCode::Minus) {
        // Decrease clustering
        config.clustering_strength = (config.clustering_strength - 10.0).max(0.0);
    }
}
```

## Physics Configuration

### Default Values

```rust
PhysicsConfig {
    gravity: Vec2::new(0.0, -980.0),
    min_ball_speed: 50.0,
    max_ball_speed: 1200.0,
    clustering_strength: 200.0,
    clustering_radius: 80.0,
    paddle_speed: 500.0,
    paddle_bounds: Rect::from_center_size(Vec2::ZERO, Vec2::new(800.0, 600.0)),
}
```

### Tuning Guidelines

- **Gravity**: Higher magnitude = faster falling. Can be zero for space-like physics
- **Clustering Strength**: Higher = stronger attraction between balls. 0 disables clustering
- **Clustering Radius**: Distance within which balls influence each other
- **Min/Max Ball Speed**: Keeps gameplay readable and prevents physics instabilities
- **Paddle Speed**: Affects responsiveness of player control

## Integration Notes

### Automatic Physics Bodies

The plugin automatically detects and configures physics for:

- **Balls**: Dynamic rigid bodies with circle colliders based on radius
- **Paddles**: Kinematic rigid bodies with box colliders based on half_extents

### Collision Handling

The crate uses Rapier's collision events to:

- Detect ball-target impacts
- Handle ball-wall bounces
- Process paddle-ball interactions

### Clustering Physics

The clustering system implements a simple O(n²) algorithm that applies attractive forces between balls within the clustering radius. This creates visually interesting behavior where balls tend to group together. For large numbers of balls, spatial partitioning optimization may be needed.

## Dependencies

- `bevy` - Core ECS functionality
- `bevy_rapier2d` - 2D physics engine integration
- `rand` - Random number generation for physics variations
- `game_core` - Core game component definitions

## Testing

The crate includes comprehensive unit tests covering:

- Physics configuration validation
- Automatic physics body attachment
- Paddle velocity system
- Component synchronization

Run tests with:

```bash
cargo test -p game_physics
```

## Performance Considerations

- Clustering forces are O(n²) in ball count - consider disabling for >100 balls
- Velocity clamping runs every frame on all balls
- Rapier physics runs at fixed timestep (configured via Rapier plugin)
- Consider using Rapier's spatial query acceleration for large scenes

## Demo: physics_playground

An interactive demo is available to experiment with physics parameters:

```bash
cargo run -p physics_playground
```

Controls:

- Left Click: spawn a ball at cursor with random velocity
- UI sliders: adjust gravity, clustering, speed limits in real-time
- Velocity gizmos: yellow lines show current velocity vectors
