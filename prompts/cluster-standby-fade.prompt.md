# Cluster Standby Fade Prompt: Dim Non-Clickable Clusters (< min_ball_count) To 50% Alpha

## Objective
You WILL implement a rendering-time fade (target alpha ≈ 0.50) for every ball (and its cluster) that does NOT meet the clickable threshold (`cluster_pop.min_ball_count`). Only clusters whose `ball_count >= min_ball_count` remain fully opaque (alpha = 1.0). This applies to:
- Individual isolated balls (cluster size = 1)
- Small clusters of size 2 or 3 (assuming default `min_ball_count = 4`)
- Any other cluster below the threshold

The feature MUST be:
1. Configurable via `cluster_pop` config additions.
2. Zero-overhead when disabled.
3. Compatible with both circle (flat 2D) rendering (`draw_circles = true`) and metaball rendering (when feasible).
4. Non-destructive: Full alpha MUST be restorable instantly if a cluster grows to threshold.

## Configuration Additions (ClusterPopConfig)
You WILL extend `ClusterPopConfig` with:
```ron
standby_fade_enabled: bool = true,        // master toggle
standby_fade_alpha: f32 = 0.50,           // target alpha for non-clickable clusters (0.05..1.0)
standby_fade_lerp_speed: f32 = 12.0,      // smoothing rate (alpha spring: dt * speed)
standby_fade_mode: u32 = 0,               // 0=Alpha only, 1=Alpha + slight luminance dim
standby_affects_popping: bool = false,    // if a cluster is in PaddleLifecycle, ignore fade
```
Validation Rules (add to existing config validation pass):
- Clamp `standby_fade_alpha` into [0.05, 1.0]; warn if outside.
- Clamp `standby_fade_lerp_speed` to ≥ 0.1.
- Unknown `standby_fade_mode` => treat as 0 (warn).
- If disabled (`standby_fade_enabled=false`), skip all runtime processing early.

## High-Level Approach
You WILL reuse existing clustering output (`Clusters` resource) WITHOUT recomputing adjacency. For each frame:
1. Build a lightweight `HashSet<Entity>` (or temporary `Vec<bool>` keyed by entity index if you already collect in uniform build step) of “active” entities (clusters meeting threshold).
2. Iterate visible balls (single query) and adjust their *visual alpha target* based on membership.
3. Apply smoothing (lerp) toward target alpha to avoid flicker if clusters fluctuate around the threshold.
4. Write alpha (and optional dim) into:
   - Circle mode: the child `ColorMaterial`
   - Metaball mode: a per-ball or per-cluster factor packed into existing uniforms WITHOUT altering struct layout (see Reuse Path below)

## Reuse Path (Minimize New Data)
### Circle Rendering (Flat)
You WILL:
- Reuse existing child mesh material (`ColorMaterial`) modification pattern used in `PaddleLifecycle` fade logic (avoid re-implementing alpha capture).
- Introduce a component `StandbyFadeBaseAlpha(f32)` inserted on first modification to store original (unfaded) value—only if not already present (do NOT duplicate with `PaddleLifecycle::alpha_base`; keep them independent to prevent cross-talk).
- Perform per-frame:
  ```rust
  new_alpha = lerp(current_alpha, target_alpha, 1.0 - f32::exp(-speed * dt));
  ```
  This exponential smoothing is stable at variable frame rates.

### Metaball Rendering
You WILL avoid structural uniform expansion. Two viable reuse strategies (choose one; prefer Strategy A initially):

**Strategy A (Cluster-scope alpha in existing `cluster_colors[].w` fractional portion)**  
If `cluster_colors[].w` is NOT already claimed by another feature, pack:
```
cluster_colors[i].w = 1.0 - standby_dim_amount   // e.g., 1.0 (active) .. 0.5 (standby)
```
Where `i` is color/palette index. DISADVANTAGE: All clusters of same color share dim (palette index conflates). Only acceptable if palette identity already implies “group”.

**Strategy B (Add secondary per-ball scalar buffer)**  
If Strategy A conflicts with color-sharing across multiple discrete clusters, you WILL:
- Reuse existing per-ball upload pipeline (where positions/radii are written) and append a parallel float array `ball_fade[i]` (storage buffer if uniform exceeds limits).
- Shader multiplies computed final color alpha by `ball_fade[i]`.
- Only implement if distinct clusters sharing palette require separate dim.

You MUST clearly document which strategy was chosen. DO NOT silently collide with any existing `.w` semantic (e.g., previous lifecycle prompt). If lifecycle encoding already occupies `.w`, prefer Strategy B.

## System Additions
You WILL add a focused plugin `StandbyFadePlugin` (or extend `ClusterPopPlugin` if policy prefers consolidation) registering:
1. `update_standby_fade_targets` (after `compute_clusters`, before rendering uniforms / material writes).
2. `apply_standby_fade_visuals` (only in circle mode; scheduled after target update & before frame present; can share ordering set with existing `PaddleLifecycle` visual update to ensure deterministic alpha layering).

Ordering:
```
compute_clusters --> update_standby_fade_targets --> (metaball uniform write / circle visuals) --> rendering
```

## Data Structures
```rust
#[derive(Component, Debug)]
pub struct StandbyFade {
    pub current_alpha: f32,    // smoothed runtime alpha
    pub target_alpha: f32,     // goal (1.0 or standby_fade_alpha)
}
#[derive(Component, Debug)]
pub struct StandbyFadeBaseAlpha(pub f32); // original material alpha
```
You WILL only attach `StandbyFade` to parent ball entities (not children). Material access remains via child query when applying results.

## Algorithm (Per Frame)
1. Early return if config disabled.
2. Build `active_entities: HashSet<Entity>`:
   - Iterate `clusters.0`
   - If `cluster.entities.len() >= min_ball_count` push all member entities.
3. Query all `Ball` entities (include optional `PaddleLifecycle`):
   - Determine `is_active = active_entities.contains(e)`
   - If `PaddleLifecycle` present and `standby_affects_popping == false`, force active.
   - Desired target = `1.0` if active else `standby_fade_alpha`
   - Insert `StandbyFade { current_alpha: base_alpha (lazy), target_alpha }` if missing
   - Update `target_alpha` if changed
4. Smoothing:
   ```
   let speed = cfg.interactions.cluster_pop.standby_fade_lerp_speed;
   fade.current_alpha += (fade.target_alpha - fade.current_alpha) * (1.0 - (-speed * dt).exp());
   ```
5. Circle mode application:
   - For each ball with children, find child `MeshMaterial2d<ColorMaterial>`, capture base alpha into `StandbyFadeBaseAlpha` once.
   - Compute final alpha = `base * fade.current_alpha`
   - If `standby_fade_mode == 1`, also apply luminance dim:
     ```
     let c = mat.color.to_srgba();
     let rgb = Vec3::new(c.red, c.green, c.blue) * (0.85 + 0.15 * fade.current_alpha);
     ```
     Keep change minimal; document constant.
6. Metaballs:
   - Strategy A: cluster-wide factor merged before shader color finalize, OR
   - Strategy B: write per-ball factors into extended buffer, multiply alpha in shader.

## Shader Adjustments (Metaballs)
(Only if implementing metaball fade initially)
Add at fragment step, before foreground compositing:
```wgsl
// Assume fade_factor provided (either cluster or per-ball)
var alpha = base_alpha * fade_factor;
// If dim mode 1:
let dim = mix(0.5, 1.0, fade_factor); // 0.5 floor to avoid invisible
color.rgb *= dim;
```
Preserve existing iso blending & glow logic order (apply dim pre-emission if used).

## Performance Constraints
You WILL:
- Allocate NO per-frame heap structures beyond a single `HashSet<Entity>` sized at O(active_balls). Reuse via `clear()` if stored as a `Resource`.
- Avoid per-ball material lookups if unfaded (skip write if delta alpha < 1e-3).
- Short-circuit entire fade pipeline if `standby_fade_enabled == false` OR all clusters active OR (min_ball_count <=1).
- Avoid cloning handles; only mutable borrow of `ColorMaterial` when alpha change required.

## Edge Cases
| Case | Expected Behavior |
|------|-------------------|
| min_ball_count <= 1 | Feature auto no-op (all clusters “active”). |
| Cluster toggles size around threshold | Alpha smoothly converges; no flicker due to smoothing. |
| Popped cluster shrinking | If `standby_affects_popping=false` remains fully opaque (prioritize lifecycle visualization). |
| draw_circles=false (metaballs only) | Circle fade system not scheduled; fallback to shader path (if implemented) or NO fade (document limitation). |
| Config reload (future) | On change, system re-evaluates targets next frame; existing `StandbyFade` components reused. |

## Testing Strategy
You WILL add:
1. Unit Test: Lerp smoothing monotonic convergence toward target.
2. Unit Test: Threshold classification (fabricate two clusters with counts 3 & 4).
3. Unit Test: Config clamp warnings (standby_fade_alpha out of range).
4. (Optional) Shader compile check (if metaball integration included).
5. Manual: Toggle `min_ball_count` in config -> observe fade update next launch.

## Accessibility & Visual Clarity
You WILL preserve hue and general contrast of active clusters for focus. Standby dim reduces visual noise but retains structure (alpha 0.5 default is sufficient). Avoid full transparency to maintain spatial awareness.

## Implementation Order (MANDATORY)
1. Extend `ClusterPopConfig` with new fields + defaults + validation warnings.
2. Add `StandbyFade`, `StandbyFadeBaseAlpha` components & `StandbyFadePlugin`.
3. Implement `update_standby_fade_targets` system (active set building + target assignment).
4. Implement `apply_standby_fade_visuals` (circle mode).
5. (Optional) Metaball Strategy selection (A or B); document choice.
6. Add tests for smoothing + threshold classification.
7. Update sample `game.ron` with new config fields (commented where defaults suffice).
8. Document limitation if metaball fade deferred.

## Validation Checklist
- [ ] Zero runtime cost when disabled.
- [ ] Active clusters remain visually unchanged.
- [ ] Non-clickable clusters smoothly settle at configured alpha.
- [ ] No panic on missing children (metaball-only mode).
- [ ] Alpha restoration immediate when cluster becomes active.
- [ ] Material updates minimized (only on meaningful delta).
- [ ] Config out-of-range values clamped & warned.

## Success Criteria
Feature considered complete when small clusters (size < min_ball_count) consistently appear at ~50% opacity (or config value) without flicker, performance regression is negligible, and enabling/disabling the feature is instantaneous via config on restart.

---
Provide a concise implementation summary & any deviations if metaball fade is postponed; otherwise include strategy annotation.
