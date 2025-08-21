# Debug Interface & Feature Flags Plan

Date: 2025-08-17
Scope: Implements audit items 7 & 8 (in‑game debug UI + feature flags) plus explicit user requirements for numbered view modes (1‑6) and AI/CLI friendly runtime logging.

## Objectives
1. Provide four keyboard‑selectable visualization / debug modes (reduced from original six):
  1. Metaball render (current shaded metaball view)
  2. Rapier debug wireframe view (colliders, joints) with optional circle meshes
  3. Metaball heightfield (grayscale scalar field visualization)
  4. Metaball color information (debug: per-cluster color index / nearest ball color boundaries)
2. Supply both on‑screen GUI + textual (CLI/log) diagnostics: FPS, ball count, cluster count, active mode, truncated metaball packing stats.
3. Add compile‑time feature flags so that in release (or when `--no-default-features`) the entire debug UI & mode switching are compiled out (keystrokes no‑op, zero runtime cost).
4. Provide AI‑agent friendly, minimally verbose, structured runtime logs summarizing simulation state periodically (≤1 line/sec default) & important state transitions (mode changes).
5. Keep performance overhead minimal; disabled code should have near‑zero branching impact.

## Feature Flag Strategy

Cargo.toml additions:
```
[features]
default = []                    # Core gameplay always on; only optional part is debug tooling
debug = ["dep:bevy_egui"]      # pulls egui only when desired (optional)
```
Notes:
- Only a single feature flag `debug` is introduced. All gameplay systems (metaballs, radial gravity, clustering, etc.) remain unconditional and always compiled.
- `debug` gates: debug plugin(s), egui dependency, rapier debug toggle logic, text overlay, shader variant uniform for metaball view switching.
- When `debug` feature is not enabled at compile time: no debug resources, no systems, key presses 1‑6 do nothing (systems behind `#[cfg(feature="debug")]`).

## High-Level Architecture

New module layout:
```
src/
  debug/
    mod.rs (DebugPlugin entry)
    modes.rs (DebugRenderMode enum & helpers)
    overlay.rs (text & egui panels)
    logging.rs (periodic structured log emission)
    keys.rs (key handling + mode switching logic)
    stats.rs (frame stats collection / smoothing)
```

Existing systems interplay:
- `MetaballsToggle` (resource) currently controls visibility. Mode switching will modify:
  * `MetaballsToggle.0` (on for modes 1,5,6; off otherwise)
  * A new `MetaballsDebugView` enum uniform value (Normal, Heightfield, ColorInfo) passed into shader.
- `GameConfig` flags (`draw_circles`, `draw_cluster_bounds`, `rapier_debug`): we will *not* mutate the config itself (treat as baseline). Instead introduce a `DebugVisualState` resource layering runtime overrides on top of config. Rendering / gizmo systems will consult override first.
- Rapier debug plugin currently added in `main` if `cfg.rapier_debug`. We add ability to spawn (and despawn) this plugin dynamically when switching to/from mode 4 under debug feature. (Bevy supports adding plugins mid‑run; removal is trickier – we will instead enable/disable a resource `RapierDebugOverride(bool)` that the plugin respects by setting `RapierDebugRenderPipeline::enabled` component or using `ResMut<RapierDebugRenderContext>` if available in 0.31; fallback: always compile plugin when either config OR debug feature is active and hide its render layer when not in mode 4.)

## Enumerations & Resources

```
#[cfg(feature="debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugRenderMode {
  Metaballs,           // 1
  RapierWireframe,     // 2
  MetaballHeightfield, // 3
  MetaballColorInfo,   // 4
}

#[cfg(feature="debug")]
#[derive(Resource)]
pub struct DebugState {
  pub mode: DebugRenderMode,
  pub last_mode: DebugRenderMode,
  pub overlay_visible: bool,          // toggle via F1 maybe
  pub log_interval: f32,              // seconds (default 1.0)
  pub time_accum: f32,                // internal
  pub frame_counter: u64,
}

#[cfg(feature="debug")]
#[derive(Resource, Default)]
pub struct DebugStats {
  pub fps: f32,              // smoothed
  pub frame_time_ms: f32,    // smoothed
  pub ball_count: usize,
  pub cluster_count: usize,
  pub truncated_balls: bool, // true if > MAX_BALLS encoded
}

#[cfg(feature="debug")]
#[derive(Resource)]
pub struct DebugVisualOverrides {
  pub draw_circles: Option<bool>,
  pub draw_cluster_bounds: Option<bool>,
  pub rapier_debug_enabled: Option<bool>,
  pub metaballs_enabled: Option<bool>,
  pub metaballs_view_variant: MetaballsViewVariant,
}

pub enum MetaballsViewVariant { Normal, Heightfield, ColorInfo }
```

Initialization:
- On Startup, `DebugState.mode = Metaballs`.
- `DebugVisualOverrides` is recalculated each frame or upon mode switch (cheap) – ensures correctness even if baseline config changed by hot reload in future.

## Mode Mapping Table

| Mode | MetaballsToggle | Circles | Rapier Debug | Metaballs Variant |
|------|-----------------|---------|--------------|-------------------|
| 1 Metaballs | true | false (unless config requested) | false | Normal |
| 2 Rapier Wireframe | false | true (optional) | true | Normal (ignored) |
| 3 Heightfield | true | false | false | Heightfield |
| 4 Color Info | true | false | false | ColorInfo |

Notes:
- If base config had `draw_circles=true` while in mode 1 (metaballs) we *override* to false to avoid duplicated visuals.
- Heightfield & ColorInfo require shader uniform switch (single shader; branch in WGSL guarded by small `switch` / `if`).

## Shader Changes (Metaballs)

Add uniform field `debug_view: u32` (0=Normal,1=Heightfield,2=ColorInfo). In fragment shader:
- Heightfield: output grayscale based on field value (optionally encode iso difference with subtle outline).
- ColorInfo: output cluster color index (e.g., map cluster index to palette color but desaturate / overlay cluster id text unreachable in shader; textual ID will appear in overlay list instead).

## Input Handling (Updated for 4 Modes)

`keys.rs` system runs early in `Update` (before rendering systems). Pseudocode:
```
if just_pressed(Digit1) { set_mode(Metaballs) }
if just_pressed(Digit2) { set_mode(RapierWireframe) }
if just_pressed(Digit3) { set_mode(MetaballHeightfield) }
if just_pressed(Digit4) { set_mode(MetaballColorInfo) }
if just_pressed(F1) { state.overlay_visible = !state.overlay_visible }
```
`set_mode` updates `DebugState`, recomputes overrides, logs mode change (`info!(event="mode_change", from=?, to=?, frame=? ...)`).

## Overlay (GUI + Non‑GUI)

Two layers:
1. Minimal Text (always when `debug` on & `overlay_visible`): Use Bevy UI `TextBundle` anchored top‑left. Columns: `FPS`, `FrameTime`, `Balls (encoded/MAX)`, `Clusters`, `Mode`, `MetaballsVariant`, `Truncated?`.
2. Egui panel (only if `bevy_egui` present— pulled by `debug` feature) with:
   - Radio buttons for modes 1‑6 (mirrors keys)
   - Slider for metaballs iso / metallic / roughness (reusing existing resources) with dirty flag
   - Toggle checkboxes for cluster bounds, rapier debug, circles (explicit user override) – writing into `DebugVisualOverrides` Option<bool>
   - Collapsible “Logs” area with last N structured log lines (ring buffer of fixed size ~128 entries) for quick review.

When debug feature absent: no UI elements inserted at all.

## Stats Collection

- Add `FrameTimeDiagnosticsPlugin` (behind debug feature) for FPS / frame time; if not available (e.g., wasm minimal), compute manually via exponential smoothing: `fps = 0.9*prev + 0.1*(1/delta)`.
- Ball & cluster counts: simple per-frame queries. (Ball count = query length; cluster count = `Clusters.0.len()`).
- Truncation detection: compare `ball_count` vs `metaballs::MAX_BALLS` when in a metaball mode.

## Structured Logging (AI Friendly)

Logging goals:
- Provide one concise periodic summary line (default each second) with machine‑parseable key=value pairs and human readability.
- Provide immediate lines for significant events: mode changes, large ball count changes (>10% delta), cluster count delta > 20% or cluster formation/merge spike.

Format examples:
```
SIM frame=1234 t=12.345s fps=59.8 ft_ms=16.7 balls=150 clusters=12 mode=Metaballs variant=Normal encoded=150/1024 trunc=false
MODE_CHANGE from=RapierWireframe to=Metaballs frame=1240 balls=150 clusters=12
ALERT cluster_spike old=12 new=25 frame=2000
```
Parsing considerations:
- All tokens space-separated; fields contain only alnum/underscore/period.
- Prefix tokens (`SIM`, `MODE_CHANGE`, etc.) disambiguate line types.

Implementation: `logging.rs` system accumulates `time_accum` and when ≥ `log_interval` emits one SIM line then resets. Stores previous stats for change detection.

## Interactions With Existing Config

Baseline config values remain authoritative when NOT in debug feature or when an override Option is None. Algorithm to compute effective visuals each frame:
```
effective.draw_circles = override.draw_circles.unwrap_or(cfg.draw_circles)
// then mode specificity tweaks (e.g. if mode uses metaballs, force circles false)
```
`debug_draw_clusters` system will check new effective state resource instead of `cfg.draw_cluster_bounds` directly (add small adaptation layer to avoid scattering logic changes).

## System Ordering

Add a new system set `DebugPreRenderSet` (after physics & cluster computation, before metaballs update) for:
- key handling (mode switches)
- stats collection (needs up-to-date counts)
- visual override application (so subsequent rendering systems read the updated toggles)

Order:
```
Update:
  PrePhysicsSet
  (physics / separation / cluster)
  DebugPreRenderSet
  Metaballs update & other rendering prep
```
This ensures cluster counts are current when logged.

## Performance Considerations

- All debug systems behind `#[cfg(feature="debug")]`.
- Mode switching queries are O(1) (just resource mutation) except counts which are O(N) (ball iteration). Additional per-frame cost acceptable; can throttle counts to every N frames if needed (not initially necessary).
- Egui only active when feature present; heightfield & color info shader branches minor cost (uniform int based branch), negligible versus field accumulation.
- Rapier debug always loaded if either config enables it or debug feature active. When not in mode 4, set its render layer visibility off (if supported) or set `enabled` flag inside its resource (need to inspect rapier debug plugin API at implementation time).

## Failure / Edge Cases

- (Metaballs are always available; no unavailable mode scenario.)
- If shader uniform update fails (missing material entity) we log one warning per minute (rate limited) not every frame.
- When ball count > `MAX_BALLS`, truncated flag set; overlay shows `balls: 1500 (1024 encoded)`; structured log includes `encoded=1024/1500 trunc=true`.

## Incremental Implementation Steps
1. Add feature flags to Cargo.toml (`features` section) & optional `bevy_egui` dependency (only with `debug`).
2. Create `src/debug` module with `mod.rs` exposing `DebugPlugin` behind `cfg(feature="debug")` plus public re-export stub when not enabled (empty struct doing nothing) so `GamePlugin` can always add it conditionally on compile features.
3. Define enums/resources (`modes.rs`).
4. Implement key handling (`keys.rs`).
5. Implement stats collection (`stats.rs`).
6. Implement logging system (`logging.rs`).
7. Implement overlay text UI (always for debug builds) (`overlay.rs`).
8. Integrate egui panel (guards with `#[cfg(feature="debug")]` + runtime detection maybe optional).
9. Extend metaballs shader uniform & code for debug variants (add variant, branch logic) & update `update_metaballs_material` to write new field.
10. Add resource translation layer `DebugVisualOverrides` -> mutate `MetaballsToggle` and control cluster/rapier drawing (might involve modifying existing systems to consult overrides).
11. Update `debug_draw_clusters` to use overrides resource.
12. Add periodic structured log tests (unit test verifying format regex for SIM & MODE_CHANGE lines) using headless `App` with forced time progression.
13. Document keys & modes in `README.md` & finalize `debug_interface_plan.md` (this file) referencing commit.
14. Provide fallback no-op definitions under `#[cfg(not(feature="debug"))]` to keep compilation simple when feature disabled.

## AI Agent Considerations

- Deterministic log ordering: only one SIM line per interval; events emitted immediately after state change to preserve causal narrative.
- Minimal abbreviations; consistent field names; numeric values fixed decimal precision where helpful (fps one decimal, frame time one decimal, positions not logged to reduce noise).
- Logs explicitly state when a mode cannot be activated and why.
- Future extensibility: Could add JSON side-channel logs; current design keeps simple parseable key=value format.

## Open Questions / Deferred Decisions
- Dynamic removal of Rapier debug plugin is non-trivial; initial approach keeps it loaded & toggles visibility.
- Heightfield & ColorInfo exact visual style: implement first-pass minimal (grayscale / palette) and refine later.
- Whether to unify CLI & overlay text generation to avoid duplication (possible future refactor: generate a single `DebugStatsSnapshot` that both overlay & logger format differently).

## Risks / Mitigations
- Shader changes may inadvertently break existing visual appearance: mitigate via keeping default path identical when `debug_view==0` (Normal) and adding tests for uniform defaults (serialize material resource).
- Feature flag misconfiguration (forgetting to add `debug` in dev). Provide build-time `println!("Debug feature disabled")` in no-op plugin stub when running with `RUST_LOG=info` to remind user.
- Input collision with existing metaball param hotkeys (`[` `]` `M` `-` `=` `E` `P`) – documented; no overlap with 1-6.

## Acceptance Criteria
- Pressing keys 1‑6 in a debug build switches modes and updates overlay & logs accordingly.
- In a non‑debug build, pressing 1‑6 produces no visual/log change (and no performance regression).
- Overlay shows FPS within ±5% of Bevy diagnostics plugin.
- Structured log lines appear once per second & on every mode change.
- Heightfield & color info modes visibly distinct from normal metaball render.

---
End of plan.
