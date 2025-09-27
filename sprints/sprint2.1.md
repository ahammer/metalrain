# Sprint 2.1: Metaball Renderer Decoupled Camera & Coordinate Pipeline

## Sprint Goal

Refactor the metaball rendering system to remove its internally owned camera and introduce a unified, downstream camera and compositing pathway: 3D World Space → (projection) → 2D Metaball Texture Space (offscreen, low‑res) → (composition) → Screen. Establish explicit coordinate transforms and prepare the renderer for integration into the multilayer pipeline (precursor to Sprint 3 rendering orchestrator).

## Rationale

Currently the metaball renderer spawns/owns a camera, coupling it to presentation. This limits:

- Combining metaballs with other 2D/3D scene content
- Independent camera movement / effects (shake, zoom, letterboxing)
- Consistent world → screen picking & interaction
- Layered compositing (metaballs should be a texture input, not a full scene)

Decoupling yields clearer responsibilities, stable low‑res performance, and futureproofing for additional layers (effects, UI, background) defined in Sprint 3.

## Target Architecture

```text
           World Entities (physics, gameplay)
                     ↓ (Transforms)
                 [Main Game Camera]
                     ↓ world_to_viewport (Bevy)
   ┌───────────────────────────────────────────────┐
   │            MetaballCoordinateMapper           │
   │  world (XY plane subset) ↔ metaball texture   │
   └───────────────────────────────────────────────┘
                     ↓ world_to_metaball
         Offscreen Metaball Texture (e.g. 512×512)
                     ↓ sampled as material/quad
             Compositing / Presentation Pass
                     ↓
                Screen Output
```

## Definitions & Spaces

- World Space: Gameplay coordinates (currently ~[-256,256]² for demos) on Z=0 plane.
- Metaball Texture Space: Discrete pixel grid (e.g. 512×512) storing blended field / shaded result.
- UV Space: Normalized [0,1]² mapping of metaball texture for sampling.
- Screen Space: Post‑camera pixel coordinates (after letterboxing/viewport scaling if any).

## Deliverables

1. Remove internal metaball camera & presentation logic.
2. Introduce `MetaballCoordinateMapper` resource defining world bounds ↔ texture mapping.
3. Adapt `MetaBall` component semantics (authoritative world position vs. derived texture coords).
4. Add systems to translate world positions into the renderer’s GPU buffer each frame.
5. Provide utility functions for projections:

     - world → metaball texture (Vec3→Vec2)
     - metaball texture → UV
     - world → screen (via main camera)
     - screen → world (picking support; plane intersection)

6. Expose new plugin configuration: `MetaballRendererPlugin { world_bounds, texture_size, ... }`.
7. Offscreen target only (no direct present); final blit/composition done externally.
8. Migration of existing demos (`metaballs_test`, `gameboard_test`) to new API.
9. Documentation + tests for coordinate correctness & parity with old visuals.

## Task Breakdown

### A. Decompose Existing Renderer

- [ ] Identify camera spawn & projection update systems in `metaball_renderer` and remove them.
- [ ] Replace any direct `Camera2d` dependencies with offscreen target handles.

### B. Coordinate Mapping Module

- [ ] Create `coordinates.rs` (or `src/coordinates/mod.rs`).
- [ ] Implement `MetaballCoordinateMapper { texture_size: UVec2, world_bounds: Rect }`.
- [ ] Functions:
  - `world_to_metaball(world: Vec3) -> Vec2`
  - `metaball_to_uv(tex: Vec2) -> Vec2`
  - `world_radius_to_tex(radius_world: f32) -> f32`

### C. Component & Buffer Adjustments

- [ ] Redefine `MetaBall` to store `world_position: Vec3` (or rely on `Transform`) and `radius_world`.
- [ ] Introduce an internal transient struct when encoding GPU buffer with texture‑space values.
- [ ] Frame system: query balls → map coords → write buffer (preserving clustering logic).

### D. Public Plugin API Changes

- [ ] Extend `MetaballRenderSettings` (or new config) with `world_bounds: Rect`.
- [ ] Provide sensible default (e.g. `Rect::from_corners(vec2(-256.,-256.), vec2(256.,256.))`).
- [ ] Add builder `.with_bounds(rect)` or modify existing `with()` constructor.

### E. Offscreen Target Handling

- [ ] Ensure plugin creates (or accepts) an `Image` render target of `texture_size`.
- [ ] Remove `present: bool` path—replace with `present_via_quad: bool` (optional) or drop entirely for Sprint 2.1.

### F. Integration Utilities

- [ ] Helper: `project_world_to_screen(world: Vec3, camera, transform) -> Option<Vec2>`.
- [ ] Helper: `screen_to_world(screen: Vec2, camera, transform) -> Option<Vec3>` (plane Z=0).
- [ ] Helper: `screen_to_metaball_uv(screen: Vec2, camera, transform, mapper) -> Option<Vec2>`.

### G. Demo Migration

- [ ] Update `gameboard_test` & `metaballs_test`: stop calling `world_to_tex`; remove local mapping funcs.
- [ ] Spawn a single main camera (outside metaball renderer) in demos.
- [ ] Add debug overlay to verify coordinate transforms (optional: draw crosshair at mouse world & metaball coords).

### H. Testing & Validation

- [ ] Unit tests: mapping edges (corners of world_bounds map to (0,0)/(W,H)).
- [ ] Property check: `metaball_to_uv(world_to_metaball(p))` remains in [0,1] for any p within bounds.
- [ ] Visual regression: compare field center & spread vs pre‑refactor (tolerance of sub‑pixel).
- [ ] Performance sanity: ensure no frame time increase > ~0.2 ms from extra mapping step.

### I. Documentation

- [ ] Update crate README with new architecture diagram & API.
- [ ] Add migration notes (old `MetaBall.center` now derived; show code diff snippet).

## Data & API Changes

Old:

```rust
MetaBall { center: Vec2, radius: f32 }
```

New (option 1 – world authoritative):

```rust
MetaBall { radius_world: f32 } // position from Transform
```

New (option 2 – explicit):

```rust
MetaBall { world_position: Vec3, radius_world: f32 }
```

Chosen: Option 1 (lean on entity `Transform` for position) to reduce duplication & sync costs.

## Coordinate Conversion (Reference Implementation)

```rust
pub struct MetaballCoordinateMapper {
    pub texture_size: UVec2,
    pub world_min: Vec2,
    pub world_max: Vec2,
}

impl MetaballCoordinateMapper {
    pub fn world_to_metaball(&self, world: Vec3) -> Vec2 {
        let p = world.truncate();
        let size = self.world_max - self.world_min;
        let norm = (p - self.world_min) / size; // 0..1
        norm * self.texture_size.as_vec2()
    }
    pub fn world_radius_to_tex(&self, r: f32) -> f32 {
        r * (self.texture_size.x as f32) / (self.world_max.x - self.world_min.x)
    }
    pub fn metaball_to_uv(&self, tex: Vec2) -> Vec2 {
        tex / self.texture_size.as_vec2()
    }
}
```

## Migration Steps (Concrete)

1. Extract & delete metaball camera spawn system.
2. Introduce mapper resource (insert during plugin build from `world_bounds`).
3. Replace all uses of `MetaBall.center` writes with deriving texture coords in the encoding system.
4. Update GPU buffer fill to call mapper each frame (remove per‑spawn mapping).
5. Adjust demos: remove `world_to_tex` helper, rely on `Transform`; spawn main camera separately.
6. Add temporary debug system to log a few mapped coordinates for sanity.
7. Run visual comparison; tune radii scaling if discrepancy observed.
8. Write README migration notes; remove debug system.

## Acceptance Criteria / Success Metrics

- ✅ Metaball renderer compiles & runs without spawning its own camera.
- ✅ Demos display metaballs identically (no major positional drift or scale difference).
- ✅ API surface simplified (no `present` flag if deprecated, or repurposed cleanly).
- ✅ Coordinate mapping unit tests pass.
- ✅ Performance overhead < 1% frame time vs baseline.
- ✅ External camera transforms (zoom / translation) reposition metaballs correctly on screen.

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Off-by-one or scaling errors | Visual misalignment | Unit tests on corners & center; side-by-side screenshot diff |
| API churn for downstream code | Refactor cost | Provide migration section & type alias (deprecated) if needed |
| Performance regression in mapping | Higher CPU cost | Batched iteration; avoid per-ball allocations |
| Z-order / compositing artifacts | Incorrect layer blend | Reserve upcoming compositing integration in Sprint 3 |
| Radius scaling mismatch | Visual size change | Compare sample ball world radius vs old texture radius mapping |

## Out of Scope (Deferred to Sprint 3+)

- Multi-layer compositor integration & blend modes
- Camera shake / letterboxing
- Dynamic resolution scaling for metaball texture
- Post-processing effects (bloom/glow refinement)

## Follow-Up / Hand-off to Sprint 3

Results here enable:

- Treating metaballs as a pure texture input in orchestrated pipeline
- Unified camera effects (shake, zoom, aspect) without special cases
- Addition of further render layers (Background, Effects, UI) with consistent mapping

## Definition of Done Checklist

- [ ] Camera code removed from metaball renderer
- [ ] Mapper resource implemented & tested
- [ ] `MetaBall` component simplified (Transform-driven position)
- [ ] Offscreen target only (no direct present path) or clearly deprecated flag
- [ ] Updated demos running & visually verified
- [ ] Unit tests for mapping pass
- [ ] README / migration docs updated
- [ ] Performance benchmark recorded

## Migration Notes (Draft for README)

Old usage:

```rust
commands.spawn(MetaBall { center: world_to_tex(pos), radius });
```

New usage:

```rust
commands.spawn((Transform::from_translation(pos.extend(0.0)), MetaBall { radius_world: radius }));
// Mapping handled internally each frame.
```

If temporary backward compatibility is desired, keep a deprecated adapter:

```rust
#[deprecated(note = "Use Transform + MetaBall { radius_world }")]
pub struct LegacyMetaBall { pub center: Vec2, pub radius: f32 }
```

---
Prepared: Sprint 2.1 plan to execute the decoupling refactor preceding multi-layer orchestration.
