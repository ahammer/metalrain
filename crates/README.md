# Metalrain Crates

This directory contains the modular crate architecture for the metalrain project - a game built with Bevy engine featuring metaball rendering, physics simulation, and a multi-layer rendering pipeline.

## Architecture Overview

The project is organized into specialized crates that can be composed together, enabling code reuse, independent testing, and clear separation of concerns. Each crate focuses on a specific domain and exposes clean APIs for integration.

## Crate Dependency Graph

```
game (top-level integration)
├── game_core (foundational types)
├── game_physics (Rapier2D integration)
│   └── game_core
├── game_rendering (multi-layer pipeline)
│   ├── game_core
│   └── metaball_renderer
├── widget_renderer (entity visuals)
│   ├── game_core
│   └── game_physics
├── background_renderer (background effects)
│   └── game_assets
├── metaball_renderer (GPU compute rendering)
│   └── game_assets
├── event_core (input/event pipeline)
└── game_assets (centralized assets)
```

## Core Crates

### game_core

**Foundational ECS types** - Components, resources, events, and bundles that define the game's domain model.

- Components: `Ball`, `Paddle`, `Target`, `Wall`, `Hazard`, `SpawnPoint`
- Events: `BallSpawned`, `TargetDestroyed`, `GameWon`, `GameLost`
- Resources: `GameState`, `ArenaConfig`

**Use when:** You need core game entity definitions shared across systems.

[📖 Full Documentation](./game_core/README.md)

### game_assets

**Centralized asset management** - Loads and provides typed handles to fonts, shaders, and other resources.

- Eliminates hardcoded asset paths
- Handles workspace structure complexity (demos, tests, workspace root)
- Future: embedded assets for web builds

**Use when:** You need access to game fonts, shaders, or centralized asset loading.

[📖 Full Documentation](./game_assets/README.md)

## Physics & Simulation

### game_physics

**Rapier2D physics integration** - Realistic ball physics, paddle kinematics, and clustering forces.

- Automatic physics body creation for game entities
- Runtime-configurable parameters (gravity, clustering, velocity limits)
- Synchronization between physics and game components
- Ball clustering behavior for visual appeal

**Use when:** You need realistic physics simulation for gameplay.

[📖 Full Documentation](./game_physics/README.md)

### event_core

**Deterministic event pipeline** - Redux-inspired architecture for input processing and game state changes.

- Frame-atomic event processing with journaling
- Middleware chain (key mapping, debouncing, cooldowns)
- Handler registry for game state mutations
- Testable, replayable event stream

**Use when:** You need structured input handling, event logging, or replay functionality.

[📖 Full Documentation](./event_core/README.md)

## Rendering

### game_rendering

**Multi-layer rendering pipeline** - Compositor with per-layer targets and camera effects.

- Four render layers: Background, GameWorld, Metaballs, UI
- Compositor with blend modes (Normal, Additive, Multiply, Screen)
- Camera system with shake and zoom effects
- Runtime layer toggling and configuration

**Use when:** You need layered rendering with compositing and camera effects.

[📖 Full Documentation](./game_rendering/README.md)

### metaball_renderer

**GPU compute metaball rendering** - High-performance blob visuals with field and albedo textures.

- Compute shader-based rendering (2-pass: field + normals)
- World-space to texture-space coordinate mapping
- Dynamic metaballs with colors and clustering
- Offscreen rendering for flexible compositing

**Use when:** You need smooth, organic blob rendering.

[📖 Full Documentation](./metaball_renderer/README.md)

### widget_renderer

**Game entity visuals** - Sprite-based rendering for walls, targets, paddles, hazards, and spawn points.

- Automatic visual spawning when game components added
- Animations: hit flashes, destruction effects, pulsing
- Health visualization through opacity
- Integration with multi-layer pipeline (RenderLayers::layer(1))

**Use when:** You need visual representations of game entities.

[📖 Full Documentation](./widget_renderer/README.md)

### background_renderer

**Background rendering system** - Configurable backgrounds with multiple visual modes.

- Modes: Solid, Linear Gradient, Radial Gradient, Animated
- Runtime-configurable colors, angles, and animation
- Material2D-based shader rendering
- Custom WGSL shader support

**Use when:** You need customizable game backgrounds.

[📖 Full Documentation](./background_renderer/README.md)

## Integration

### game

**Top-level integration crate** - Assembles all subsystems into a complete game.

- Coordinates between game_core, physics, rendering
- Implements high-level game logic (win/lose conditions)
- Serves as reference implementation
- Currently minimal, expanding with gameplay features

**Use when:** You want a complete, integrated game setup.

[📖 Full Documentation](./game/README.md)

## Getting Started

### Running Demos

The project includes several demo applications in `demos/` that showcase individual crates:

```bash
# Physics simulation playground
cargo run -p physics_playground

# Metaball rendering test
cargo run -p metaballs_test

# Compositor demonstration
cargo run -p compositor_test

# Full architecture test
cargo run -p architecture_test
```

### Using Crates in Your Project

To use individual crates, add them as dependencies:

```toml
[dependencies]
game_core = { path = "../path/to/crates/game_core" }
game_physics = { path = "../path/to/crates/game_physics" }
game_rendering = { path = "../path/to/crates/game_rendering" }
```

### Minimal Game Setup

```rust
use bevy::prelude::*;
use game_core::GameCorePlugin;
use game_physics::GamePhysicsPlugin;
use game_rendering::GameRenderingPlugin;
use widget_renderer::WidgetRendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            GameCorePlugin,
            GamePhysicsPlugin,
            GameRenderingPlugin,
            WidgetRendererPlugin,
        ))
        .add_systems(Startup, spawn_game_world)
        .run();
}
```

## Design Principles

### Modularity

Each crate has a single, well-defined responsibility. Functionality is pushed down to the most appropriate crate rather than accumulating in high-level integration crates.

### Loose Coupling

Crates depend on abstractions (components, events) rather than implementations. This enables:

- Independent development and testing
- Flexible composition
- System substitution (e.g., swap physics engines)

### Clean APIs

Public APIs are minimal and focused. Internal implementation details are kept private. Resources and components use clear, documented types.

### Testing

Each crate maintains its own test suite. Integration tests in demos verify inter-crate communication. Property-based and determinism tests ensure reliability.

## Development Workflow

### Building All Crates

```bash
# From workspace root
cargo build --all

# Release build
cargo build --all --release
```

### Running Tests

```bash
# All crates
cargo test --all

# Specific crate
cargo test -p game_core
cargo test -p event_core
```

### Checking Documentation

```bash
# Generate docs for all crates
cargo doc --all --no-deps --open

# Specific crate
cargo doc -p metaball_renderer --open
```

### Formatting and Linting

```bash
# Format all code
cargo fmt --all

# Check for issues
cargo clippy --all -- -D warnings
```

## Crate Maturity

| Crate | Status | Stability | Documentation |
|-------|--------|-----------|---------------|
| game_core | ✅ Stable | High | Complete |
| game_assets | ✅ Stable | High | Complete |
| game_physics | ✅ Stable | High | Complete |
| event_core | ✅ Stable | High | Complete |
| game_rendering | ⚠️ Active Dev | Medium | Complete |
| metaball_renderer | ⚠️ Active Dev | Medium | Complete |
| widget_renderer | ✅ Stable | High | Complete |
| background_renderer | ✅ Stable | High | Complete |
| game | 🚧 Early | Low | Complete |

**Legend:**

- ✅ Stable: API unlikely to change significantly
- ⚠️ Active Dev: Functional but evolving
- 🚧 Early: Minimal implementation, expect changes

## Common Patterns

### Spawning Entities with Multiple Crates

```rust
use game_core::{BallBundle, GameColor};
use bevy::prelude::*;

// game_core provides the bundle
// game_physics automatically adds RigidBody + Collider
// widget_renderer automatically adds Sprite visuals
// metaball_renderer can optionally render as metaball

commands.spawn(BallBundle::new(
    Vec2::new(100.0, 200.0),
    16.0,
    GameColor::Blue,
));
```

### Accessing Render Outputs

```rust
use metaball_renderer::metaball_textures;
use game_rendering::RenderTargetHandles;

// Get metaball textures for custom processing
if let Some((field, albedo)) = metaball_textures(&world) {
    // Use in custom materials or effects
}

// Access layer render targets
fn use_layer_textures(handles: Res<RenderTargetHandles>) {
    if let Some(game_world_tex) = handles.game_world.as_ref() {
        // Use GameWorld layer texture
    }
}
```

### Event-Driven Game Logic

```rust
use game_core::{BallSpawned, TargetDestroyed, GameWon};
use event_core::{EventCorePlugin, EventHandler};

// Listen to core game events
fn handle_gameplay_events(
    mut ball_spawned: EventReader<BallSpawned>,
    mut target_destroyed: EventReader<TargetDestroyed>,
) {
    for event in ball_spawned.read() {
        info!("Ball spawned: {:?}", event);
    }
    
    for event in target_destroyed.read() {
        // Award points, trigger effects, etc.
    }
}
```

## Contributing

When adding new functionality, consider:

1. **Which crate should it live in?** - Use existing crates when functionality fits their domain
2. **Does it need a new crate?** - Create one if it's a distinct, reusable concern
3. **Dependencies** - Minimize coupling, depend on abstractions
4. **Tests** - Add unit tests in the crate, integration tests in demos
5. **Documentation** - Update the crate README and add doc comments

## Troubleshooting

### Asset Loading Issues

Ensure you're using `game_assets` configuration helpers:

```rust
use game_assets::configure_demo;

let mut app = App::new();
configure_demo(&mut app); // Automatically sets correct asset paths
```

### Physics Not Working

Make sure entities have both game components AND Transform:

```rust
commands.spawn((
    Ball { /* ... */ },
    Transform::from_xyz(0.0, 0.0, 0.0), // Required!
    GlobalTransform::default(),
));
```

### Visuals Not Appearing

Check render layers match your setup:

```rust
use bevy::render::view::RenderLayers;

commands.spawn((
    Sprite::default(),
    RenderLayers::layer(1), // GameWorld layer
    Transform::default(),
));
```

## Performance Tips

- **Clustering**: Disable for >100 balls (`PhysicsConfig.clustering_strength = 0.0`)
- **Metaballs**: Lower texture resolution for mobile (`MetaballRenderSettings.with_resolution(854, 480)`)
- **Render Layers**: Use targeted layers to reduce overdraw
- **Event Pipeline**: Keep handlers lightweight, defer heavy work to regular systems

## License

All crates are dual-licensed under MIT or Apache-2.0 at your option.

## Questions?

Each crate has detailed documentation in its README.md. For integration examples, see the demo applications in the `demos/` directory.
