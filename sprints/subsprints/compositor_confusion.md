# Subsprint: Correct Metaball Layer Composition

## 1. Objective

Ensure metaball rendering output is composited exclusively via the Metaballs layer (layer 2) texture in the compositor pipeline, eliminating accidental direct rendering to the Background layer (layer 0).

## 2. Problem Statement

Metaballs currently appear on layer 0 (background) and the intended layer 2 render target remains empty. This breaks the compositing model (multi‑layer post process) and prevents correct ordering / blending.

## 3. Current Symptoms

1. Metaballs visible even if layer 2 sampling is disabled.
2. Layer 2 texture appears black / untouched.
3. Presentation quad appears to bypass offscreen path and draw straight to main target.

## 4. Architectural Context (Simplified)

- Layer system: Background (0), Mid / Other (1), Metaballs (2), UI (N).
- Expected flow: Simulation -> Offscreen metaball texture (compute + material) -> Presentation quad (RenderLayers::2) -> Compositor samples layer 2 texture -> Final screen.
- Actual: Presentation quad (or equivalent mesh/material) lives on RenderLayers::0.

## 5. Hypothesis / Root Cause

The presentation quad entity spawned by metaball renderer lacks: (a) stable identifying Name, (b) proper RenderLayers assignment, and/or (c) correct system ordering so reassignment never occurs. Secondary risk: offscreen target not bound because pipeline assumes layer-based filtering.

## 6. Investigation Tasks

1. Log all entities with `Name` + `RenderLayers` after startup.
2. Confirm existence (or absence) of entity named `MetaballPresentationQuad`.
3. Inspect components on that entity (Mesh2d, material, RenderLayers, visibility, target binding if applicable).
4. Verify compositor actually samples layer 2 texture handle (trace where texture is produced / inserted).
5. Confirm system ordering: metaball presentation spawn vs configuration pass.

## 7. Implementation Tasks

### 7.1 Add Naming

Add `Name::new("MetaballPresentationQuad")` to the spawn in `metaball_renderer/src/present/mod.rs`.

### 7.2 Layer Assignment Strategy

Option A (dynamic): Keep default spawn (layer 0) then a configuration system moves it to layer 2 once found.

Option B (preferred): Inject settings so it spawns directly with the correct `RenderLayers` component.

### 7.3 Extend Settings API

In metaball renderer settings struct add `presentation_layer: Option<u8>` plus builder `with_presentation_layer(u8)`; default None preserves current behavior.

### 7.4 Apply Layer on Spawn

If `presentation_layer` present, insert `RenderLayers::layer(presentation_layer)` immediately during presentation quad creation.

### 7.5 Fallback Reassignment System

Add a lightweight system (runs once, After spawn set) in compositor test demo to assert / repair layer if misconfigured.

### 7.6 Debug Instrumentation (Temporary)

Add trace logging in `configure_metaball_presentation`:

```rust
info!(?entity, ?name, ?layers = ?layers.bits());
```

Remove after verification.

## 8. System Ordering

Ensure `configure_metaball_presentation` runs After the metaball plugin's display spawn set (e.g. `.after(MetaballDisplaySet)`). If sets are not exposed, add an explicit `StartupSet` in the plugin and document it.

## 9. Verification & Acceptance Criteria

- [ ] Entity `MetaballPresentationQuad` exists.
- [ ] It has `RenderLayers` containing layer 2 only (or includes 2 if multi-layer needed).
- [ ] Layer 2 offscreen texture shows non‑black content (diagnostic: copy to debug quad / readback if needed).
- [ ] Disabling compositor sampling of layer 2 removes metaballs from final frame.
- [ ] Background layer (0) free of metaball visuals.
- [ ] Blend mode for metaball layer in compositor is Normal (no unintended premult bugs).
- [ ] No stray log warnings about missing quad.

## 10. Observability Enhancements (Optional but Recommended)

- Add a feature‑gated debug system to dump per-layer draw counts.
- Expose a CLI arg / env var to force override presentation layer for rapid experimentation.

## 11. Risks / Edge Cases

- System ordering regressions if plugin init changes.
- Multiple quads spawned (guard with `debug_assert!(found_once)`).
- Future refactor removing Name => rely on an explicit marker component `MetaballPresentation` to reduce string fragility.

## 12. Rollback Plan

If issues arise, disable new settings field (leave `presentation_layer = None`) and rely on reassignment system; remove instrumentation after stable.

## 13. Follow‑Up Improvements (Post‑Subsprint)

- Replace ad‑hoc RenderLayers with a declarative layer registry enum + helper macro.
- Add automated test that spawns app headless and asserts the layer assignment via world query.
- Snapshot test of layer 2 texture (compare hash within tolerance).

## 14. Implementation Snippets

Spawn (simplified):

```rust
commands.spawn((
    Mesh2d(quad_handle),
    MeshMaterial2d(material_handle),
    Name::new("MetaballPresentationQuad"),
    // Insert when configured:
    // RenderLayers::layer(settings.presentation_layer.unwrap())
));
```

Settings builder:

```rust
pub struct MetaballRenderSettings { /* ... */ pub presentation_layer: Option<u8>, }
impl MetaballRenderSettings {
    pub fn with_presentation_layer(mut self, layer: u8) -> Self { self.presentation_layer = Some(layer); self }
}
```

## 15. Definition of Done

All acceptance criteria in Section 9 satisfied, debug instrumentation removed or gated, and a brief note added to renderer README about `presentation_layer`.

---
Owner: (assign)  | Target Duration: < 1 day focus | Status: Planned
