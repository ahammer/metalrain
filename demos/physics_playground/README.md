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

## Controls (Compositor Mode)

### Spawning & World Editing

* LMB: Spawn a ball at cursor (or via nearest active spawn point when Shift held)
* RMB (two clicks): Define start/end to create a wall segment
* MMB: Spawn a target sensor
* H: Spawn a hazard (pit) rectangle
* C: Clear walls / targets / hazards
* S: Create spawn point at cursor
* Shift+1..9: Select spawn point index
* Q / E (with spawn point selection mode): Cycle active spawn point (unchanged from legacy)
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

### Compositor / Rendering

* 1..5: Toggle Background / GameWorld / Metaballs / Effects / Ui layers
* M: Toggle Metaballs layer (alias of key 3)
* Q / W / E: Set Metaballs blend mode (Normal / Additive / Multiply)

### HUD

* Displays body count, FPS, layer enable states, and current metaball blend.

### Spawn Point Selection vs Layer Toggles

* To avoid conflicts, digit keys for spawn point selection now require holding Shift.

## Metaball Overlay

The metaball renderer presents its composited field into the Metaballs layer via `.with_presentation_layer(RenderLayer::Metaballs.order() as u8)`. Disabling the Metaballs layer (3 or M) removes the overlay while keeping physics running.

## Determinism Considerations

The compositor systems only read layer & camera resources; they do not mutate physics state. Physics determinism (for identical input sequences) should remain intact whether layers are toggled or not. Manual verification procedure:

1. Run with compositor enabled, capture positions (e.g. log one marked ball for 300 frames with a fixed RNG seed).
2. Run again with identical interactions and layers differently toggled.
3. Compare logged coordinates (they should match within floating point tolerance).

## Manual Test Matrix

* Toggle each layer individually (1..5) – no panic, framebuffer updates.
* Disable all layers – final output is clear/empty (black) but app continues.
* Toggle Metaballs rapidly while spawning balls – no crashes, HUD updates.
* Change metaball blend Q/W/E – visible difference in composite.
* Resize window – quad & render targets resize (no stretching artifact).
* Gravity adjustments still work; layer toggles do not interfere with physics.

## Legacy Mode

If run with `--features no-compositor`, the app:

* Spawns a single `Camera2d` instead of layered cameras + compositor.
* Uses a simplified HUD string indicating legacy mode.
* Keeps prior controls (digits 1..9 no longer masked by layer logic unless Shift not held).

## Future Enhancements

* Add BackgroundRenderer integration once background sprint lands.
* Introduce Effects layer examples (pulse, particles).
* Determinism automated harness (record & diff frame states).
* Optional per-layer performance overlay.

---

Enjoy experimenting with the composited physics sandbox!
