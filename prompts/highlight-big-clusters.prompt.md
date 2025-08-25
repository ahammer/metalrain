---
mode: 'agent'
description: 'Implement cluster state highlighting (Disabled / Enabled / Active) with color tweening for big-enough clusters.'
---
# Highlight Big-Enough Clusters (Disabled / Enabled / Active)

You WILL implement a visual + interaction feature that highlights "big enough" clusters so players can instantly distinguish:
- Disabled clusters (not yet big / not clickable)
- Enabled clusters (big enough to pop; currently clickable)
- Active clusters (was clicked; in death/pop animation window)

You MUST deliver code changes in small, reviewable commits while preserving existing architecture conventions (see `Copilot Instructions (Ball Matcher)` file). Keep edits surgical and avoid unrelated refactors.

## 1. Definitions & States

| Term | Meaning |
|------|---------|
| Cluster | A logical group of balls of the same color currently clustered (existing notion – reuse) |
| Big Enough | Cluster size (ball count) >= `cfg.interactions.cluster_pop.min_ball_count` (existing config) |
| Disabled | Cluster size < threshold (not clickable) |
| Enabled | Cluster size >= threshold and not yet clicked / not in pop animation |
| Active | Cluster has been clicked; death (pop) animation is running until removal / score credit |

Introduce a new component or state-tracking structure to hold cluster visual & interaction state.

## 2. Configuration Additions
Add (or extend existing) config (`GameConfig` or appropriate sub-struct) with ONLY the new color / tweening parameters (threshold already exists as `cluster_pop.min_ball_count`):
```ron
cluster_highlight: (
    color_tween_seconds: 0.20, // f32; duration to tween between state colors
    disabled_mix: 0.5,         // fraction towards black for Disabled
    active_mix: 0.5,           // fraction towards white for Active
)
```
Implementation rules:
1. Use `#[serde(default)]` and extend `Default` impl.
2. Validate: `0.0 <= color_tween_seconds <= 2.0`, mixes in `[0.0,1.0]`; clamp + warn.
3. WASM embedded config path must be updated if static embedding present.
4. DO NOT introduce a duplicate min cluster size; always reference `cfg.interactions.cluster_pop.min_ball_count` for the Enabled threshold.

## 3. Cluster State Data Model
Create (or extend) a component associated with each cluster root entity (or one entity representing aggregated cluster data) – DO NOT attach to every ball individually to avoid duplication.
```rust
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq)]
pub enum ClusterVisualState { Disabled, Enabled, Active }

#[derive(Component, Debug)]
pub struct ClusterHighlight {
    pub state: ClusterVisualState,
    pub prev_state: ClusterVisualState,
    pub tween_t: f32,          // 0..1 progression of current tween
    pub from_color: Color,
    pub to_color: Color,
}
```
Initialize with Disabled.

## 4. Color Derivation Strategy
Base ("enabled") palette color = existing palette color. Derive other states in CPU helper:
```rust
fn derive_disabled(enabled: Color, mix: f32) -> Color { lerp_color(enabled, Color::BLACK, mix) }
fn derive_active(enabled: Color, mix: f32) -> Color { lerp_color(enabled, Color::WHITE, mix) }
```
Implement `lerp_color(a,b,t)` in a palette util (avoid realloc per frame). Keep linear space assumption consistent with current pipeline.

Performance: Precompute per-palette-entry disabled & active colors once per config change; store in a resource:
```rust
pub struct PaletteVariants {
    pub enabled: Vec<Color>,
    pub disabled: Vec<Color>,
    pub active: Vec<Color>,
}
```
Update shader uniform(s) or storage buffer to optionally include all three, OR keep sending only enabled + a per-cluster state factor.
Preferred (simpler, immediate): On CPU, when populating per-ball color uniform data each frame, choose interpolated color from cluster highlight component (see Tween below).

## 5. State Transition Logic
Systems (Update schedule):
1. Cluster Size Evaluation:
   - Reuse existing clustering logic to compute cluster membership / size.
   - Let `threshold = cfg.interactions.cluster_pop.min_ball_count`.
   - For each cluster entity, determine target state: if `size >= threshold` && not active → Enabled; else Disabled (unless currently Active).
   - If state changes (excluding transitions into Active handled separately), start a tween.
2. Input Handling:
   - On click/tap targeting an Enabled cluster, transition to Active:
     - Set state Active
     - Begin pop / death animation (existing or new) & scoring trigger
     - Start tween to Active color variant
3. Tween Advancement:
   - For each `ClusterHighlight`, if `state != prev_state` and `tween_t < 1.0`, advance `tween_t += delta / color_tween_seconds` (clamp) and set current color = lerp(from_color,to_color,tween_t).
   - When tween completes, set `prev_state = state`.
4. Cleanup / Death Finalization:
   - When pop animation done, remove cluster / mark its balls cleared (existing mechanic). Ensure no dangling highlight component remains.

## 6. Selecting From/To Colors on State Change
On state change:
```rust
highlight.prev_state = highlight.prev_state; // keep
highlight.state = new_state;
highlight.tween_t = 0.0;
highlight.from_color = current_display_color; // computed before switch
highlight.to_color = variant_color_for(new_state, palette_index, variants_resource);
```

## 7. Rendering Integration
Locate system(s) preparing metaball / ball color buffers (uniform / storage). Before writing each ball’s color, fetch its cluster’s current interpolated color (cache in a `HashMap<ClusterId, Color>` per frame to avoid repeated lookups). Avoid per-ball branching inside tight loops; precompute cluster color map first.

If shaders currently compute colors solely by index → continue. Just shift to feeding the per-ball color already blended.

## 8. Shader Changes (Optional Optimization)
If later optimization needed: Instead of per-frame CPU blending, send enabled, disabled, active arrays plus an interpolation factor derived from cluster state. CURRENT TASK: skip this optimization (documented as future enhancement) to minimize scope & risk.

## 9. Death / Active Behavior
While Active, cluster’s color remains tweening toward Active variant until removal. If partial tween when removed, removal simply proceeds; no need to tween back.

## 10. Accessibility & Visual Contrast
- Ensure disabled variant still distinct: validate relative luminance difference of at least ~0.25 (simple heuristic). If too close, auto-adjust mix upward by +0.1 (clamp ≤ 1.0).
- Provide optional future config fields `min_luminance_delta_disabled`, `min_luminance_delta_active` (NOT required now – note as potential enhancement comment).

## 11. Testing Checklist
You MUST add (or update) tests where applicable (pure functions only):
- Color derivation lerp correctness.
- Palette variant generation respects mix values & clamps.
- State transition logic chooses correct state given sizes & active status (unit test via extracted pure function referencing threshold param passed in).
(Integration tests for ECS optional if existing harness present; otherwise leave a TODO comment.)

## 12. Logging & Diagnostics
- Log (info target `cluster_highlight`) on first initialization and on config validation warnings.
- Debug (only when feature `debug`) log state transitions (cluster id, old -> new, size) throttled (avoid per-frame spam).

## 13. Performance Considerations
- Avoid allocating new vectors per frame for palette – store variant arrays once.
- Reuse frame-local `HashMap` with `clear()` (or consider `SmallVec` + sort) if cluster count modest.
- Early exit tween system if no active tweens.

## 14. Migration / Backward Compatibility
- Default behavior (if feature disabled) should still show original enabled colors. Implement a config toggle `cluster_highlight.enabled: bool = true` (default true). If false, skip all new systems and variant generation.

## 15. Step-by-Step Implementation Order (DO THIS SEQUENCE)
1. Extend config structs + defaults + validation (new cluster_highlight block only).
2. Add `PaletteVariants` resource and generation system (Startup after config load, & Update on config change if hot-reload exists).
3. Add `ClusterVisualState` + `ClusterHighlight` components; attach where cluster entities are established.
4. Implement size evaluation system (uses existing cluster data + threshold from cluster_pop config) to set Disabled/Enabled.
5. Implement input handler modification to set Active & trigger pop logic.
6. Implement tween advancement system.
7. Integrate cluster current color into rendering pipeline.
8. Add unit tests for color math & state decision function.
9. Add logging + feature toggle guard.
10. Manual playtest: verify smooth ~200ms tween transitions; confirm states switch appropriately around threshold.

## 16. Success Criteria
- Disabled clusters visibly darker (towards black) than base by ~50% mix.
- Active clusters visibly lighter (towards white) than base by ~50% mix.
- Transition smooth (no jump) when crossing threshold or clicking cluster.
- No measurable (>0.2ms/frame) perf regression in color preparation for standard scene (baseline ball count).
- Feature can be disabled via config to revert original visuals instantly.

## 17. Future Enhancements (Document Only; DO NOT Implement Now)
- GPU-side blending using state factor to reduce CPU per-ball color writes.
- Configurable easing curve (ease-in-out vs linear) for tween.
- Dynamic mix percentages per color to maintain perceived luminance uniformity.
- Status outline / halo effect instead of only fill color change.

## 18. Deliverables
- Code changes implementing above.
- Updated / new tests.
- Updated documentation (config reference + brief note in README if one lists visual features).
- Summary of warnings if any invalid config encountered.

Execute now following the implementation order. Maintain minimal footprint; no unrelated refactors.
