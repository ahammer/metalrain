# Sub‑Sprint: Update `physics_playground` to Use Compositor

## Goal

Integrate the multi‑layer compositing pipeline (`GameRenderingPlugin` + layer targets + compositor pass) into `physics_playground`, enabling full visual stack: Background, GameWorld (walls/paddles/bodies), Metaballs (optional overlay), Effects (future), Ui (HUD / stats).

## Current State

* `physics_playground` runs physics simulation with Rapier debug visuals and/or simple direct rendering.
* Compositor integration demonstrated in `compositor_test` demo: layer toggles, blend adjustments.
* Background & improved UI layers may still be under development in parallel sprints.

## Objectives

1. Add `GameRenderingPlugin` to the playground app pipeline.
2. Route existing physics body visuals to `GameWorld` layer (`RenderLayers` tagging).
3. Add optional metaball overlay (runtime toggle) for hybrid rendering experiments.
4. Provide on‑screen HUD (layer status + physics stats) via `Ui` layer.
5. Layer toggle & blend hotkeys (mirror `compositor_test`).
6. Preserve physics determinism (render pipeline must not modify simulation ordering semantics).

## In Scope

* Code changes only to `demos/physics_playground`.
* Minimal UI (text HUD) referencing counts: bodies, contacts, FPS.
* Key bindings for toggling layers & enabling metaballs overlay.

## Out of Scope

* Performance benchmarking automation.
* Advanced rendering features (particle effects, trails).
* Full UI theming.

## Architecture & Design

Add plugins in `main.rs` (order: after default Bevy plugins, before or after physics as long as systems schedule clean). Example ordering conceptually:

```rust
App::new()
  .add_plugins(DefaultPlugins)
  .add_plugins(GameRenderingPlugin) // creates layer targets
  .add_plugins(GamePhysicsPlugin)
  .add_plugins(UiCompositorPlugin) // when available
  .add_systems(Startup, setup_playground)
  .add_systems(Update, (layer_input_system, blend_input_system, metaball_toggle_system, hud_update_system));
```

### Data / Resources

* `LayerToggleState`, `LayerBlendState` – mirrored from compositor test or extracted to shared module.
* `PhysicsStats` resource collected each frame (entity/body count, maybe broadphase pair count) -> HUD.
* `PlaygroundConfig` (optional): holds flags like `enable_metaballs_on_start`.

### Systems

1. `setup_playground` – inserts initial `LayerToggleState` (all on), spawns world entities, ensures they have `RenderLayers(GameWorld)`.
2. `apply_gameworld_layer` – fallback system to tag any physics body missing layer.
3. `metaball_toggle_system` – on key (e.g., `M`) add/remove `MetaballRendererPlugin` or spawn sample metaball field entity.
4. `layer_input_system` – keys 1–5 toggle layers; updates `LayerToggleState`.
5. `blend_input_system` – cycles specific layer blend modes (e.g., metaballs) via key (e.g., `B` or `L`).
6. `collect_physics_stats` – aggregates counts each frame / at interval.
7. `hud_update_system` – updates Ui text when stats or layer state changes.

### Determinism Considerations

* Rendering systems should run after physics step; ensure schedule ordering (e.g., physics in `PostUpdate`, rendering in built‑in render schedule). Avoid mutating physics components in render systems.

## Tasks

1. Add dependency in `demos/physics_playground/Cargo.toml`: `game_rendering`, (and `metaball_renderer` if overlay used), `widget_renderer` / `ui` module.
2. Insert `GameRenderingPlugin` into app builder.
3. Introduce helper for `RenderLayer::GameWorld` mapping if not already public.
4. Tag existing entities (paddles, walls) with GameWorld layer.
5. Implement layer toggle system (reuse code or extract from `compositor_test`).
6. Implement blend mode cycle system (targeting metaballs layer).
7. Implement metaball overlay spawn/despawn on key press (spawn sample config of a few metaballs for effect).
8. Add HUD text entity (if Ui plugin available) showing: Layers state summary, Body count, FPS.
9. Provide README for `physics_playground` explaining compositor integration + controls.
10. Manual test matrix: toggle each layer individually; verify expected visuals, no panic.
11. Manual determinism check: log position of a selected body for N frames with compositor on/off (should match).
12. Optional feature flag `playground-compositor` to disable for comparison (document if added).

## Acceptance Criteria

* Running `cargo run -p physics_playground` shows composited output (final fullscreen quad) not direct sprite pass.
* Keys 1–5 toggle layers; HUD (if present) updates accordingly.
* Metaball overlay appears/disappears with key press without panic; other layers unaffected.
* Disabling all layers results in empty (clear) output rather than crash.
* Window resize updates render targets (no stretched artifacts) and continues simulation.
* Physics object positions deterministic compared between compositor enabled/disabled runs (manual verification acceptable this sprint).

## Edge Cases

* Metaballs plugin absent -> layer texture None; compositor must handle gracefully.
* Background crate not yet implemented -> background layer blank but functional.
* Ui plugin absent -> still runs; HUD missing; no panics.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Layer toggle logic diverges between demos | Extract shared helper module for input mapping. |
| Compositor ordering breaks physics timing | Confirm schedule order; render shouldn't mutate physics state. |
| Increased GPU cost impacts test iteration | Allow disabling high‑cost layers quickly (metaballs off by default?). |

## Definition of Done

`physics_playground` fully driven by compositor pipeline; layer toggles & overlays operational; README updated; no regressions to physics behavior.

## Follow‑Ups

* Automated determinism test harness (capture & diff positions).
* Integration of new background & enhanced UI once those sprints land.
* Performance metrics overlay (frame timings per layer).
