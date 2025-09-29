# Sprint 4.5: Interactive Paddles & Spawn Points

## Sprint Goal

Introduce controllable/interactive Paddles and configurable Ball Spawn Points to enrich sandbox experimentation in the physics playground, setting the stage for structured game flow in Sprint 5.

## Context & Rationale

Sprint 4 delivered static world widgets (walls, targets, hazards). Sprint 5 will add gameplay state & progression. This intermediate sprint (4.5) adds dynamic interaction elements—player-like control (paddles) and explicit spawning logic—to validate future game loop mechanics (serve phases, respawns, scoring events) before full state management.

## Current State (From End of Sprint 4)

- ✅ Walls / Targets / Hazards rendering + physics
- ✅ Interactive placement of world elements
- ✅ Target animations and hazard pulsing
- ⚠️ No controlled entities (player proxies)
- ⚠️ Ball spawning is implicit (manual clicks only)
- ⚠️ No reusable spawn point entities
- ⚠️ No paddle collision shaping / motion constraints
- ⚠️ No distinction between user-controlled vs. world-static elements

## New Concepts

### Paddle (Dynamic Interaction Surface)

A movable rectangular (or capsule) collider that can deflect balls. Initially player-mouse or keyboard controlled; future AI or network control later.

### SpawnPoint

A persistent entity designating where new balls (or later other objects) appear. Multiple spawn points can be cycled, activated/deactivated, or triggered manually.

## Deliverables

### 1. Component & Data Model (AUDITED 2025-09-28)

- [x] `Paddle` component (size, control mode, cooldowns placeholder) – implemented in `game_core::components` with default sizes & speed
- [x] `SpawnPoint` component (radius/visual, active flag, spawn cooldown) – implemented (cooldown field present, not yet used for throttling)
- [x] Optional `ActiveSpawn` resource (tracks current rotation index) – implemented as `ActiveSpawnRotation` with advance/retreat
- [x] Basic events: `SpawnBallEvent { origin: Entity }` – implemented (`SpawnBallEvent` includes optional override position)

### 2. Rendering & Visuals (AUDITED)

- [x] Paddle visual (solid color rectangle or rounded capsule) – rectangle added in `widget_renderer::spawn_paddle_visuals`
- [x] Spawn point visual (ring + inner fill + subtle pulse) – inner disc + ring implemented
- [x] Active spawn highlight (glow / scale pulse) – pulsing system present (`update_active_spawnpoint_pulse`)
- [x] Layering consistent with existing `RenderLayers::layer(1)` – all visuals assigned layer 1

### 3. Physics Integration (AUDITED)

- [x] Paddle = kinematic body using Rapier (`attach_paddle_kinematic_physics` adds kinematic body + cuboid collider; motion via `drive_paddle_velocity` and transform clamp)
- [x] Spawn point: sensor only OR no collider – currently no collider (acceptable MVP)
- [ ] Ball spawn system consumes events, emits new ball entities at spawn transform – MISSING (events produced but no consumer system found)
- [ ] Paddle collision verified (balls deflect with correct restitution) – Colliders set; no explicit verification test / doc yet

### 4. Input & Interaction (AUDITED)

- [ ] Key/Mouse mappings:
  - [x] Left Click: Spawn ball at cursor (implemented `handle_spawn_input`)
  - [x] Shift + Left Click: Spawn via nearest active spawn point (implemented)
  - [x] S Key / Middle Click: Place Spawn Point – S implemented; Middle Click currently spawns Target (spec partially fulfilled)
  - [x] P Key: Place Paddle (implemented)
  - [x] Number Keys (1..9): Activate spawn point index (implemented)
  - [ ] Tab: Toggle debug – NOT implemented (consider adding a state or toggling Rapier debug)
  - [x] LCtrl + Scroll / Q/E: Cycle active spawn point – Q/E implemented; scroll not implemented
  - [x] Arrow Keys / WASD: Move paddle (implemented for any Player paddle)
  - [ ] Backspace/Delete: Remove hovered/selected widget – NOT implemented
- [x] Selection feedback (simple tint or outline) – `Selected` component + highlight system
- [ ] Optional: hold Space to freeze physics – NOT implemented (optional)

### 5. Systems & Plugins (AUDITED)

Code implements and registers:

```rust
pub struct PaddlePlugin; // in game_core::spawning
pub struct SpawningPlugin; // in game_core::spawning
```

Both added in `physics_playground` demo startup chain. ✅

### 6. Basic Gameplay Hooks (AUDITED)

- [x] Resource: `BallSpawnPolicy { mode: Manual | Auto(interval_s) }`
- [x] If Auto + at least one active spawn point → schedule spawns (events emitted)
- [ ] Metric counters: total spawned, active balls, despawned – struct exists (`SpawnMetrics`) but unused
- [ ] Logging for spawn origin + ball id – not present

### 7. Performance & Stability (AUDITED)

- [ ] Target perf measurement (<16 ms) – not benchmarked / recorded
- [ ] Spawn throttling – cooldown field unused; no throttling logic
- [ ] Overlap avoidance for spawn points – not implemented

### 8. Documentation (AUDITED)

- [x] Update `widget_renderer` README with paddle & spawn point section (present)
- [ ] Add usage notes to `physics_playground` README (no README yet)
- [ ] Update architecture diagram (dynamic interaction layer) – pending

## Technical Specifications

### Components (MVP)

```rust
#[derive(Component, Debug)]
pub struct Paddle {
    pub half_extents: Vec2,      // size /2
    pub move_speed: f32,         // units/sec
    pub control: PaddleControl,  // control scheme
}

#[derive(Clone, Debug)]
pub enum PaddleControl {
    Player,          // single local
    FollowCursor,    // (optional toggle)
    Static,          // not movable
    // Future: AI(…), Network(…), Scripted(…)
}

#[derive(Component, Debug)]
pub struct SpawnPoint {
    pub radius: f32,
    pub active: bool,
    pub cooldown: f32,      // seconds between spawns
    pub timer: f32,         // internal accumulator
}

#[derive(Event, Debug)]
pub struct SpawnBallEvent {
    pub spawn_entity: Entity,
}

#[derive(Resource, Default)]
pub struct ActiveSpawnRotation {
    pub indices: Vec<Entity>,
    pub current: usize,
}
```

### Paddle Motion System (Outline)

```rust
fn paddle_input_system(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&mut Velocity, &Paddle)>,
) {
    for (mut vel, paddle) in &mut q {
        if !matches!(paddle.control, PaddleControl::Player) { continue; }
        let mut dir = Vec2::ZERO;
        if keys.pressed(KeyCode::A) { dir.x -= 1.0; }
        if keys.pressed(KeyCode::D) { dir.x += 1.0; }
        if keys.pressed(KeyCode::W) { dir.y += 1.0; }
        if keys.pressed(KeyCode::S) { dir.y -= 1.0; }
        if dir.length_squared() > 0.0 {
            dir = dir.normalize();
        }
        vel.linvel = (dir * paddle.move_speed).into();
    }
}
```

### Spawn Processing

```rust
fn process_spawn_points(
    time: Res<Time>,
    mut q: Query<(Entity, &mut SpawnPoint)>,
    mut ev_writer: EventWriter<SpawnBallEvent>,
    policy: Res<BallSpawnPolicy>,
) {
    if !matches!(policy.mode, BallSpawnPolicyMode::Auto(_)) { return; }
    let interval = match policy.mode { BallSpawnPolicyMode::Auto(i) => i, _ => 0.0 };
    for (entity, mut sp) in &mut q {
        if !sp.active { continue; }
        sp.timer += time.delta_seconds();
        if sp.timer >= interval && sp.cooldown <= 0.0 {
            sp.timer = 0.0;
            ev_writer.send(SpawnBallEvent { spawn_entity: entity });
        }
    }
}
```

### Ball Spawn Consumption

```rust
fn consume_spawn_events(
    mut commands: Commands,
    mut ev: EventReader<SpawnBallEvent>,
    spawn_q: Query<&Transform, With<SpawnPoint>>,
) {
    for e in ev.read() {
        if let Ok(tf) = spawn_q.get(e.spawn_entity) {
            // Spawn ball using existing ball factory utilities (from prior sprints)
            spawn_ball(&mut commands, tf.translation.truncate());
        }
    }
}
```

(Assumes a helper `spawn_ball` already in `game_physics` / `game_core`; if not, define lightweight factory.)

### Rendering

- Paddle: `MaterialMesh2dBundle` rectangle; color maybe cyan / player color
- SpawnPoint: two-layer: outer ring (thin torus approximated with circle mesh + scale), inner disc (low alpha fill)
- Active highlight: color shift or scale oscillation (`sin(time * k)`)

### Input Placement Flow

1. Press `P` → "placing paddle" mode (UI hint optional) → first click drops paddle center
2. (MVP) Immediate fixed size (e.g. 120x20). (Future: drag to size)
3. Press `S` → create spawn point at cursor
4. Click on existing spawn point to toggle active (or number key selection)
5. `Cycle` keys rotate active pointer for spawn events

### Debug / Dev Aids

- Show paddle AABB when debug mode
- Show spawn index labels (text2d)
- Optional: spawn vector rays when a ball is emitted (faint line, ephemeral)

## Physics Considerations

- Paddle restitution > ball restitution for lively rebounds (e.g. 1.1 vs 0.9)
- Clamp paddle position inside world bounds (system clamps transform after integration)
- Ensure kinematic vs. dynamic ball collision stability (Rapier config tuned; may need CCD if fast)

## Edge Cases

- Overlapping spawn points: allow but warn (log)
- Removing active spawn point: auto-advance rotation
- No active spawn points while in Auto mode: system dormant (log once)
- Paddle placed intersecting wall: nudge outward along minimal penetration axis (deferred; MVP allow overlap)
- Excess ball count ( > threshold): auto-despawn oldest (log) (optional stretch)

## Performance Targets

| Metric | Target | Stretch |
|--------|--------|---------|
| Paddles | 8 | 12 |
| Spawn points | 10 | 20 |
| Simultaneous balls | 150 | 250 |
| Frame time | <16ms | <12ms |
| Spawn dispatch cost | <0.2ms | <0.1ms |

## Testing Strategy

### Unit Tests

- Paddle default sizes & control modes
- Spawn point activation toggle logic
- Spawn timer accumulation edge at delta overshoot

### Integration Tests

- Paddle plugin registers systems (schedule contains expected labels)
- SpawnBallEvent spawns a ball entity with required components
- Cycling active spawn points updates resource state

### Manual Tests

- Place multiple paddles; verify movement & collision
- Measure rebound angles vs. incoming angle variance
- Toggle spawn points; auto spawn operates only on active ones
- Stress test: rapid spawn placement + mass ball spawn (frame pacing)

### Potential Bench (criterion)

- Spawn event throughput (1k events no panic)
- Paddle motion system cost with N paddles

## Acceptance Criteria (AUDITED)

- [x] Paddle component & basic movement implemented
- [x] SpawnPoint component with activation + cycling
- [ ] Automatic + manual ball spawning works – auto events not consumed yet
- [x] Visuals for paddles and spawn points match layer conventions
- [ ] Physics collisions stable (no tunneling) – needs validation / CCD check
- [ ] Input mappings documented & conflict-free – partial, debug + delete not implemented
- [ ] Debug toggle reveals helpful overlays – missing
- [ ] Performance within targets at test scale – unmeasured
- [ ] Documentation updated – partial

## Definition of Done (AUDITED)

- [ ] All acceptance criteria satisfied (several outstanding)
- [ ] All new public APIs documented (add rustdoc for Paddle/SpawnPoint systems)
- [ ] No clippy warnings introduced (not verified)
- [ ] Tests pass (existing tests cover rotation & paddle physics; need spawn consumption + movement integration tests)
- [ ] Benchmarks (optional) compile (not created)
- [ ] README sections updated (widget rendering ✅, playground ❌)
- [ ] Sprints backlog updated referencing this sprint completion (pending)
- [ ] Ready handoff to Sprint 5 (blocked by missing spawn consumption & perf check)

---

### Audit Summary (2025-09-28)

Core component, rendering, input (majority), and physics foundations are in place. Remaining gaps blocking full completion:

1. Implement consumer system for `SpawnBallEvent` to actually spawn balls at spawn point transforms (and integrate metrics + logging).
2. Add spawn metrics increments & logging (origin + entity id).
3. Implement throttling using `SpawnPoint.cooldown` or per-policy limit.
4. Add debug toggle (Tab) & optional physics freeze (Space) if still desired.
5. Implement deletion (Backspace/Delete) of selected widgets.
6. Documentation: create `physics_playground` README & update architecture diagram.
7. Performance measurement & record results; optional simple benchmark harness.
8. Acceptance criteria / DoD items above then re-audit.

After these, sprint can be marked complete and handed off to Sprint 5.

## Stretch Goals (If Time Allows)

- Paddle spin / angular influence (apply tangential velocity)
- Capsule collider instead of rectangle
- Spawn point priority weighting
- Visual trail for recently spawned balls
- UI mini-panel listing spawn statuses

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Input mapping collisions | Confusion | Centralize bindings constants |
| Too many balls degrade FPS | Frame drops | Cap + auto-despawn policy |
| Paddle tunneling | Missed collisions | Enable CCD on balls if high speed |
| Over-engineering spawn scheduling | Delays | Keep MVP simple event-driven |

## Notes Toward Sprint 5

The addition of paddles and spawn points enables:

- Serve state (choose active spawn, fire ball)
- Scoring based on paddle hits or target destruction
- Level scripts controlling spawn activation patterns
- Player skill differentiation via paddle control

Focus here on reliability; defer polish (effects, SFX, UI overlays) to Sprint 5+.

---
