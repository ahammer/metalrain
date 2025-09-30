# event_core

Deterministic event + input reduction pipeline for the metalrain project.

## Status

Implemented (Sprint Subset Complete):

```text
Input(KeyDown) -> [KeyMapping] -> GameEvent -> [Debounce] -> [Cooldown] -> Queue(Frame N) -> Reducer(PostUpdate) -> Handlers -> ECS World
                                                     |                                           |
                                                     +--> Journal (ring buffer, most recent N) <-+
```

- Event enums (minimal field set; placeholders where future data planned)
- Input unification via `InputEvent::KeyDown` + `KeyMappingMiddleware`
- Frame‑atomic queue draining with deferral of handler-emitted events (processed next frame)
- Ring buffer journal retaining most recent N events (configurable via plugin)
- Middleware: Filter, KeyMapping, Debounce (frame window), Cooldown (frame separation)
- Handler registry with basic Ball + Target interaction + Reset logic (resource counters)
- Determinism + middleware semantics + queue/journal tests
- Clippy clean (`-D warnings`)

Deferred (Future Enhancement Opportunities):

- Rich event payload data (positions, entities, colors) beyond stubs
- Time‑based (wall‑clock) debounce/cooldown; current implementation is frame-based
- Mockable time source for deterministic timestamps (journal uses monotonic now)
- Advanced handlers (win/lose emission logic, level loading) & editor/debug events
- Replay tooling & serialization / serde feature opt‑in usage
- Coverage tooling integration (tarpaulin) – design kept simple for instrumentation later

## Quick Start

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

To enqueue a raw key input (custom source system):

```rust
fn inject_r_key(world: &mut World) {
    let frame = world.resource::<FrameCounter>().0;
    world.resource_mut::<EventQueue>()
        .enqueue(EventEnvelope::new(EventPayload::Input(InputEvent::KeyDown(KeyCode::KeyR)), EventSourceTag::Input, frame), frame);
}
```

## Migration Notes

1. Start by adding `EventCorePlugin` to the app; no other systems need to change.
2. Replace direct key handling branches with injection of `InputEvent::KeyDown` (or in future higher-level action events).
3. Gradually move spawn / reset / pause logic into handlers; keep perf‑critical per‑frame logic out of the event path.
4. Use middleware to consolidate mapping & throttling instead of sprinkling ad‑hoc timing checks.

## Design Intent

The long‑term architecture mirrors a Redux style reducer lane to make high‑level game state transitions explicit, traceable, and replayable.

Current minimal implementation focuses on the scaffolding needed to: (a) prove determinism, (b) enforce atomic per‑frame processing, (c) demonstrate middleware chaining, (d) offer a clean extension surface.

## Journal Access

Read via `world.resource::<EventQueue>().journal()` returning an iterator over recent processed events (most recent at the back). Oldest dropped when capacity exceeded.

## Extending Middleware

Implement `Middleware` and call `app.register_middleware(MyMw)`. Return `None` to short‑circuit downstream chain & handlers.

### Configurable Key Mapping

`KeyMappingMiddleware` is now configurable instead of hardcoded:

```rust
// Custom remap: swap Reset (R) with Pause (P), add Space -> PrimaryAction
let mut km = KeyMappingMiddleware::empty();
km.map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::PauseGame))
    .map(KeyCode::KeyP, KeyMappingOutput::Game(GameEvent::ResetLevel))
    .map(KeyCode::Space, KeyMappingOutput::Action(PlayerAction::PrimaryAction));

app.register_middleware(km);
```

Use `with_default_gameplay()` to obtain the standard mapping (WASD/Arrows movement, Space primary, R reset, P pause).

## Safety / Performance Considerations

- O(n) per frame where n = events for that frame; no heap allocations during processing besides potential hashmap growth in debounce/cooldown on first encounter of a new key.
- Frame deferral ensures handlers cannot recursively process newly emitted events in the same frame (eliminates reentrancy hazards / ordering ambiguity).

## Future Work Outline

- Rich event data & feature‑gated debug/editor enums
- Handler modules by domain & win/lose logic emission
- Time‑source abstraction for ms‑based debouncing
- Replay / snapshot serialization using optional serde feature
- Macro sugar for handler registration ordering

## License

Dual-licensed under MIT or Apache-2.0.
