# Metaballs Cluster State Prompt: Disabled / Ready / Dying Lifecycle & GPU Color Tweening

## Objective
You WILL implement per-cluster lifecycle visualization states (Disabled, Ready, Dying) for metaball rendering with smooth tweening entirely (or predominantly) on the GPU by reusing the existing `cluster_colors` uniform array (`vec4` per cluster) without expanding the `MetaballsData` uniform layout. You WILL encode state + transition progress in the unused `.w` channel of each cluster color to drive a deterministic color remap in the WGSL fragment path.

## Target States
| State | Definition | Visual Requirement | Interaction |
|-------|------------|--------------------|-------------|
| Disabled (0) | Cluster ball_count < `cluster_pop.min_ball_count` OR (optionally) total_area < `cluster_pop.min_total_area` when > 0 | Fade base palette color toward black (desaturate & darken) | NOT clickable (should not trigger cluster_pop) |
| Ready (1) | Meets / exceeds thresholds and not yet popped | Current behavior (unchanged) | Click / tap can trigger pop |
| Dying (2) | All member balls have entered `PaddleLifecycle` (post cluster_pop) and are shrinking until despawn | Tween from base color toward a luminous highlight (mix toward white) while optional radius fade occurs | NOT clickable |

## GPU Encoding (No Layout Breakage)
You MUST avoid changing the binary layout of `MetaballsData`. Instead, you WILL pack lifecycle `state` and a `progress` scalar into `cluster_colors[i].w`:

```
let packed = state as f32 + progress; // state in {0.0,1.0,2.0}; progress in [0,1) guaranteed
state   = u32(floor(packed + 1e-6));
progress = fract(packed);
```

Constraints:
1. `progress` MUST remain < 1.0 so integer floor is stable.
2. `progress` is a monotonic (clamped) tween parameter for the *active* state transition (entering Disabled or Dying).
3. Ready state uses `progress = 0.0` (ignored in shader).

## State Transitions & CPU Responsibilities
You WILL manage cluster lifecycle each frame after cluster computation (`compute_clusters`) but before uniform upload for metaballs:

1. Build / update a `ClusterLifecycleState` resource keyed by *cluster id* (reusing persistence mapping already maintained for clusters). Each entry stores:
   - `state: ClusterVisualState` (enum Disabled/Ready/Dying)
   - `t: f32` (current tween progress 0..1)
   - `entered_at: f32` (time for debugging / potential easing)
2. Determine raw state:
   - Collect `ball_count` and `total_area` from `Clusters` resource.
   - If cluster id (color group) has *any* entity with `PaddleLifecycle` => raw state = Dying.
   - Else if `ball_count < cfg.interactions.cluster_pop.min_ball_count` OR (cfg.interactions.cluster_pop.min_total_area > 0 && total_area < that) => Disabled.
   - Else Ready.
3. If raw state differs from stored state, reset tween timer (`t = 0`, update `state`).
4. Advance tween: `t = min(1.0, t + dt / TRANSITION_SECS)`. Provide config constant: `state_tween_duration` (default 0.25s) under a new optional `visual` config section OR hard‑code initially (document location for future config addition). Avoid adding to existing uniform for now.
5. Pack: `cluster_colors[i].w = state as f32 + t.clamp(0.0, 0.999_99);`

## WGSL Shader Modification (Minimal & Localized)
You WILL modify only the fragment section after retrieving `base_col` and before foreground mode switch:

Add decode + color shaping:
```wgsl
let packed = metaballs.cluster_colors[cluster_idx].w;
let fstate = floor(packed + 1e-6);
let state_u = u32(fstate);
let prog = clamp(packed - fstate, 0.0, 1.0);
var color = base_col;
switch (state_u) {
  case 0u: { // Disabled -> darken toward black
      // perceptual-ish fade: use pow for smoother low-end rolloff
      let dim = pow(1.0 - prog, 1.2);
      color = base_col * dim; // no hue shift
  }
  case 1u: { // Ready -> unchanged
      color = base_col;
  }
  default: { // Dying (2)
      // brighten toward white; optional slight desat (lerp to luminance) for glow neutrality
      let white_mix = prog; // linear; consider pow(prog,0.9) if wanting early pop
      color = mix(base_col, vec3<f32>(1.0,1.0,1.0), white_mix);
  }
}
// Replace original base color usage with `color` downstream.
```

You MUST ensure this runs *before* bevel / glow foreground computations so lighting reflects modified color. Replace references to `fg_ctx.cluster_color` with `color` (or update the construction of `fg_ctx`).

## Easing & Visual Guidelines
You SHOULD favor visually smooth but brief transitions:
* Disabled fade: darkens relatively quickly but leaves silhouettes visible (avoid full black -> aim for ~5–15% base luminance floor if you later introduce floor).
* Dying brighten: avoid 100% white until late; consider using `pow(prog, 1.5)` for highlight only near completion to reduce constant glare.
* Keep tween compute branchless-ish where possible; current switch acceptable (only 3 states) — DO NOT introduce loops.

## Interaction Consistency
Disabled clusters MUST remain excluded from cluster_pop selection logic (already implicitly true due to threshold checks) — no change required.
Dying clusters (balls with `PaddleLifecycle`) already excluded if `exclude_from_new_clusters` is true; no additional input filtering required.

## Data Flow Summary
```
physics.clustering.compute_clusters --> Clusters(Vec<Cluster>)
cluster_state_update (NEW)          --> ClusterLifecycleState (resource updated per cluster id)
uniform_fill (existing)             --> cluster_colors[i].rgb (palette) & .w (state+progress)
WGSL fragment decode                --> final per-pixel color transform
```

## Rust Additions (High-Level)
You WILL add:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClusterVisualState { Disabled=0, Ready=1, Dying=2 }

#[derive(Default, Resource)]
pub struct ClusterLifecycleState(pub HashMap<u64, ClusterVisualEntry>);

pub struct ClusterVisualEntry { pub state: ClusterVisualState, pub t: f32 }
```
Where `u64` is the persistent `cluster_id` already assigned in `cluster.rs` (persistence map). Update / insert entries each frame before uniform upload.

Add a system `update_cluster_visual_state` scheduled AFTER `compute_clusters` and BEFORE the system that writes metaball uniforms (locate existing plugin ordering). Use an explicit system label or ordering set to guarantee correctness; DO NOT rely on insertion order.

## Uniform Packing Logic (Example Snippet)
```rust
let s_val = entry.state as u32 as f32;
let packed = s_val + entry.t.min(0.999_99);
cluster_colors[i].w = packed; // where i matches palette color index mapping
```

IMPORTANT: Ensure `cluster_colors[i].rgb` remains the canonical palette color (no pre-darkening on CPU) so GPU path holds single source of truth and palette editing / runtime palette swaps continue to function.

## Testing Strategy
1. Unit Test: Transition packing round-trip (encode -> decode logic replicated in test) yields original state enum & progress within ~1e-6 tolerance.
2. Unit Test: Disabled detection — fabricate Clusters with varying counts and assert state assignment.
3. Integration (visual/manual): Force a cluster_pop; observe progressive brighten; ensure decode does NOT flicker (state stable, progress monotonic).
4. Performance: Confirm no additional allocations in hot path: reuse buffers; `HashMap` growth stabilized (reserve using previous capacity).
5. Shader Validation: Build native + wasm; ensure no layout changes triggered (MetaballsData size unchanged) & no additional uniform added.

## Performance Constraints
You MUST keep per-frame CPU overhead minimal:
* O(clusters) updates only.
* No iteration over all balls just for dying detection: mark a flag when inserting `PaddleLifecycle` OR maintain a `Query<(Entity, &PaddleLifecycle)>` and build a `HashSet<u64>` of dying cluster ids using persistence map lookups (bounded by popped cluster size, typically small).
* Avoid floating division in shader inside switch — current operations are simple mix/mul.

## Accessibility & Visual Clarity Considerations
You WILL retain color hue for Disabled clusters (only luminance attenuation) so color-blind users relying on isolating by hue can still differentiate when zooming or focusing on active clusters.
You WILL avoid pure black to maintain silhouette contrast against darker backgrounds (consider documenting a floor later: `color *= dim * 0.85 + 0.05`). This is OPTIONAL now (do not implement until validated) but note in code with a comment if you add it.

## Future Extensions (Document, DO NOT Implement Now)
* Optional secondary highlight ring shader path for `Dying` using existing OutlineGlow mode synergy.
* Optional palette-driven target color for Dying (instead of white) stored in another palette entry.
* Optional per-cluster timers encoded via alternative packing (e.g., `state*4.0 + t` for more future states).

## Step-by-Step Implementation Order (MANDATORY)
1. Add `ClusterVisualState`, `ClusterLifecycleState`, and system `update_cluster_visual_state`.
2. Hook system ordering: after `compute_clusters`, before metaball uniform upload system (locate & label if missing).
3. When cluster_pop triggers (adding `PaddleLifecycle`), you MAY (optional) immediately set cluster state to Dying by marking in lifecycle map (system will also derive it next frame).
4. Implement encoding; assign `.w` each uniform update.
5. Modify WGSL: decode + color transform, replacing `base_col` usage.
6. Add decoding unit test for packing.
7. Add disabled detection unit test.
8. Manual visual validation (native + wasm) verifying smooth tween ~0.25s.
9. Update docs / README feature list (optional) referencing lifecycle visuals.

## Validation Checklist
- [ ] No change to `MetaballsData` struct size / layout.
- [ ] Shader compiles (native + wasm).
- [ ] Disabled cluster dims correctly within tween duration.
- [ ] Ready clusters unaffected.
- [ ] Dying cluster brightens smoothly until despawn.
- [ ] State transitions never skip (e.g., Disabled directly to Dying only if popped while below threshold — acceptable) and no flicker.
- [ ] No panics or unwraps introduced; all queries gracefully early‑out.
- [ ] Packing / decoding consistent (manual test prints match expected state/progress).

## Success Criteria
The implementation is complete when per-cluster states drive GPU-only color modulation with smooth (visually artifact-free) transitions, requiring no additional uniform buffers, and preserving existing interaction semantics & performance characteristics (≤ negligible frame time increase).

---
Provide progress metrics & any deviations if constraints force a layout extension (should NOT be necessary). If deviations required, clearly justify.
