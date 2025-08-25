# Cluster Pop (Tap Explode & Clear) Prompt

## User Story (Priority #3)
As a player, I can pop a same‑color cluster once it’s big enough so clearing feels obvious.

## Objective
You WILL implement a tap interaction that, when released on (or very near) a qualifying same‑color cluster, triggers an **explode + clear** action: an outward impulse burst (visual/physical feedback) followed by removal (despawn) of that cluster’s balls, and emission of a pop event to drive timer reset / scoring in later stories.

## Current Relevant Systems & Data
- Clustering already computed each frame by `physics::clustering::cluster::compute_clusters` producing `Res<Clusters>` (vector of `Cluster { color_index, entities, min, max, centroid, total_area }`).
- Tap / pointer release is currently handled by `handle_tap_explosion` (in `interaction::input::input_interaction.rs`) which applies an area explosion impulse if enabled.
- Explosion config exists under `GameConfig.interactions.explosion` (impulse, radius, falloff_exp, enabled).

## High‑Level Behavioral Requirements
You WILL:
1. Detect a pointer/tap release (mouse or touch) location in world space.
2. Identify the cluster (if any) that spatially contains or is close enough to that point (point inside cluster AABB expanded by a small epsilon or minimal distance to centroid < threshold radius).
3. Check cluster size thresholds (by ball count and/or accumulated area) from new config.
4. If thresholds pass, apply a directed outward impulse (mini explosion) to each ball in the cluster (scaled by existing explosion impulse * configurable scale * per‑ball radius factor) and then despawn the entities (immediate or after short optional delay).
5. Emit a `ClusterPopped` event containing: `color_index`, `ball_count`, `total_area`, `centroid`.
6. Suppress the generic area explosion for that same tap (so only one effect fires). If no qualifying cluster is popped, fall back to existing explosion logic unchanged.
7. Leave physics & rendering stable (no panics if cluster empties mid‑frame).

## Non‑Goals (For This Prompt)
- Scoring, combo, or timer reset logic (future stories will subscribe to the event).
- Visual particle FX, sounds, or UI feedback beyond physical impulse (can be added later via event listener).
- Partial cluster removal (all or nothing for the tapped cluster).

## Config Additions (`GameConfig`)
Add a new block under `interactions`:
```
cluster_pop: (
    enabled: true,
    min_ball_count: 4,        // cluster must have at least this many balls
    min_total_area: 1200.0,   // OR area threshold (π r^2 summed); set to 0 to ignore
    impulse_scale: 1.0,       // scales existing explosion.impulse baseline
    outward_bonus: 0.6,       // additional scalar when directing from centroid
    despawn_delay: 0.0,       // seconds; 0 = immediate despawn
    aabb_pad: 4.0,            // screen units to pad bounding box selection
    tap_radius: 32.0,         // fallback radial hit test if not inside AABB
)
```
Default strategy: qualify if `ball_count >= min_ball_count` **AND** `total_area >= min_total_area` (if `min_total_area > 0`, otherwise ignore area check). Provide `#[serde(default)]` & `Default` impl; integrate into validation warnings (clamp negatives to 0, impulse_scale & outward_bonus >= 0, etc.).

## Data / Types
Introduce:
```rust
#[derive(Event, Debug, Clone)]
pub struct ClusterPopped {
    pub color_index: usize,
    pub ball_count: usize,
    pub total_area: f32,
    pub centroid: Vec2,
}
```
Add `ClusterPopConfig` struct (serde) mirroring the config block.

## New Plugin
Create `ClusterPopPlugin` (e.g., `interaction::cluster_pop` module) that:
- Registers event type `ClusterPopped`.
- Inserts a transient `TapConsumed` resource (bool flag reset each frame) or a `Local<bool>` in the system to gate explosion fallback.
- Adds system `handle_tap_cluster_pop` scheduled BEFORE `handle_tap_explosion` (same schedule set `PrePhysicsSet` is fine, but ensure ordering with `.before(handle_tap_explosion)` label or add a system set label to explosion and order relatively).

## System: `handle_tap_cluster_pop`
Input:
- Button & touch release detection (replicate pointer world position logic used in `handle_tap_explosion`).
- `Res<Clusters>`.
- `Query<(&Transform, &BallRadius, &mut Velocity), With<Ball>>` for impulse application.
- `Res<GameConfig>` for thresholds + existing explosion impulse reference.
- `EventWriter<ClusterPopped>`.
- Mut `Commands` for despawn.
- Mut `ResMut<TapConsumed>` (or local bool) to signal success.

Algorithm:
1. Early return if config disabled or release didn’t happen or tap consumed by drag (respect existing `ActiveDrag.started`).
2. Compute world tap point.
3. Find candidate clusters:
   - For each cluster: if `point inside (min - pad .. max + pad)` OR distance(point, centroid) <= tap_radius.
   - Among candidates choose cluster with: (a) largest `ball_count` or (b) smallest distance to centroid; pick deterministic rule (document). Recommended: **largest ball_count**, tie‑break by smaller distance to centroid.
4. Validate size thresholds.
5. If not valid -> return (do not set consumed; explosion system proceeds).
6. For each entity in cluster:
   - Fetch velocity; compute direction = (entity_pos - cluster.centroid).normalize_or_zero(); magnitude = `explosion.impulse * cluster_pop.impulse_scale * (ball_radius.0 / 10.0).max(0.1)`; optionally multiply by `outward_bonus` (distinct from fallback explosion radial falloff; no distance attenuation inside cluster necessary or add mild random jitter).
   - Apply linear velocity impulse (additive like existing explosion).
7. If `despawn_delay <= 0` → immediately `commands.entity(e).despawn_recursive()` after impulse application; otherwise insert a `PopFade { timer }` component for a follow‑up fade+despawn system (optional stretch: implement simple alpha fade if material supports or just timer then despawn).
8. Accumulate area & ball count; emit `ClusterPopped` event.
9. Mark tap consumed to prevent generic explosion.

## Optional Fade System (Include Only If `despawn_delay > 0`)
Add a simple system iterating entities with `PopFade` reducing velocity slightly and despawning when timer finishes.

## Ordering & Safety
- Ensure `handle_tap_cluster_pop` executes before `handle_tap_explosion` so consumed flag bypasses explosion. Add label:
  - Add a public `SystemSet` label `TapExplosionSet` to explosion system; order with `.before(TapExplosionSet)`.
  - Or simpler: in `InputInteractionPlugin` modify registration: give `handle_tap_explosion` a label, then add new system with `.before(label)`. (Prompt includes making minimal intrusions—prefer adding labels without refactoring unrelated systems.)
- Despawning during Update BEFORE cluster recomputation in next frame is safe (cluster list rebuilt each frame). No need to manually remove from `Clusters` now.

## Validation Warnings (Config)
- `min_ball_count < 1` → set to 1 & warn.
- Negative numeric fields → clamp to 0.
- `impulse_scale == 0` disables outward velocity (allowed; warn if both `impulse_scale` and `outward_bonus` are 0 resulting in no visual feedback).

## Acceptance Criteria
- Tapping on sufficiently large same‑color cluster causes those balls to burst outward momentarily and then disappear.
- Tapping on clusters below threshold performs original explosion behavior (area impulse but no removal).
- Dragging (active drag) release does NOT pop clusters.
- Generic explosion still works when clicking empty space or tiny clusters.
- Event `ClusterPopped` observable (can be debug logged during development).
- Performance impact negligible (only per tap iteration over clusters, not per frame heavy logic).
- WASM parity maintained (no native‑only API usage).

## Edge Cases & Handling
| Case | Behavior |
|------|----------|
| Tap between overlapping AABBs | Deterministic selection (largest ball_count). |
| Cluster qualifies by count but not area (min_total_area > 0) | Not popped. |
| All thresholds 0 | Every cluster can pop; acceptable for sandbox. |
| Despawn delay > 0 but fade system absent | Just despawn when timer expires (no visual fade). |
| User taps while clusters vector empty | No action; explosion fallback may still run (0 balls → nothing). |

## Implementation Steps (Ordered)
1. Add `ClusterPopConfig` + defaults + integrate into `GameConfig` & `serde` defaults (`#[serde(default)]`).
2. Extend config validation with new warnings.
3. Add new module `src/interaction/cluster_pop/mod.rs` (plugin + systems + event + optional fade component).
4. Register `ClusterPopPlugin` from a central plugin aggregator (`interaction::mod.rs` or root `GamePlugin`).
5. Introduce system label for existing explosion (`.in_set(PrePhysicsSet).after(<drag set>)`) and order cluster pop system before it.
6. Implement `handle_tap_cluster_pop` logic; emit event; set consumed flag.
7. Modify `handle_tap_explosion` early return if consumed flag true.
8. (Optional) Implement `PopFade` if `despawn_delay > 0` path chosen.
9. Add debug log on event (behind `debug` feature) summarizing `color_index`, `ball_count`, `total_area`.
10. Manual test: threshold met vs not met; verify explosion suppression when popped.
11. Update sample `game.ron` with `cluster_pop` block & rerun README generator if it enumerates config fields.

## Future Extensions (Defer)
- Particle / sound FX triggered by `ClusterPopped` event.
- Timer reset & scoring integration (stories #4–#5 & #12+).
- Combo multiplier & event chaining.
- Partial pops (e.g., trim large cluster to threshold remainder).
- Visual highlight on hover/touch hold preview of qualifying cluster.

## Notes (Performance & Style)
- Keep new plugin small and isolated; avoid scattering logic into existing explosion system for clarity & toggling.
- Avoid allocating per frame; allocate only inside tap handler (rare). Reuse small temporary vectors if needed.
- Respect existing coding conventions: early returns, minimal mutable resources, no panics on `Query::single` assumptions.

## Accessibility Considerations (Foundational)
- Popping removes balls: ensure future visual/audio feedback also includes non‑color cues (sound, brief size flash) for color‑blind users (story #11 ties in).

## Summary
This prompt specifies an additive interaction enabling cluster popping (explode + clear) while preserving existing explosion behavior for non‑qualifying taps. It leverages current clustering data, introduces minimal configuration, and establishes an event foundation for subsequent gameplay systems (timer, scoring, combos).

---
Generated with accessibility in mind; please still review and test.
