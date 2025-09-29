# Sub‑Sprint: Better Text Compositor

## Goal

Improve text / UI layer compositing so the UI (`Ui` layer) clearly shows layer status (names + enabled/disabled), basic stats, and supports dynamic text updates with minimal overdraw.

## Current State

* Layer toggling feedback is mostly via logs; on‑screen visibility minimal.
* `widget_renderer` (if present) or raw Bevy `TextBundle` usage not fully isolated to the `Ui` compositing layer.
* `RenderLayer::Ui` exists but underutilized.

## Objectives

1. Route all UI/text entities to `Ui` layer render target.
2. Add on‑screen layer list (1–5) with name + ON/OFF state, auto‑updating when toggled.
3. Provide scoreboard / debug metrics example (e.g., FPS, body count, metaball count).
4. Implement optional dirty detection to disable Ui layer in compositor when no UI entities exist.

## In Scope

* Simple text overlays using Bevy built‑in text.
* `UiRenderConfig` resource (e.g., pixel snap bool, scale factor).
* Key binding parity with `compositor_test` (keys 1–5 already toggle layers; overlay reflects status).

## Out of Scope

* Custom glyph atlas management beyond Bevy default.
* Rich layout / flex UI.
* Text animations or transitions.
* SDF / MSDF font pipeline.

## Architecture

Stage 1 (this sprint) sticks to built‑in `TextBundle`:

* One dedicated `UiCamera` (2D) that renders only `Ui` layer.
* Systems maintain a `LayerHudState` (Vec of structs { index, name, enabled }).
* A single entity with a multi‑section `Text` displays the HUD; updated only when state changes.
* Dirty detection system counts `Text` + `RenderLayers(Ui)` entities; if zero set `LayerToggleState.ui_enabled = false` (or mark for compositor to skip).

### Data Structures

```rust
pub struct UiRenderConfig { pub pixel_snap: bool, pub scale: f32 }
pub struct LayerHudNeedsRebuild; // event or marker
pub struct LayerHudState(pub Vec<LayerHudEntry>);
pub struct LayerHudEntry { pub index: u8, pub name: &'static str, pub enabled: bool }
```

### Systems

1. `collect_layer_state` – reads `LayerToggleState`, rebuilds `LayerHudState` if changed.
2. `update_hud_text` – if `LayerHudNeedsRebuild`, updates the HUD entity `Text` sections.
3. `dirty_ui_detector` – toggles compositor Ui layer enable flag when zero text entities.
4. `spawn_ui_camera_and_hud` – startup camera + initial HUD entity in top‑left (anchored).
5. (Optional) `pixel_snap_text` – rounds transforms if `pixel_snap` true.

## Tasks

1. Create / update `widget_renderer` (or new `ui_layer` module) to house plugin `UiCompositorPlugin`.
2. Implement data types & plugin registration order (after rendering targets, before compositor finalize is acceptable since it's its own layer texture render pass).
3. Spawn `UiCamera` with `RenderLayers::layer(Ui)` + appropriate clear (transparent).
4. Spawn HUD entity; initial text lists placeholders until first state collection.
5. Implement layer state collection & diff detection (store previous mask or hash of booleans).
6. Update existing demos (`compositor_test`) to add the plugin and remove any ad‑hoc text.
7. Add scoreboard / debug stats (systems update text sections; ensure they don't trigger full HUD rebuild unless changed).
8. Dirty layer detection + integration with compositor (skip sampling when disabled) – optional if existing API allows.
9. Add keybinding reference overlay (e.g., small legend) if space allows.
10. Document usage in `widget_renderer` README + link from this sprint file.

## Acceptance Criteria

* HUD shows numbered layers with correct names & ON/OFF, updating within a frame of toggle.
* Enabling/disabling a layer via keys instantly reflects in HUD.
* HUD text itself resides only on `Ui` layer; disabling `Ui` layer removes all overlay text.
* No panics on window resize; HUD remains anchored top‑left.
* (If implemented) Disabling all UI entities causes compositor to skip Ui layer fetch (observable via log once) within ~1 second.
* Score / FPS metrics update at least once per second without stutter.

## Edge Cases

* Rapid toggling (spam keys) does not cause text entity churn (only text section updates).
* Zero layers (if future dynamic removal) – HUD gracefully displays "No layers".
* Long layer names – truncated or wrapped cleanly (decide max width; initial approach: let overflow happen if rare).

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Over‑updating text every frame | Only rebuild on state change diff; metrics sections updated at fixed interval. |
| Layer index to name divergence | Centralize names in a constant array shared with compositor. |
| UI camera ordering conflicts | Use explicit render graph / schedule ordering or rely on dedicated layer target. |

## Definition of Done

HUD functional, dynamic, documented; demos use `UiCompositorPlugin`; layer toggles visibly reflected; no regressions to other layers.

## Follow‑Ups

* Custom text batching / atlas metrics.
* SDF/MSDF high quality fonts.
* Animated transitions (fade in/out, slide).
* Interactive debug panel widgets.
