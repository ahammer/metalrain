# event_core

Deterministic event and input reduction pipeline providing a Redux-inspired architecture for the metalrain project.

## Description

This crate implements a centralized event processing system that decouples raw input from game state mutations. It provides a deterministic, testable pipeline with middleware support for input transformation, debouncing, cooldowns, and event journaling. The architecture ensures that all high-level game state changes flow through a single, auditable path.

The system uses frame-atomic processing: all events enqueued during a frame are processed together in `PostUpdate`, with any handler-emitted events deferred to the next frame. This eliminates reentrancy hazards and ensures predictable ordering.

## Purpose

**For Users:**

- Provides consistent, responsive input handling
- Enables features like input rebinding and accessibility options
- Supports replay and debugging through event journaling
- Ensures predictable game behavior through deterministic processing

**For Downstream Developers:**

- Decouples input handling from game logic
- Provides testable, replayable event stream
- Eliminates ad-hoc debounce/cooldown code scattered across systems
- Enables debugging through event journal inspection
- Supports middleware for cross-cutting concerns (logging, validation, transformation)
- Clean extension points for custom event types and handlers

## Key API Components

### Plugin

- **`EventCorePlugin`** - Main plugin that sets up the event pipeline
  - `journal_capacity: usize` - Maximum events to retain in journal (default: 512)

### Resources

- **`EventQueue`** - Central event queue and journal
  - `enqueue(envelope, frame)` - Add event to queue
  - `journal()` - Iterator over recent processed events (ring buffer)

- **`HandlerRegistry`** - Registry of event handlers
  - `register<H: EventHandler>(handler)` - Add event handler

- **`MiddlewareChain`** - Ordered chain of middleware processors
  - `add<M: Middleware>(middleware)` - Add middleware to chain

- **`FrameCounter`** - Monotonic frame counter for event ordering
  - `0: u64` - Current frame number

### Traits

- **`EventHandler`** - Implement to handle processed events
  - `handle(&self, event: &EventEnvelope, world: &mut World)`

- **`Middleware`** - Implement to transform/filter events
  - `process(&mut self, envelope: EventEnvelope, world: &World) -> Option<EventEnvelope>`

### System Sets

- **`EventFlowSet`** - Ordered system sets for input processing
  - `InputCollect` - Gather raw device input
  - `InputProcess` - Transform input into domain events
  - `UIUpdate` - Update overlays based on processed input

### Extension Trait

- **`EventCoreAppExt`** - Builder-style registration methods
  - `register_handler<H>(handler)` - Register event handler
  - `register_middleware<M>(middleware)` - Register middleware

## Architecture

```text
Input(KeyDown) -> [KeyMapping] -> GameEvent -> [Debounce] -> [Cooldown] 
    -> Queue(Frame N) -> Reducer(PostUpdate) -> Handlers -> ECS World
                              |                      |
                              +--> Journal (ring buffer, N most recent)
```

### Event Flow

1. Raw input collected during `EventFlowSet::InputCollect`
2. Input transformed to domain events in `EventFlowSet::InputProcess`
3. Events pass through middleware chain (filtering, transformation)
4. Processed events queued atomically by frame
5. `reducer_system` in `PostUpdate` drains queue and invokes handlers
6. All processed events stored in journal ring buffer
7. Handler-emitted events deferred to next frame

## Usage Example

### Basic Setup

```rust
use bevy::prelude::*;
use event_core::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EventCorePlugin { journal_capacity: 256 })
        .register_middleware(KeyMappingMiddleware::with_default_gameplay())
        .register_middleware(DebounceMiddleware::new(0))
        .register_middleware(CooldownMiddleware::new(2))
        .register_handler(handlers::BallLifecycleHandler)
        .run();
}
```

### Custom Event Handler

```rust
use bevy::prelude::*;
use event_core::*;

struct MyGameHandler;

impl EventHandler for MyGameHandler {
    fn handle(&self, envelope: &EventEnvelope, world: &mut World) {
        match &envelope.payload {
            EventPayload::Game(GameEvent::ResetLevel) => {
                // Reset game state
                info!("Resetting level at frame {}", envelope.frame);
                // Modify world resources/entities...
            }
            EventPayload::Game(GameEvent::PauseGame) => {
                // Toggle pause state
                if let Some(mut state) = world.get_resource_mut::<GameState>() {
                    state.is_paused = !state.is_paused;
                }
            }
            _ => {}
        }
    }
}

// Register in app
app.register_handler(MyGameHandler);
```

### Custom Middleware

```rust
use event_core::*;

struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn process(
        &mut self,
        envelope: EventEnvelope,
        _world: &World,
    ) -> Option<EventEnvelope> {
        info!("Event: {:?} at frame {}", envelope.payload, envelope.frame);
        Some(envelope) // Pass through
    }
}

app.register_middleware(LoggingMiddleware);
```

### Custom Key Mapping

```rust
use event_core::*;

let mut key_mapping = KeyMappingMiddleware::empty();
key_mapping
    .map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel))
    .map(KeyCode::Space, KeyMappingOutput::Action(PlayerAction::PrimaryAction))
    .map(KeyCode::Escape, KeyMappingOutput::Game(GameEvent::PauseGame));

app.register_middleware(key_mapping);
```

### Enqueuing Events Programmatically

```rust
fn spawn_ball_on_input(
    mut queue: ResMut<EventQueue>,
    frame: Res<FrameCounter>,
    input: Res<ButtonInput<MouseButton>>,
) {
    if input.just_pressed(MouseButton::Left) {
        let envelope = EventEnvelope::new(
            EventPayload::Game(GameEvent::SpawnBall),
            EventSourceTag::Input,
            frame.0,
        );
        queue.enqueue(envelope, frame.0);
    }
}
```

### Accessing Event Journal

```rust
fn debug_recent_events(queue: Res<EventQueue>) {
    for event in queue.journal() {
        info!("Recent: {:?} @ frame {}", event.payload, event.frame);
    }
}
```

## Built-in Middleware

### KeyMappingMiddleware

Maps raw `KeyCode` input to high-level game events.

```rust
// Use default gameplay mapping
KeyMappingMiddleware::with_default_gameplay()

// Or build custom mapping
let mut km = KeyMappingMiddleware::empty();
km.map(KeyCode::KeyW, KeyMappingOutput::Action(PlayerAction::MoveUp))
  .map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel));
```

### DebounceMiddleware

Filters rapid repeated events within a frame window.

```rust
DebounceMiddleware::new(3) // Ignore events if same event within last 3 frames
```

### CooldownMiddleware

Enforces minimum frame separation between events.

```rust
CooldownMiddleware::new(5) // At least 5 frames between same events
```

## Event Types

The crate defines several event enums in `event.rs`:

- `EventPayload` - Top-level discriminated union
  - `Input(InputEvent)` - Raw input events
  - `Game(GameEvent)` - High-level game events
  - `Action(PlayerAction)` - Player action events

Check the source for complete event definitions and extend as needed for your game.

## Dependencies

- `bevy` - Core ECS functionality
- `serde` - (Optional) Event serialization for replay/tooling

## Features

- **`serde`** - Enables serialization of event types for debugging and replay tools

## Testing

The crate includes comprehensive tests covering:

- Event queue operations and journal management
- Middleware chaining and filtering
- Deterministic frame-based processing
- Handler registration and invocation
- Debounce and cooldown behavior

Run tests with:

```bash
cargo test -p event_core
```

## Performance Considerations

- O(n) processing per frame where n = events in that frame
- Minimal allocations (only HashMap growth in debounce/cooldown on new keys)
- Frame deferral prevents recursive event processing
- Ring buffer journal has fixed capacity (no unbounded growth)

## Design Principles

1. **Determinism**: Frame-atomic processing ensures consistent behavior
2. **Testability**: Pure functions and explicit state make testing straightforward
3. **Auditability**: Journal provides complete event history
4. **Extensibility**: Middleware and handlers are pluggable
5. **Safety**: Frame deferral eliminates reentrancy hazards

## Future Enhancements

- Time-based (wall-clock) debounce/cooldown in addition to frame-based
- Rich event payload data (positions, entities, colors)
- Replay system with serialization support
- Performance profiling and optimization tooling
- Visual event inspector/debugger UI
