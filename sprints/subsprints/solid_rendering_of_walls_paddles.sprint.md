# Sub‑Sprint: Solid Rendering of Walls & Paddles

## Goal

Render walls and paddles as solid, palette‑driven geometry on the `GameWorld` layer (beneath metaballs, above background) instead of relying solely on physics debug visuals.

## Current State

* Rapier debug rendering shows colliders (lines / wireframes) for paddles & walls.
* No dedicated solid sprite/mesh representation in composited pipeline.
* `GameWorld` layer exists; may not yet host these entities with proper `RenderLayers` tagging.

## Objectives

1. Introduce visual components (sprites or simple quads) for paddles and arena walls.
2. Centralize color palette via `WorldPalette` resource.
3. Ensure entities render on `GameWorld` layer and composite correctly with metaballs (metaballs additive over them).
4. Maintain alignment between physics colliders and visual bounds.

## In Scope

* Axis‑aligned rectangular quads sized to collider shapes.
* Color only (no textures / lighting yet).
* Optional debug toggle to hide walls/paddles for profiling.

## Out of Scope

* Texturing, normal maps, lighting, animations.
* Particle/impact effects (future Effects layer work).

## Architecture & Design

Simplest implementation uses `SpriteBundle` (2D) or `MaterialMesh2dBundle` with a rectangle mesh:

* `WorldPalette` holds distinct colors: wall, paddle, hazard.
* A spawn system `apply_gameworld_layer` tags relevant entities with `RenderLayers::layer(GameWorld)`.
* Paddle & wall spawn code sets size via transform scale or mesh dimensions to match physics collider extents.

```rust
pub struct WorldPalette {
    pub wall: LinearRgba,
    pub paddle: LinearRgba,
    pub hazard: LinearRgba,
}
impl Default for WorldPalette { /* choose contrasting accessible colors */ }
```

If later advanced materials needed, can wrap into `SimpleGameMaterial`; skip now to reduce abstraction overhead.

### Ordering / Layering

* All wall/paddle sprites: `RenderLayers(GameWorld)`.
* Metaballs: `Metaballs` layer (already additive in compositor pass).
* Background: unaffected.

### Resize / Arena Changes

* If arena size resource changes (e.g., dynamic resizing), a system updates wall transforms.

## Tasks

1. Add (or update) `WorldPalette` definition in `game_core` (or appropriate core crate) with defaults.
2. Provide helper: `fn game_world_layer() -> RenderLayers` centralizing index mapping.
3. Modify paddle spawn to include `SpriteBundle` (size: collider extents) and color = palette.paddle.
4. Add walls: four `SpriteBundle` entities sized to arena boundaries (derived from `ArenaDimensions` or existing config).
5. System `sync_wall_transforms` (optional) to adjust if arena changes at runtime.
6. Add keybinding (e.g., `H`) to toggle visibility (set `Visibility::Hidden`) for walls & paddles for quick profiling.
7. Validate visual alignment: log warning if |visual_size - collider_size| > epsilon.
8. Update `physics_playground` to show new visuals (will integrate with compositor in its own sprint).
9. Document palette and layering choices in this file & optionally in `game_core` README.
10. (Optional) Add a simple test/system assert ensuring every paddle entity has either a collider + visual pair.

## Acceptance Criteria

* Paddles & walls appear as filled shapes with defined palette colors during gameplay.
* Disabling `GameWorld` layer (via compositor demo toggles) hides them entirely.
* Metaballs & background still visible / unaffected when GameWorld hidden.
* Visual quads align with physics colliders (no obvious offset or scaling mismatch to the eye; debug log may confirm).
* Changing `WorldPalette` at runtime (mutating resource) updates colors next frame.
* No new runtime warnings (except intentional mismatch warnings if triggered for test).

## Edge Cases

* Zero arena size / degenerate dimensions – walls gracefully shrink / hide (skip spawn or mark hidden).
* Multiple paddle entities (future multiplayer) all get correct layer tagging automatically.
* High DPI / window resize – visuals scale correctly or remain in world units independent of pixel density.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Collider & sprite drift over time | Single authoritative dimensions resource; update both via one system. |
| Premature material abstraction | Start with sprite colors; add material only when effects require it. |
| Layer index drift | Central helper function for layer mapping; used everywhere. |

## Definition of Done

Walls and paddles render solidly on GameWorld layer; palette applied; layer toggling works; alignment validated; documentation added.

## Follow‑Ups

* Textured or animated paddle skins.
* Impact flash / damage shaders.
* Trail effects on paddle movement (Effects layer).
* Hazard / power‑up zone visuals.
