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

### 1. Component & Data Model

- [ ] `Paddle` component (size, control mode, cooldowns placeholder)
- [ ] `SpawnPoint` component (radius/visual, active flag, spawn cooldown)
- [ ] Optional `ActiveSpawn` resource (tracks current rotation index)
- [ ] Basic events: `SpawnBallEvent { origin: Entity }`

### 2. Rendering & Visuals

- [ ] Paddle visual (solid color rectangle or rounded capsule)
- [ ] Spawn point visual (ring + inner fill + subtle pulse)
- [ ] Active spawn highlight (glow / scale pulse)
- [ ] Layering consistent with existing `RenderLayers::layer(1)`

### 3. Physics Integration

- [ ] Paddle = kinematic body (or dynamic with constraints) using Rapier:
  - Shape: `Collider::cuboid(half_w, half_h)` (capsule future)
  - Motion driven via velocity update system
- [ ] Spawn point: sensor only (optional) OR no collider (MVP)
- [ ] Ball spawn system consumes events, emits new ball entities at spawn transform
- [ ] Paddle collision verified (balls deflect with correct restitution)

### 4. Input & Interaction (Physics Playground Enhancements)

- [ ] Key/Mouse mappings:
  - Left Click: (unchanged) Spawn ball at cursor (bypasses spawn points)
  - Shift + Left Click: Spawn ball at nearest active spawn point
  - S Key / Middle Click (proposed reassignment): Place Spawn Point
  - P Key: Place Paddle (click = center; drag = size optional future)
  - Number Keys (1..9): Activate spawn point index
  - Tab: (unchanged) Toggle debug
  - LCtrl + Scroll / Q/E: Cycle active spawn point
  - Arrow Keys / WASD: Move selected paddle (if exactly one paddle selected)
  - Backspace/Delete: Remove hovered/selected widget (consistent future)
- [ ] Selection feedback (simple tint or outline)
- [ ] Optional: hold Space to freeze physics (existing), still allow paddle reposition previews

### 5. Systems & Plugins

Add to `widget_renderer` or `game_core`:

```rust
pub struct PaddlePlugin;
pub struct SpawningPlugin;
```

Registered in root demo app after existing rendering/physics plugins.

### 6. Basic Gameplay Hooks (Prep for Sprint 5)

- [ ] Resource: `BallSpawnPolicy { mode: Manual | Auto(interval_s) }`
- [ ] If Auto + at least one active spawn point → schedule spawns
- [ ] Metric counters: total spawned, active balls, despawned
- [ ] Logging for spawn origin + ball id (to validate pipeline)

### 7. Performance & Stability

- [ ] Target: Up to 8 paddles, 10 spawn points, 150 balls sustained <16 ms frame (baseline measurement)
- [ ] Spawn throttling to prevent accidental bursts
- [ ] Basic overlap avoidance when placing spawn points (snap / nudge)

### 8. Documentation

- [ ] Update `widget_renderer` README with paddle & spawn point section
- [ ] Add usage notes to `physics_playground` README (if exists) or create docs snippet
- [ ] Update architecture diagram (if maintained) to include dynamic interaction layer

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

## Acceptance Criteria

- [ ] Paddle component & basic movement implemented
- [ ] SpawnPoint component with activation + cycling
- [ ] Automatic + manual ball spawning works
- [ ] Visuals for paddles and spawn points match layer conventions
- [ ] Physics collisions stable (no tunneling in normal use)
- [ ] Input mappings documented & conflict-free
- [ ] Debug toggle reveals helpful overlays
- [ ] Performance within targets at test scale
- [ ] Documentation updated

## Definition of Done

- [ ] All acceptance criteria satisfied
- [ ] All new public APIs documented (rustdoc)
- [ ] No clippy warnings introduced (baseline)
- [ ] Tests pass (unit + integration added)
- [ ] Benchmarks (optional) compile
- [ ] README sections updated (widget rendering + playground usage)
- [ ] Sprints backlog updated referencing this sprint completion
- [ ] Ready handoff to Sprint 5 (game loop / scoring)

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
