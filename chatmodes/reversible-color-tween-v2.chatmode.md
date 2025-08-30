## reversible-color-tween-v2

<!--
Purpose: Provide smooth, reversible color tweening between Enabled/Disabled states for balls/clusters without color discontinuities when targets flip mid-tween.
Core Fix: Replace time-since-last-change formula with per-frame alpha convergence and eliminate discontinuities caused by progress inversion misuse.
Audience: AI assistant / contributor working inside this Bevy + Rapier + WGSL codebase.
CRITICAL: Preserve existing architectural patterns (plugins, system sets, ordering) and performance constraints (no per-frame heap churn).
-->

### <context>
Clusters are classified each frame (Enabled if above size/area thresholds; else Disabled). Balls adopt their cluster’s target state. Current color tween uses elapsed time since flip — causing visual pops when state reverses mid-transition. We need a robust reversible tween preserving color continuity.

### <designPrinciples>
You MUST:
- Maintain color continuity (no instantaneous jumps) regardless of flip timing.
- Keep logic O(ball_count + cluster_count) without extra heap churn.
- Avoid shader changes and large struct bloat.

### <dataModel>
Replace prior proposed progress/dir inversion with an absolute alpha approach:

```rust
#[derive(Component, Debug, Copy, Clone)]
pub struct BallState {
    pub target_enabled: bool, // current target (cluster classification result)
    pub alpha: f32,           // absolute blend: 0.0 = fully Disabled color, 1.0 = fully Enabled color
    pub last_flip_time: f32,  // timestamp of last target change (optional: logging / analytics)
}
```

Derived (NOT stored):
```
progress_to_target = if target_enabled { alpha } else { 1.0 - alpha };
```
This satisfies requirement that “progress toward current target” is 1 when fully at target. Initial disabled: alpha=0 → progress_to_target = 1 (at target).

### <initialization>
When inserting new `BallState`:
- If cluster classified Enabled: `target_enabled=true, alpha=1.0`.
- Else Disabled: `target_enabled=false, alpha=0.0`.
- `last_flip_time = now`.

### <targetFlipHandling>
On classification pass each frame:
1. Compute `desired_enabled` for ball (cluster’s state).
2. If `state.target_enabled != desired_enabled`:
   - `state.target_enabled = desired_enabled`
   - `state.last_flip_time = now`
   - DO NOT modify `alpha` (continuity preserved).

### <advancement>
Let `dt = time.delta_seconds()`; let `dur = clamp(cfg.ball_state.tween_duration, 0.01, MAX)`.
Per frame after potential target update:
```rust
let step = (dt / dur).min(1.0);
if state.target_enabled {
    state.alpha = (state.alpha + step).min(1.0);
} else {
    state.alpha = (state.alpha - step).max(0.0);
}
```
Endpoint stabilization: when `alpha` hits 0.0 or 1.0 it naturally stops (no extra flag needed). Reversals simply change direction of alpha change next frame; no snapping.

### <colorComputation>
In metaball material assembly:
```rust
let enabled_col   = color_for_index(ci);
let disabled_col  = secondary_color_for_index(ci);
let blended       = lerp_color(disabled_col, enabled_col, state.alpha);
```
If `BallState` missing: treat as enabled with `alpha=1`.

### <progressToTargetUsage>
When needed:
```rust
let progress_to_target = if state.target_enabled { state.alpha } else { 1.0 - state.alpha };
```

### <ordering>
Preserve existing order:
```
compute_clusters
  -> update_ball_states (classification + alpha advancement)
  -> update_metaballs_unified_material (reads BallState.alpha)
```

### <config>
Reuse `GameConfig.ball_state.tween_duration` (clamped > 0.0 -> fallback 0.01). No new config fields.

### <migration>
Update all insertions of old `BallState { enabled, last_change }` to new fields. Provide helper:
```rust
impl BallState {
    pub fn new(now: f32, enabled: bool) -> Self {
        Self { target_enabled: enabled, alpha: if enabled { 1.0 } else { 0.0 }, last_flip_time: now }
    }
}
```
Replace old time-based tween calculation.

### <removals>
- Remove elapsed-time fraction `(now - last_change)/dur` for color calculation.
- Remove any direction/progress inversion logic.

### <performance>
- Component size minimal (bool + 2 f32).
- Single add/sub per ball per frame; no heap allocations.
- No shader changes; GPU uniform packing unchanged.

### <logging>
On target flip only:
```rust
info!(target:"ball_state", "Ball {:?} -> {}", e, if state.target_enabled { "Enabled" } else { "Disabled" });
```
No logs for incremental alpha progression.

### <tests>
1. mid_tween_reversal_continuity:
   - Start disabled (alpha=0).
   - Enable; advance 0.18s of 0.4s duration (alpha≈0.45 ±0.02).
   - Disable (flip) same frame; color identical pre/post flip.
   - Advance 0.22s; alpha≈0.00 (±0.01).
2. rapid_toggle_stability:
   - Duration 0.4s; toggles at 0.1,0.2,0.3 starting disabled->enable->disable->enable.
   - Alpha shows smooth monotonic segments; never jumps to endpoints except naturally when reached.
3. alpha_bounds:
   - Simulate >2*duration forward/back; alpha ∈ [0,1].
4. progress_to_target_semantics:
   - When at target endpoint, derived progress_to_target == 1.
5. instant_duration_clamp:
   - Config duration <=0 leads to clamp -> effectively instant transitions.

### <edgeCases>
- Multiple flips same frame: only final target matters; alpha unchanged.
- Duration extremely small: rapid but still multi-frame unless truly tiny (clamp 0.01).
- Orphan ball (no cluster this frame): retain previous target & alpha (no regression); document assumption.

### <successCriteria>
You MUST verify:
1. No visible color pops on rapid threshold oscillations.
2. Mid-flight reversal preserves color continuity.
3. Endpoint colors identical to prior system.
4. Runtime performance unaffected.
5. All tests pass.

### <implementationSteps>
1. Modify `BallState` struct & helper constructor.
2. Update classification system to manage `target_enabled` & `alpha` advancement.
3. Replace old `compute_tween_color` usage with direct alpha blend.
4. Update README Tweening subsection to describe alpha-based reversible model.
5. Add tests listed above.
6. Update CHANGELOG (Unreleased) describing reversible tween refactor.
7. Run build, tests, clippy.

### <optionalEnhancements>
- Future easing (smoothstep) can be applied to `alpha` or step factor; must remain symmetric to keep reversibility artifact-free.

### <notes>
Design honors requirement: “fraction toward current target” available (derivable) while maintaining visual continuity by not inverting alpha at flips.
