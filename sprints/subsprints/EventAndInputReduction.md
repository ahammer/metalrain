# EventAndInputReduction Subsprint

## Overview

This subsprint establishes a centralized event-driven architecture inspired by Redux patterns to consolidate world state mutations, input handling, and game actions into a single deterministic processing pipeline. The goal is to replace the current sprawled direct ECS mutations with a controlled, testable, and debuggable event system.

## Sprint Goals

Focused, high‑impact objectives for this subsprint (scoped strictly to the new `event_core` crate plus light integration surface in `game_core`):

1. Introduce a deterministic, serialized event processing lane (queue + reducer + handlers) for high‑level game actions defined by the design doc (ball lifecycle, target interaction, win/lose flow, level management, future player agency abstraction).
2. Provide middleware extensibility (mapping, debounce, cooldown, filtering) without entangling core game logic with raw device input APIs.
3. Establish a journal (ring buffer) of processed events with source, timestamp, frame index, and result classification for debugging and future replay tooling.
4. Supply minimal but representative handlers (ball spawn / loss, target hit / destroy, game win / loss, level reset) demonstrating the contract between events and ECS mutations.
5. Scaffold (but do not fully migrate) demo usage: expose clean registration APIs so individual demos can plug in their own input sources and optional debug/editor handlers without modifying core.
6. Achieve strong unit test coverage of queue determinism, middleware chaining semantics, handler invocation correctness, and journaling behavior (target ≥80% line / branch for the crate).
7. Maintain zero required changes to rendering / physics hot loops (no perf regressions in frame‑critical systems); only high‑level, non‑per‑frame mutation flows traverse the reducer.
8. Provide concise developer documentation & migration guidance to progressively refactor demos toward action vocabulary instead of ad‑hoc ECS mutations.

## Acceptance Criteria

Each criterion must be demonstrably verifiable via automated tests, doc inspection, or simple runtime assertions:

### Core Infrastructure

1. EventQueue supports FIFO ordering; test proves insertion order == dispatch order under mixed sources.
2. Queue draining is atomic per frame: events enqueued during processing are deferred to next frame (test simulates handler pushing follow‑up event; it appears next frame).
3. Journal retains the N most recent events (configurable capacity); when over capacity oldest are dropped (ring behavior test).
4. Each journal entry records: event enum variant, source tag, frame number, monotonic timestamp (mockable), and EventResult (Handled / Ignored / Error).

### Middleware

1. MiddlewareChain short‑circuits filtered events (None return) without invoking later middleware or handlers (test with counting middleware).
2. KeyMappingMiddleware maps multiple raw inputs (e.g. WASD, ArrowUp) to identical PlayerAction variants; unmapped keys pass through unchanged or are dropped (documented behavior decided & tested).
3. DebounceMiddleware prevents duplicates within configured window; advancing mock time beyond window allows next event.
4. CooldownMiddleware enforces per‑key (or per discriminant) cooldown; overlapping distinct actions do not block each other.
5. Middleware ordering is honored (test: mapping before debounce vs debounce before mapping yields expected pass/block semantics, documented recommended order).

### Handlers

1. SpawnBall handler spawns exactly one Ball entity with expected component bundle; test asserts presence & initial transform/velocity defaults.
2. BallLostToHazard handler despawns the ball and triggers GameLost emission only when no balls remain & ≥1 target present.
3. TargetHit handler either emits TargetDestroyed or leaves target intact based on (mocked) health; TargetDestroyed handler decrements target counter & emits GameWon when final target removed and ≥1 ball remains.
4. ResetLevel handler clears dynamic entities (balls, targets) while preserving structural elements (walls / hazards) as defined by a marker component strategy (test with mixed entities).

### Determinism & Isolation

1. Two identical sequences of enqueued events over two fresh worlds produce identical journal sequences (excluding timestamps) — deterministic order & results.
2. Processing cost: micro‑benchmark shows O(n) traversal w/out per event heap allocations beyond initial enqueue (assert no unexpected allocations in debug instrumentation OR doc note if minimal unavoidable ones remain).

### API & Extensibility

1. Public trait `EventHandler` + registration API allow adding a custom handler in a test crate without modifying internals.
2. Demos can register additional middleware/handlers after plugin insertion (test by adding a test handler post‑plugin and verifying invocation order).
3. Debug / editor events are conditionally compiled behind `cfg(debug_assertions)` (test via compile features or doc snippet).

### Documentation & Developer Experience

1. README (crate‑level or module docs) contains: architecture diagram (ASCII), quick start code snippet, event lifecycle description, middleware authoring example, migration checklist.
2. `cargo doc` build succeeds without warnings for the crate.

### Testing & Quality Gates

1. Unit + integration tests cover ≥80% lines (use `cargo tarpaulin` or similar—if coverage tooling deferred, include instrumentation friendly design & manual estimate with rationale).
2. All tests pass on stable Rust (version pinned in workspace) in CI / local run.
3. `cargo clippy --all-targets -- -D warnings` passes for the new crate (or documented justified allow attributes for specific lints).
4. No new warnings introduced in workspace build after adding crate (baseline comparison).

### Non‑Intrusiveness

1. Existing demos compile unchanged (aside from optionally adding the plugin) — zero mandatory edits required to adopt incrementally.
2. Opting out: If plugin not added, no behavior regressions in current demos.

## Scope Boundaries

In Scope:

- New crate `crates/event_core` implementing queue, reducer, middleware, handlers (minimal core set), journal, and registration API.
- Minimal glue in `game_core` (e.g. shared component/event type definitions) only if unavoidable; prefer event enums live in `event_core` and re‑exported if needed.
- Test scaffolding (unit + integration) including small mock components/resources for counting targets/balls.
- Developer documentation & migration notes.

Out of Scope (Deferred to later sprints / per‑demo integration):

- Full migration of existing demos to the event system (only illustrative examples / tests now).
- Replay / undo / networking synchronization mechanisms (journal design anticipates these but not implemented).
- UI overlays, particle/audio effect implementations — stubbed via comments or feature‑gated placeholders.
- Advanced input sources (gamepad, touch, remapping persistence).
- Performance tuning beyond basic O(n) correctness & absence of pathological allocations.
- Macro‑based auto registration / derive tooling.

## Deliverables

1. `crates/event_core` with: `lib.rs`, `event.rs`, `queue.rs`, `reducer.rs`, `middleware.rs`, `handlers/`, `sources/`.
2. Public `GameEvent`, `PlayerAction`, `DebugEvent` enums aligned with design doc.
3. Middleware implementations: KeyMapping, Debounce, Cooldown, (simple) Filter.
4. Core handlers: Ball lifecycle, Target interaction, Game flow, Level reset, Pause/Resume (scaffold where future dependencies not yet present).
5. Journal implementation + accessor API (read‑only slice, capacity config).
6. Test suite: unit (middleware, queue, handlers) + integration (determinism, chaining, incremental adoption) + optional micro benchmark harness stub.
7. Crate README / rustdoc module docs including architecture and migration guide.
8. Example snippet (minimal App showing enqueue → process → journal inspect).

## Definition of Done (Checklist)

- [ ] All Acceptance Criteria above satisfied (or explicitly waived with rationale recorded under a "Deviations" note in this file).
- [ ] New crate builds with `--release` and `--all-targets` without warnings.
- [ ] Tests pass locally (document rustc version).
- [ ] Documentation updated (README + module docs) & linked from root `design-doc.md` or sprint index if appropriate.
- [ ] Journal capacity configurable via plugin constructor or resource.
- [ ] No panics on invalid event handling paths (handlers return Error variant instead).
- [ ] CI configuration (if present) extended to include crate (or TODO note added if CI config lives elsewhere).

## Verification & Measurement

| Aspect | Method | Target |
|--------|--------|--------|
| Determinism | Dual-run journal diff | Identical variants order |
| Coverage | Tarpaulin (or manual instrumentation) | ≥80% |
| Allocations | `cargo +nightly bench --profile=perf` / `heaptrack` (optional) | No unexpected growth vs baseline test harness |
| Latency | Measure queue process time for 1k synthetic events | << 1ms (informational) |
| API Clarity | Dev review (1 other contributor) | No blocking feedback |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Over-engineering before demos migrate | Wasted effort | Keep handlers minimal; defer features (undo, replay). |
| Performance regressions if misused for hot loops | Frame drops | Document "Do NOT use for per-frame physics/render adjustments" prominently. |
| Enum growth causing monolithic handler | Maintainability | Enforce discrete handler modules per semantic domain. |
| Middleware ordering confusion | Subtle bugs | Provide recommended order + test demonstrating difference. |
| Debug events leaking into release | Player confusion | `cfg(debug_assertions)` gating + test compile path. |

## Implementation Phases (Suggested)

1. Queue & Journal core + simple enqueue/dequeue tests.
2. Event enums + discriminant helpers + handler trait & registry.
3. Core handlers (ball, target, game flow) with mock world tests.
4. Middleware chain + KeyMapping + Debounce + tests.
5. Cooldown + Filter + ordering tests.
6. Integration tests (determinism, frame deferral) + docs first draft.
7. Bench / micro perf sanity + finalize docs + polish & DoD audit.

---

> NOTE: Demo‑specific key mappings / editor handlers should be registered within each demo crate, not added to core; this preserves modularity and keeps `event_core` lean.

## Problem Statement

**Current Issues:**

- Multiple systems directly mutate ECS world state using `Commands` and `Query<&mut T>`
- Input handling is scattered across demo systems (e.g., [`physics_playground/src/lib.rs`](demos/physics_playground/src/lib.rs) lines 144-342)
- No central authority for high-level actions like "spawn ball", "create wall", "adjust gravity"
- Difficult to debug action history or implement features like replay, undo, or multiplayer sync
- Systems tightly coupled to specific component types and input mechanisms

**Examples from Codebase:**

```rust
// physics_playground - direct ECS manipulation scattered throughout
if buttons.just_pressed(MouseButton::Right) {
    commands.spawn((wall, Transform::..., Collider::...));
}
if keys.just_pressed(KeyCode::KeyC) {
    for e in &mut clear_q { commands.entity(e).despawn(); }
}
```

## Architecture

### Core Pattern: Redux-Inspired Event Flow

```text
[Event Sources] → [Middleware Chain] → [Event Queue] → [Reducer] → [Handlers] → [ECS World]
     ↓                    ↓                   ↓            ↓           ↓
  Keyboard          KeyMapping          Sequential    Pattern      Commands
  Mouse             Debounce            Ordering      Matching     Resources
  Systems           Filtering           Journal       Dispatch     Events
  TimersTick        Transform                                    
```

### Key Components

1. **Event Sources**: Generate raw events from various inputs
2. **Middleware**: Transform, filter, or enrich events before reduction
3. **Event Queue**: Ordered, sequential processing buffer with journaling
4. **Reducer**: Central dispatcher that routes events to appropriate handlers
5. **Handlers**: Pure functions that generate ECS mutations (Commands)
6. **ECS World**: Single source of truth (mutable via handlers only)

## Crate Structure: `event_core`

```text
crates/event_core/
  Cargo.toml
  src/
    lib.rs              # Plugin, public API
    event.rs            # GameEvent enum, trait definitions
    queue.rs            # EventQueue, journal, ordering
    reducer.rs          # Reducer system, handler registry
    middleware.rs       # Middleware trait, built-in implementations
    handlers/           # Handler implementations
      mod.rs
      spawn.rs          # Spawn/despawn handlers
      physics.rs        # Physics config handlers
      input.rs          # Input action handlers
    sources/            # Event source systems
      mod.rs
      keyboard.rs
      mouse.rs
      system.rs         # System-generated events
  tests/
    integration_tests.rs
    middleware_tests.rs
    handler_tests.rs
```

## Event Type Hierarchy

### Core Game Events (Aligned with Design Doc)

```rust
/// High-level game events that represent meaningful game state changes
/// Based on the "Color Fusion" design: kinetic blobs bouncing in arena,
/// hitting targets, avoiding hazards
#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    // === Ball Lifecycle ===
    // "Launch / release initial balls (or they auto-spawn)" - Design Doc §3
    
    /// Spawns a new ball into play from a spawn point
    /// Position override allows manual placement (editor mode)
    SpawnBall {
        spawn_point: Entity,
        position: Option<Vec2>,
        color: BallColor,
    },
    
    /// Ball enters a hazard zone and is removed from play
    /// "Hazards remove balls; risk escalates as ball count drops" - Design Doc §4,5
    BallLostToHazard {
        ball: Entity,
        hazard: Entity,
    },
    
    // === Target Interaction ===
    // "Targets struck: they pop / vanish, giving progress feedback" - Design Doc §3
    
    /// Target takes damage from ball collision
    /// One-hit in MVP: "one-hit fragile object" - Design Doc §4
    TargetHit {
        target: Entity,
        ball: Entity,
        impact_position: Vec2,
    },
    
    /// Target is destroyed (health reaches zero)
    /// "provides audible/visual burst" - Design Doc §4
    TargetDestroyed {
        target: Entity,
        final_position: Vec2,
    },
    
    // === Win/Lose Conditions ===
    // "Win Condition: 0 targets left, ≥1 ball" - Design Doc §5
    // "Lose Condition: 0 balls left, ≥1 target" - Design Doc §5
    
    /// All targets cleared with at least one ball remaining
    GameWon {
        balls_remaining: u32,
        time_elapsed: f32,
    },
    
    /// Last ball lost with targets still remaining
    GameLost {
        targets_remaining: u32,
        time_elapsed: f32,
    },
    
    // === Level Flow ===
    // "Quick restart; loop repeats" - Design Doc §3
    
    /// Start a new level/round
    StartLevel {
        level_id: Option<String>, // None = restart current
    },
    
    /// Reset current level to initial state
    ResetLevel,
    
    /// Pause active gameplay
    PauseGame,
    
    /// Resume from pause
    ResumeGame,
    
    // === Player Agency (Future Expansion) ===
    // "optionally influence future launches in later expansions" - Design Doc §3
    // "Add paddle to introduce agency" - Design Doc §12
    
    /// Abstract player action (decoupled from input)
    /// Middleware translates input → PlayerAction
    PlayerAction(PlayerAction),
}

/// Abstract player actions independent of input device
/// "Strategic early target prioritization" - Design Doc §9
/// "Risk management: allow ball to approach hazard..." - Design Doc §9
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    /// Primary interaction (future: launch ball, activate paddle)
    /// Design-ready for "Add paddle to introduce agency" - §12
    PrimaryAction,
    
    /// Secondary interaction (future: slow-motion, special ability)
    SecondaryAction,
    
    /// Directional movement for paddle/cursor control
    /// WASD/Arrow keys → abstract direction
    Move(Direction2D),
    
    /// Confirm selection (menus, level select)
    Confirm,
    
    /// Cancel/back (menus, pause)
    Cancel,
    
    /// Cycle selection forward (future: color selection, powerup)
    SelectNext,
    
    /// Cycle selection backward
    SelectPrevious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction2D {
    Up,
    Down,
    Left,
    Right,
}

// === Debug/Editor Events (Non-Game) ===
// These support demos and level editing but aren't part of core game loop

/// Debug and editor-only events for testing and content creation
/// Not exposed to player during normal gameplay
#[derive(Debug, Clone, PartialEq)]
pub enum DebugEvent {
    /// Manually spawn target at position (editor)
    SpawnTarget {
        position: Vec2,
        health: u32,
        radius: f32,
    },
    
    /// Create static wall/barrier (level editor)
    /// "Wall / Barrier: defines play bounds + optional interior angles" - Design Doc §4
    SpawnWall {
        start: Vec2,
        end: Vec2,
        thickness: f32,
    },
    
    /// Create hazard zone (level editor)
    /// "Hazard (Pit / Void / Kill Zone): negative space or marked region" - Design Doc §4
    SpawnHazard {
        position: Vec2,
        size: Vec2,
    },
    
    /// Remove entity (editor cleanup)
    DespawnEntity {
        entity: Entity,
    },
    
    /// Clear all dynamic entities (balls, not level geometry)
    DespawnAllDynamic,
    
    /// Adjust physics parameters (testing)
    /// "Ball speed range (too slow = dull, too fast = unreadable)" - Design Doc §8
    SetGravity {
        gravity: Vec2,
    },
    
    AdjustGravity {
        delta: Vec2,
    },
    
    /// Tune clustering strength (metaball rendering)
    SetClusteringStrength {
        strength: f32,
    },
    
    /// Manage spawn points (editor mode)
    CreateSpawnPoint {
        position: Vec2,
    },
    
    ToggleSpawnPoint {
        entity: Entity,
    },
    
    SelectSpawnPoint {
        index: usize,
    },
    
    CycleSpawnPoint {
        direction: i32,
    },
    
    /// Toggle automatic spawning (testing)
    ToggleAutoSpawn {
        interval: Option<f32>,
    },
}
```

## Event Semantics & Design Alignment

### 1. Ball Lifecycle

**`SpawnBall`**

- **Purpose**: Introduce new ball into active gameplay
- **Design Context**: "Launch / release initial balls (or they auto-spawn)" (Design Doc §3)
- **Parameters**:
  - `spawn_point`: Which SpawnPoint entity activates (level design)
  - `position`: Override for manual placement (editor/testing only)
  - `color`: Ball color (future: color-gated targets from Design Doc §12)
- **Handler Responsibilities**:
  - Create Ball entity with physics (RigidBody, Collider)
  - Initialize velocity based on spawn point configuration
  - Emit `BallSpawned` for UI updates (ball count)
  - Play spawn sound/particle effect

**`BallLostToHazard`**

- **Purpose**: Remove ball from play, advance lose condition
- **Design Context**: "Hazards remove balls; risk escalates as ball count drops" (Design Doc §5)
- **Parameters**:
  - `ball`: Entity being removed
  - `hazard`: Which hazard zone triggered removal (for analytics/feedback)
- **Handler Responsibilities**:
  - Despawn ball entity
  - Decrement ball count
  - Check lose condition: if balls == 0 && targets > 0 → emit `GameLost`
  - Play hazard removal sound ("soft dissolve" - Design Doc §10)
  - Trigger camera shake/flash if last ball

### 2. Target Interaction

**`TargetHit`**

- **Purpose**: Register collision between ball and target
- **Design Context**: "Targets struck: they pop / vanish" (Design Doc §3)
- **Parameters**:
  - `target`: Entity being hit
  - `ball`: Colliding ball entity
  - `impact_position`: For particle effects
- **Handler Responsibilities**:
  - Reduce target health (MVP: one-hit = instant destroy)
  - If health == 0: emit `TargetDestroyed`
  - Play impact sound ("Pop" - Design Doc §10)
  - Trigger hit animation on target (flash, scale pulse)
  - Screen shake intensity scales with remaining targets

**`TargetDestroyed`**

- **Purpose**: Remove target, advance win condition
- **Design Context**: "0 targets left, ≥1 ball -> win splash" (Design Doc §5)
- **Parameters**:
  - `target`: Entity to destroy
  - `final_position`: For particle burst location
- **Handler Responsibilities**:
  - Despawn target entity
  - Decrement target count
  - Check win condition: if targets == 0 && balls >= 1 → emit `GameWon`
  - Spawn particle burst at `final_position` ("micro burst" - Design Doc §7)
  - Play destruction sound
  - If final target: "tiny screen pulse" (Design Doc §6)

### 3. Win/Lose Flow

**`GameWon`**

- **Purpose**: Player achieved victory condition
- **Design Context**: "Win: brief celebratory color flare" (Design Doc §6)
- **Parameters**:
  - `balls_remaining`: For score/analytics
  - `time_elapsed`: Round duration
- **Handler Responsibilities**:
  - Transition to Win state
  - Display win UI overlay
  - Play victory fanfare
  - Trigger "celebratory color flare" effect
  - Enable restart/next level input

**`GameLost`**

- **Purpose**: Player failed to clear targets
- **Design Context**: "Lose: subdued fade" (Design Doc §6)
- **Parameters**:
  - `targets_remaining`: For analytics
  - `time_elapsed`: How long they survived
- **Handler Responsibilities**:
  - Transition to Loss state
  - Display loss UI overlay
  - Play defeat sound (subdued)
  - Trigger "subdued fade" effect
  - Enable restart input

### 4. Level Management

**`StartLevel`**

- **Purpose**: Initialize new level from definition
- **Design Context**: "Quick restart; loop repeats" (Design Doc §3)
- **Parameters**:
  - `level_id`: Optional level to load (None = current)
- **Handler Responsibilities**:
  - Clear existing entities (balls, targets)
  - Load level definition (walls, hazards, spawn points, target layout)
  - Spawn initial targets based on "Target placement density" (Design Doc §8)
  - Configure physics (gravity, ball speed) per level tuning
  - Reset ball/target counters
  - Transition to Playing state

**`ResetLevel`**

- **Purpose**: Restart current level (shortcut to `StartLevel` with None)
- **Design Context**: "Quick restart" (Design Doc §3), "compulsion loop intact" (Design Doc §13)
- **Handler Responsibilities**: Same as `StartLevel(level_id: None)`

**`PauseGame` / `ResumeGame`**

- **Purpose**: Halt/resume physics simulation
- **Handler Responsibilities**:
  - Toggle Rapier physics time scale (0.0 / 1.0)
  - Show/hide pause menu overlay
  - Prevent input processing while paused

### 5. Player Agency (Future)

**`PlayerAction`**

- **Purpose**: Abstract layer between input and game mechanics
- **Design Context**: "Add paddle to introduce agency" (Design Doc §12)
- **Why Middleware**: Decouples KeyCode/MouseButton from game logic
  - `Space` → `PrimaryAction` → Launch ball / Activate paddle
  - `WASD` → `Move(direction)` → Paddle movement
  - Allows rebinding, gamepad support, touch controls without changing game code
- **Handler Responsibilities** (future sprint):
  - `PrimaryAction`: Trigger ball launch from selected spawn point
  - `Move(direction)`: Update paddle velocity
  - `Confirm`/`Cancel`: Menu navigation

## Middleware Mapping Examples

### KeyMappingMiddleware (Game-Focused)

```rust
// Core gameplay mappings
Space → PlayerAction::PrimaryAction
W/↑ → PlayerAction::Move(Direction2D::Up)
S/↓ → PlayerAction::Move(Direction2D::Down)
A/← → PlayerAction::Move(Direction2D::Left)
D/→ → PlayerAction::Move(Direction2D::Right)
Enter → PlayerAction::Confirm
Escape → PlayerAction::Cancel
Tab → PlayerAction::SelectNext
Shift+Tab → PlayerAction::SelectPrevious

// Level flow
R → GameEvent::ResetLevel
P → GameEvent::PauseGame

// Debug only (filtered out in release builds)
#[cfg(debug_assertions)]
{
    F1 → DebugEvent::SpawnTarget { ... }
    F2 → DebugEvent::DespawnAllDynamic
    F3 → DebugEvent::ToggleAutoSpawn { ... }
}
```

### DebounceMiddleware

```rust
// Prevent accidental double-restart
GameEvent::ResetLevel → 500ms debounce

// Limit spawn rate in editor
DebugEvent::SpawnTarget → 100ms debounce
DebugEvent::SpawnWall → 100ms debounce
```

### CooldownMiddleware

```rust
// Future: Paddle dash ability
PlayerAction::SecondaryAction → 2s cooldown

// Future: Ball launch rate limit
GameEvent::SpawnBall → 0.5s cooldown (per spawn point)
```

## Updated Handler Registry

```rust
impl Plugin for EventCorePlugin {
    fn build(&self, app: &mut App) {
        app
            // Core game handlers
            .register_handler(BallLifecycleHandler)   // SpawnBall, BallLostToHazard
            .register_handler(TargetInteractionHandler) // TargetHit, TargetDestroyed
            .register_handler(GameFlowHandler)         // GameWon, GameLost, StartLevel, ResetLevel
            .register_handler(PauseHandler)            // PauseGame, ResumeGame
            .register_handler(PlayerActionHandler)     // PlayerAction (future paddle)
            
            // Debug/editor handlers (stripped in release)
            #[cfg(debug_assertions)]
            {
                .register_handler(EditorSpawnHandler)   // SpawnTarget, SpawnWall, SpawnHazard
                .register_handler(EditorManipHandler)   // DespawnEntity, DespawnAllDynamic
                .register_handler(PhysicsTuningHandler) // SetGravity, SetClusteringStrength
                .register_handler(SpawnPointHandler)    // CreateSpawnPoint, ToggleSpawnPoint
            }
            
            // Middleware (order matters!)
            .register_middleware(KeyMappingMiddleware::game_mode()) // Input → PlayerAction
            .register_middleware(DebounceMiddleware::default())     // Anti-spam
            .register_middleware(CooldownMiddleware::default())     // Ability cooldowns
            #[cfg(debug_assertions)]
            {
                .register_middleware(DebugFilterMiddleware)         // Strip debug events in release
            }
    }
}
```

## Design Doc Alignment Summary

| Design Doc Requirement | Event Coverage |
|------------------------|----------------|
| "Launch / release initial balls" (§3) | `SpawnBall` |
| "Targets struck: pop / vanish" (§3) | `TargetHit` → `TargetDestroyed` |
| "Hazards remove balls" (§3,5) | `BallLostToHazard` |
| "All targets cleared → win" (§5) | `TargetDestroyed` → `GameWon` |
| "Last ball lost → lose" (§5) | `BallLostToHazard` → `GameLost` |
| "Quick restart" (§3) | `ResetLevel` |
| "Add paddle (future)" (§12) | `PlayerAction::Move` (scaffolded) |
| "Color-gated targets (future)" (§12) | `BallColor` field in `SpawnBall` |
| "Binary, immediate outcome" (§5) | No partial states; events atomic |
| "Average round length < 60s" (§5) | `time_elapsed` tracked in Win/Loss events |
