# UpgradePhysicsPlaygroundToEventCore Subsprint

## Overview

Integrate the new `event_core` deterministic action pipeline into the `physics_playground` demo to: (1) validate the event system against a real interactive scenario, (2) reduce direct input + ad‑hoc ECS mutation complexity, and (3) establish a reference migration pattern for other demos. This subsprint performs a targeted, incremental retrofit — **not** a ground‑up rewrite — keeping gameplay parity while routing high‑level intent through events.

## Sprint Goals

1. Add `EventCorePlugin` + middleware/handlers to `physics_playground` with a feature (or runtime toggle) for gradual adoption (`event_core_integration` default ON after completion).
2. Replace scattered keyboard/mouse conditionals (spawn ball, clear entities, reset, pause, editing utilities) with emitted `InputEvent` → mapped `GameEvent` / `PlayerAction`.
3. Introduce collision observers / systems to emit `TargetHit` and `BallLostToHazard` events instead of performing immediate despawns / counter logic inline.
4. Centralize level reset & bulk despawn into a `ResetLevel` / `DespawnAllDynamic` event path (editor/debug when applicable).
5. Maintain deterministic ordering: identical input sequence across two runs produces identical event journal (excluding timestamp) under controlled seed.
6. Reduce direct `keyboard_input` / `mouse_button_input` queries & imperative branches by ≥50% (baseline captured at start).
7. Provide an event journaling debug overlay (text UI optional / feature‑gated) listing last N processed events for developer validation.
8. Document migration delta: before/after code excerpt comparisons and guidance for future demo integrators.

## In Scope

- Modifications to `demos/physics_playground` only (plus optional new helper module under that crate for emit utilities).
- Adding a `feature = "event_core_integration"` (or cargo cfg alias) or runtime env flag to enable/disable plugin.
- Emitting `InputEvent::KeyDown` from a single consolidated input capture system (instead of scattered key checks).
- Mapping keys via configurable `KeyMappingMiddleware::with_default_gameplay()` plus per-demo overrides.
- Systems translating physics / collision results into high‑level `GameEvent` emissions.
- Lightweight UI (Bevy text) for journaling if debug assertions active.
- Local integration tests (under `physics_playground/tests/`) that spin an `App` with deterministic inputs and assert resulting journal / world state.

## Out of Scope

- Full gameplay redesign, scoring, or advanced UI.
- Editing tools or complex level authoring beyond existing functionality.
- Network / replay persistence (journal consumed only in‑process).
- Cross‑demo refactors (will treat this as reference template later).

## Acceptance Criteria

### Core Integration

1. `physics_playground` builds & runs with `EventCorePlugin` inserted when feature/toggle enabled; compiles & runs unchanged (legacy path) when disabled.
2. A single system captures keyboard/mouse (and optional pointer position) and enqueues `InputEvent::KeyDown` (and future `InputEvent` variants if needed) instead of branching logic elsewhere.
3. Key mapping configured using `KeyMappingMiddleware` (default mapping plus demo‑specific additions) producing at minimum: `ResetLevel`, `PauseGame`, `PlayerAction::PrimaryAction`, directional `Move`, and any existing spawn/clear semantics.
4. Direct ECS mutations for: spawn ball, clear dynamic entities, reset level, pause/resume are replaced by event handlers or existing `event_core` handlers (extended minimally if needed).
5. Hazard / out‑of‑bounds or kill‑zone logic emits `BallLostToHazard` events; target collisions emit `TargetHit` leading (optionally) to `TargetDestroyed` events.
6. Journal overlay (debug only) shows the last ≥16 processed events & updates each frame; toggleable via a key (e.g. F12) mapped to a debug event (if debug gating added) or a simple resource toggle.

### Determinism & Testing

1. Integration test seeds fixed RNG (if present) & injects a scripted input sequence; resulting ordered list of `GameEvent` variants matches golden set across runs.
2. Test verifying that an event emitted mid‑frame (e.g. handler triggering follow‑up spawn) appears in the **next** frame’s journal (frame deferral demonstration).
3. Test verifying key remapping works: customizing `KeyMappingMiddleware` remaps `KeyR` from reset to pause and assertions confirm outcome.

### Complexity Reduction & Metrics

1. Baseline count of direct input condition lines (e.g., `if keys.just_pressed` or `mouse_buttons.just_pressed`) recorded before changes; post‑migration count reduced by ≥50%.
2. All ball spawn / destruction side effects invoked from handlers only; zero direct despawn/spawn commands remain in input capture system.
3. Event queue depth (peak per frame) logged for a representative session and remains below a threshold (e.g., <64 events/frame) to validate no runaway generation.

### Quality Gates

1. `cargo test -p physics_playground` passes including new integration tests.
2. `cargo clippy --all-targets -D warnings` remains clean (or documented minimal allows).
3. No regressions: manual smoke run demonstrates identical visible behavior (spawn, reset, hazards, target interaction) comparing legacy vs event paths.

### Documentation & Developer Experience

1. Added `docs` section in `physics_playground/README.md` summarizing the event integration with a mini ASCII flow.
2. Subprint doc updated with a **Deviations** section if any acceptance criteria are waived.

## Key & Pointer Mappings (Extended Plan)

### Tool Palette (Numeric Keys)

Numeric keys select an "active tool" affecting what a pointer tap vs drag produces. Selection is reflected in on‑screen HUD text (debug overlay). Each selection emits a `SelectTool` (new event) or `PlayerAction::SelectNext/Previous` may cycle.

| Key | Tool | Entity/Construct Placed | Placement Gesture |
|-----|------|-------------------------|-------------------|
| 1 | Ball | Ball entity (metaball) | Tap (click release w/ small drag threshold) spawns a ball. Drag ignored (treated as tap). |
| 2 | Wall | Static wall segment (line collider) | Click + drag defines start→end line; on release emits commit. Tap spawns a minimal short segment (optional). |
| 3 | Hazard | Hazard area (rect) | Click + drag defines opposite corners of axis-aligned bounding box; tap spawns small default square. |
| 4 | Spawner | Ball spawn point | Tap places spawn point; drag optionally sets spawn direction/impulse vector (preview arrow). |
| 5 | Target | Destructible target | Tap places target; drag adjusts radius (dynamic preview). |
| 6 | Paddle | Linear paddle constraint (movement line) | Click + drag defines line segment along which future paddle entity moves; tap cycles orientation presets. |

### General Keyboard / Pointer

| Input | Event / Action | Notes |
|-------|----------------|-------|
| Space | `PlayerAction::PrimaryAction` | Contextual: spawn ball (tool 1) or confirm placement (if in preview) |
| R | `ResetLevel` | Level reset path |
| P | `PauseGame` | Toggle pause |
| C | (Debug) `DespawnAllDynamic` | Debug builds only |
| F12 | Toggle journal overlay | Debug only; toggles journal UI visibility |
| Arrows / WASD | Directional `Move` | Future paddle / camera / cursor movement (scaffold) |
| Mouse Left Tap | `PlacementTap` (derived → SpawnBall / PlaceEntity) | Distinguished from drag by distance/time threshold |
| Mouse Left Drag Start | `PlacementBegin { tool, start }` | Captures starting world position |
| Mouse Move (while dragging) | `PlacementPreview { tool, start, current }` | Throttled (e.g. every few frames) for performance |
| Mouse Left Release (after drag) | `PlacementCommit { tool, shape }` | Shape derived from start/current (line, rect, radius, vector) |
| Escape / Right Click | `PlacementCancel` or `Cancel` | Cancels current preview without commit |
| Mouse Wheel | `AdjustParameter { delta }` | Adjusts size (target radius, hazard thickness) when relevant |
| Shift (modifier) | Snap mode toggle | Snaps angles (walls 45° increments) / grid size for rects |
| Ctrl+Z (future) | Undo last placement | Depends on future journal replay tooling |

### New Event Additions (Proposed)

These extend `GameEvent` or a new `EditorEvent` (debug gated) to support rich placement semantics:

```text
SelectTool { tool: ToolKind }
PlacementBegin { tool: ToolKind, start: Vec2 }
PlacementPreview { tool: ToolKind, start: Vec2, current: Vec2 }
PlacementCommit { tool: ToolKind, shape: PlacementShape }
PlacementCancel { tool: ToolKind }
AdjustParameter { tool: ToolKind, delta: f32 }
```

`ToolKind` enum mirrors the palette (Ball, Wall, Hazard, Spawner, Target, Paddle). `PlacementShape` is a tagged enum: `Line { a: Vec2, b: Vec2 }`, `Rect { min: Vec2, max: Vec2 }`, `Point { pos: Vec2 }`, `Radius { center: Vec2, r: f32 }`, `Vector { from: Vec2, to: Vec2 }`.

### Gesture Semantics

| Tool | Tap (Click w/ movement < threshold) | Drag (Press→Move→Release) | Preview Feedback |
|------|--------------------------------------|---------------------------|------------------|
| Ball | SpawnBall event | Same as tap | None / small highlight |
| Wall | Short default segment (optional) | Line segment from start to end | Line ghost + length text |
| Hazard | Small square | Rectangle area | Translucent rect overlay |
| Spawner | Spawn point (default direction) | Start=position, vector=drag delta for launch direction | Arrow indicating direction/magnitude |
| Target | Target with default radius | Center + radius = distance(start,current) | Circle outline scaling with drag |
| Paddle | Default horizontal line (length preset) | Movement constraint line (paddle spawns/adjusts) | Line ghost + angle snap markers |

### Metaball Contraptions Goal

The above tooling enables rapid construction of Rube Goldberg-style contraptions: players place walls and hazards so metaballs (balls) fall, bounce, and route through created geometry. Success metric: user can (a) spawn multiple balls, (b) construct at least three interacting structures (wall funnel + hazard pit + paddle line) in <60s without editing raw code.

## Migration Phases

1. **Instrumentation & Baseline**: Count current input branches; snapshot relevant functions; add feature flag & no‑op plugin wiring (disabled path).
2. **Input Consolidation**: Introduce single input capture system emitting `InputEvent::KeyDown`. Remove inline spawn/reset branches (temporarily comment or behind legacy flag).
3. **Handler Routing**: Map keys to events; implement or reuse handlers for spawn/reset/clear; replace direct commands with events.
4. **Collision Emission**: Adapt collision/hazard systems to emit `TargetHit` / `BallLostToHazard` instead of immediate state mutation.
5. **Journal Overlay & Debug Mappings**: Add UI + toggle key; ensure gating under debug.
6. **Refinement & Cleanup**: Remove legacy code paths if feature flag toggles show parity; update README & metrics.
7. **Testing & Determinism Validation**: Add scenario tests; golden journal comparison; finalize docs & deviations.

## Task Breakdown (Backlog)

1. Add feature flag / cfg & plugin insertion logic.
2. Capture baseline metrics (line counts & branch list) commit as comment or README appendix.
3. Implement unified keyboard + pointer capture system → enqueue `InputEvent` (keys + mouse button + cursor position at press/move/release).
4. Implement gesture recognizer resource (tracks press start, movement delta, threshold, tool at start) emitting Placement* events.
5. Extend `event_core` or demo-local events with placement events (ToolKind, PlacementShape enums) behind feature.
6. Configure `KeyMappingMiddleware::with_default_gameplay()` + per‑demo overrides (C, F12, numeric tool selection, tool cycling via Tab/Shift+Tab).
7. Implement tool selection handler updating active tool resource & HUD.
8. Implement placement handlers: translate PlacementCommit into actual ECS entity creation per tool.
9. Add spawn ball translation: `PrimaryAction` -> `SpawnBall` only when tool=Ball & not currently dragging.
10. Refactor reset path to `ResetLevel` event.
11. Add `DespawnAllDynamic` debug event & handler (if not already present) or reuse existing.
12. Add pause/resume mapping & event emission.
13. Modify hazard detection to emit `BallLostToHazard` instead of immediate despawn.
14. Modify target collision to emit `TargetHit` and optionally `TargetDestroyed`.
15. Implement paddle constraint placement (creates/updates paddle entity on commit).
16. Add journal overlay UI component (debug) + toggle event or key resource.
17. Add HUD text for current tool + brief instructions; auto-hide after inactivity.
18. Write integration tests: mapping, reset, deferral, determinism, placement drag line, placement rect.
19. Add peak queue depth logging resource (optional) + debug print or overlay line.
20. Remove or gate legacy direct input branches.
21. Documentation updates (README + this subsprint file Deviations if needed).
22. Final metrics diff (baseline vs post counts) & record.
23. Clippy/test final pass & signoff.

## Metrics & Verification Plan

| Aspect | Method | Target |
|--------|--------|--------|
| Input branch reduction | Grep / manual count before vs after | ≥50% fewer branches |
| Determinism | Dual run golden journal comparison | Identical order/variants |
| Queue peak depth | Instrument resource tracking max length | <64 per frame |
| Event coverage | Manual table mapping features → events | 100% of targeted actions routed |
| Build health | Clippy & tests | PASS with -D warnings |

## Testing Strategy

Location: `demos/physics_playground/tests/` (new). Use `App` with `MinimalPlugins` + required subsets to accelerate tests where possible.

Test Cases:

1. `test_key_mapping_reset`: Press R → `ResetLevel` event journaled.
2. `test_spawn_primary_action`: Space → `PlayerAction::PrimaryAction` → spawn handler increments ball count resource.
3. `test_deferral_followup_event`: Handler issues a second event; ensure frame+1 processing.
4. `test_hazard_ball_loss`: Simulated hazard system emits `BallLostToHazard` -> ball counter decrement.
5. `test_target_hit_to_destroy`: Emit `TargetHit` => follow-up `TargetDestroyed` (if one-hit) and counter decrement.
6. `test_deterministic_sequence`: Script sequence of input + collision events produces stable journal (snapshot compare ignoring timestamps).
7. `test_remap_key`: Custom middleware remaps R→Pause; assert event change.

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Over-refactor causing regression | Lost demo behavior | Feature flag fallback; small phased PRs |
| Latency from added indirection | Input responsiveness | Keep event path minimal; measure frame time |
| Excess event spam (collision floods) | Queue bloat / perf | Debounce/cooldown or domain-specific filtering |
| Journal overlay performance | Minor UI cost | Debug-only gating; small fixed buffer |
| Test flakiness (timing) | Undermines determinism claim | Use frame stepping & explicit scheduling, avoid real time |

## Open Questions (To Resolve Early)

1. Do we treat ball spawn as idempotent if key held (needs debounce) or only on key press? (Likely: `just_pressed` semantics reproduced via Debounce=0 frame gating.)
2. Should pause toggle consolidate into single `PauseGame` vs separate pause/resume events? (Current design uses both; may implement toggle handler.)
3. Do hazards and targets already have component markers we can leverage, or add lightweight markers for detection systems? (Survey world state first.)

## Definition of Done Checklist

- [ ] Feature flag & plugin insertion logic committed.
- [ ] Unified input system emitting events; legacy branches removed or gated.
- [ ] Spawn / reset / pause / clear / collision flows fully event-driven.
- [ ] Journal overlay functioning & toggleable (debug only).
- [ ] All acceptance criteria tests implemented & passing locally.
- [ ] Metrics table filled with actual numbers (baseline vs final) appended below.
- [ ] No clippy warnings; tests green in CI / local.
- [ ] Deviations section added if any criteria waived.

### Post-Execution Metrics (to fill)

| Metric | Baseline | Final | Delta | Notes |
|--------|----------|-------|-------|-------|
| Input branches | (TBD) | (TBD) | (TBD) | |
| Peak queue depth | n/a | (TBD) | — | Representative 60s run |
| Event types used | 0 | (TBD) | + | Distinct GameEvent variants |
| Lines removed (input logic) | (TBD) | (TBD) | (TBD) | via `git diff --stat` |

## Deviations (Populate if Needed)

_None yet._
