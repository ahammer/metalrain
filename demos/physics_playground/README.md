# Physics Playground (Compositor Integrated)

This demo showcases the physics sandbox integrated with the multi-layer compositor pipeline (`GameRenderingPlugin`). It allows interactive spawning and manipulation of balls, walls, targets, hazards, paddles, and spawn points while routing visuals through named render layers:

| Layer | Purpose |
|-------|---------|
| Background | (Reserved – currently blank) |
| GameWorld | World geometry (walls, paddles, spawn points, future sprites) |
| Metaballs | Metaball field visualization of balls |
| Effects | (Reserved – future transient effects) |
| Ui | HUD overlay text |

## Run

```bash
cargo run -p physics_playground
```

A `--features no-compositor` run path is available for legacy (pre-compositor) behavior:

```bash
cargo run -p physics_playground --features no-compositor
```

## Controls

All compositor layers are always enabled in this playground; controls focus purely on spawning and physics interaction.

### Spawning & World Editing

* LMB: Spawn a ball at cursor (or via nearest active spawn point when Shift held)
* RMB (two clicks): Define start/end to create a wall segment
* MMB: Spawn a target sensor
* H: Spawn a hazard (pit) rectangle
* C: Clear walls / targets / hazards
* S: Create spawn point at cursor
* 1..9: Select spawn point index
* Q / E: Cycle active spawn point
* X: Toggle active flag on selected spawn point
* P: Spawn paddle at cursor
* A: Toggle auto-spawn policy

### Physics & Simulation

* Arrow Keys: Adjust gravity components
* G: Toggle gravity on/off (default off -> sets (0,-500))
* R: Reset (despawn) all balls
* T: Stress test fill (adds up to 60 balls)
* +/- : Adjust clustering strength
* [ / ] : Adjust clustering radius

### HUD

* Displays body count and FPS plus a condensed control reminder.

## Metaball Overlay

The metaball renderer presents its composited field into the Metaballs layer via `.with_presentation_layer(RenderLayer::Metaballs.order() as u8)`. In this demo the layer is permanently enabled (no toggle keys) so you can concentrate on physics editing.

## Determinism Considerations

The compositor systems only read layer & camera resources; they do not mutate physics state. Physics determinism (for identical input sequences) should remain intact whether layers are toggled or not. Manual verification procedure:

1. Run with compositor enabled, capture positions (e.g. log one marked ball for 300 frames with a fixed RNG seed).
2. Run again with identical interactions and layers differently toggled.
3. Compare logged coordinates (they should match within floating point tolerance).

## Manual Test Matrix

* Spawn, move, and clear world elements (walls, targets, hazards) – no panics.
* Spawn points selection (1..9) functions without interfering with other inputs.
* Stress test (T) maintains responsive HUD and rendering.
* Window resize preserves correct aspect and physics behavior.
* Gravity and clustering adjustments update HUD values correctly (where shown).

## Legacy Mode

If run with `--features no-compositor`, the app still works identically, just without the layered render pipeline (direct camera rendering).

## Future Enhancements

* Add BackgroundRenderer integration once background sprint lands.
* Introduce Effects layer examples (pulse, particles).
* Determinism automated harness (record & diff frame states).
* Optional per-layer performance overlay.
* (In Progress) Event Core integration behind `--features event_core_integration` enabling deterministic input→event→handler pipeline. Phase 1 adds plugin + raw key capture; later phases will migrate spawn/reset/pause and collision flows to events and provide a journal overlay.

### Event Core Migration Baseline (Recorded)

| Metric | Baseline |
|--------|----------|
| `keys.just_pressed` branches | 16 unique occurrences |
| Mouse button just_pressed branches | 3 (LMB, RMB, MMB) |
| Direct reset/despawn branches | 2 (R reset balls, C clear world) |

These numbers will be reduced by ≥50% after full migration (see `sprints/subsprints/UpgradePhysicsPlaygroundToEventCore.md`).

---

Enjoy experimenting with the composited physics sandbox!
