# Cluster Pop (Explosive Burst + 1s Physics-Active Fade-Out) Prompt

## User Story (Enhanced Interaction)
As a player, when I burst (pop) a qualifying same-color cluster, I want the balls to violently shoot outward from their centroid and then visibly shrink & fade over ~1 second while still colliding / influencing physics, so the pop feels juicy and satisfying instead of a bland instant despawn.

## Objective
You WILL extend the existing cluster pop mechanic to replace instant despawn (or inert delay) with a **physics-active explosive ejection + graceful 1.0s shrink/fade cycle** that ends in despawn. During the fade period the balls MUST:
- Continue to have physics bodies & velocities (they still bounce / transfer momentum).
- Visually scale down (default: from 100% radius to 0% over fade duration) and optionally alpha fade (if materials permit).
- Become non-selectable for subsequent pops (they should not be re-clustered / re-popped).

## Scope
You WILL modify / add systems inside `interaction::cluster_pop` (and minimally inside clustering logic if exclusion is needed) without regressing existing generic explosion behavior.

## Additions to Config (`GameConfig.interactions.cluster_pop`)
Add the following fields (with defaults):
```
fade_enabled: true,        // Master toggle for fade-out (if false, fall back to immediate despawn behavior)
fade_duration: 1.0,        // Seconds from start of pop to final despawn (>= 0.05 for visible effect)
fade_scale_end: 0.0,       // Target visual scale factor at end (0 = vanish). Range [0,1]
fade_alpha: true,          // If true attempt to lerp material alpha to 0 (only if material supports)
exclude_from_new_clusters: true, // If true, popped balls are ignored in recomputation to avoid re-selection
collider_shrink: false,    // If true, also reduce collider radius in sync with visual (cost: recreate collider)
collider_min_scale: 0.25,  // Clamp physics collider shrink to this fraction if collider_shrink = true
velocity_damping: 0.0,     // Optional linear damping added ONLY during fade (additive), e.g. 2.5 for gentle slowdown
spin_jitter: 0.0,          // If > 0, add small random angular impulse at pop start for visual flair
```
Validation (during config load):
- Clamp negatives to 0.
- Enforce `fade_duration >= 0.05` if `fade_enabled` (warn if shorter; auto-adjust to 0.05).
- Clamp `fade_scale_end` to [0,1].
- If `fade_enabled` == false but any fade_* overrides differ from defaults, warn (settings ignored).

## Data / Types
Add / extend types (existing `ClusterPopped` event retained):
```rust
#[derive(Component, Debug)]
pub struct PoppingBall {
    pub elapsed: f32,          // seconds since pop started
    pub duration: f32,         // cached from config.fade_duration
    pub start_radius: f32,     // original logical radius
    pub end_scale: f32,        // target scale factor (fade_scale_end)
    pub fade_alpha: bool,      // per-entity toggle (copied from config at pop time)
    pub collider_shrink: bool, // apply physics shrink
}
```
Replace / supersede lightweight `PopFade` (you MAY remove it if unused). If you retain it, deprecate in doc comments and ensure only one is attached at a time.

## System Overview
You WILL implement / adjust these systems:
1. `handle_tap_cluster_pop` (existing):
   - After selecting & validating cluster, for each ball entity:
     - Apply outward impulse (retain existing direction logic) + optional spin jitter (if enabled).
     - Insert `PoppingBall` component (NOT `PopFade`).
     - If `velocity_damping > 0`, insert / adjust a `Damping` component (additive or override linear_damping = current + velocity_damping) only for fade lifetime.
   - DO NOT despawn immediately.
   - Emit `ClusterPopped` event.
2. `update_popping_balls` (NEW, in `Update`):
   - Query `(Entity, &mut PoppingBall, &mut Transform, Option<&mut Collider>, Option<&mut MeshMaterial2d<ColorMaterial>>, Option<&BallRadius>)` + optional `&mut Damping`.
   - Advance `elapsed` by `delta`.
   - Compute normalized t = (elapsed / duration). Clamp to [0,1]. Optionally ease (recommend smoothstep: `t * t * (3 - 2 * t)`).
   - Visual scale factor = lerp(1.0, end_scale, eased_t).
   - Update child visual scale: if using child `BallCircleVisual`, adjust its `Transform.scale`; if the collider & rendering share parent scale, adjust parent `Transform.scale` uniformly.
   - Alpha fade: if `fade_alpha` and material supports, multiply base color alpha by (1 - eased_t). (You MUST store original alpha; easiest: read once at insert time & store in a component or extend `PoppingBall` with `base_alpha: f32`.)
   - If `collider_shrink` true: compute physics scale factor = lerp(1.0, max(end_scale, collider_min_scale), eased_t); if factor changed beyond small epsilon, recreate or replace collider shape (Rapier requires new `Collider::ball(new_radius)`). Keep `BallRadius` immutable unless design requires; if you update it, note future systems may depend on original radius (Document choice). Recommended: keep `BallRadius` constant; only shrink collider shape.
   - When `elapsed >= duration`: despawn entity (use `despawn_recursive()` to remove child visuals); ensure removal order safe.
3. `exclude_popping_from_clustering` (OPTIONAL if `exclude_from_new_clusters`): add filter in clustering computation (e.g., skip entities with `PoppingBall`) OR pre-pass removal so they are not re-added to clusters while fading.
4. `cleanup_popping_state` (OPTIONAL): On despawn finished cluster you can rely on entity removal; not required.

## Selection & Explosion (Recap + Enhancement)
Selection logic stays: AABB pad hit OR centroid distance <= `tap_radius`; choose largest `ball_count`, tie-break by nearest centroid distance. Outward impulse magnitude formula (retain):
```
mag = explosion.impulse * cluster_pop.impulse_scale * (1 + outward_bonus) * radius_factor
radius_factor = (ball_radius / 10.0).max(0.1)
```
Add spin jitter:
```
if spin_jitter > 0 { apply angular velocity or torque scaled by spin_jitter * random_sign }
```

## Ordering
- Keep `handle_tap_cluster_pop` BEFORE generic explosion set (`TapExplosionSet`).
- Run `update_popping_balls` in `Update` AFTER physics step that integrates previous frame (or BEFORE if you want scaling to affect broadphase at same frame). Minimal: `.after(PrePhysicsSet)` or a distinct set.
- If collider shrinking is enabled, execute `update_popping_balls` BEFORE broad-phase sync for the next physics frame (i.e., still inside Update and BEFORE Rapier's finalize). Document chosen ordering.

## Interaction With Other Systems
You MUST ensure:
- Generic explosion system ignores taps that triggered a pop (unchanged via `TapConsumed`).
- Cluster recomputation (if each frame) either ignores `PoppingBall` or gracefully handles disappearing entities mid-fade.
- No double-despawn (guard against both fade completion & external cleanup).

## Acceptance Criteria
1. Popping a qualifying cluster launches its balls outward; they remain active, colliding for ~1 second.
2. During pop lifetime each ball visibly shrinks (and alpha fades if supported) toward `fade_scale_end` (default 0) using a smooth easing curve.
3. At exactly (±1 frame) `fade_duration` the balls are despawned.
4. Balls in fade state do NOT get re-clustered or re-popped.
5. Performance cost negligible (only O(pop_size) work per frame for active fading set; typical pop sizes small).
6. Config toggles correctly disable fade (falling back to immediate despawn) or collider shrink.
7. WASM compatible (no platform-specific APIs).
8. No panic / error logs when popping multiple clusters quickly.

## Edge Cases
| Case | Expected Behavior |
|------|-------------------|
| fade_enabled = false | Revert to immediate despawn (legacy behavior) |
| fade_duration = 0 | Force to 0.05 and warn (still visible minimal) |
| collider_shrink = true & very small end scale | Clamp collider scale to >= collider_min_scale |
| alpha unsupported | Skip alpha fade gracefully without panic |
| rapid taps causing overlapping pops | Each cluster handles independently; existing friction/impulse interplay stable |

## Implementation Steps (Ordered)
1. Extend `ClusterPopConfig` with new fade & physics fields + defaults & validation.
2. Add `PoppingBall` component (plus optional stored `base_alpha`). Remove `PopFade` or adapt migration (log once if both used).
3. Update `handle_tap_cluster_pop` to insert `PoppingBall` instead of immediate despawn when `fade_enabled`.
4. Add `update_popping_balls` system with scaling, easing, optional alpha & collider shrinking, final despawn.
5. Adjust clustering logic to skip `PoppingBall` entities if `exclude_from_new_clusters`.
6. Maintain `TapConsumed` gating for generic explosion.
7. Add debug logging (behind `debug` feature flag) when fade completes per entity count (optional aggregate: log once per pop when last ball despawns).
8. Manual test scenarios: qualifying cluster vs. too-small cluster; fade enabled & disabled; collider shrink toggled.
9. Update / regenerate README or configuration docs to include new fields.

## Easing Function Guidance
Default: smoothstep for subtle acceleration then deceleration: `eased = t * t * (3.0 - 2.0 * t)`. Provide constant-time alternative note: developers MAY swap to `t` or an ease-out (e.g., `1 - (1 - t)^2`). Keep function small & inline (no heavy curve libs required).

## Telemetry / Future Hooks (Deferred)
- Particle & sound triggers at pop start (subscribe to `ClusterPopped`).
- Screen shake amplitude scaled by total_area.
- Score multiplier weighting radius shrink progress (combo window).

## Accessibility Considerations
- Shrink + fade adds non-color motion cue to indicate removal.
- Future: add brief radial halo or pulse at pop start for low-vision clarity.

## Summary
You now have a juicy, kinetic cluster death: outward impulse + 1s physics-active shrink/fade improves player feedback while keeping systems modular and configurable. Follow ordered steps to implement with minimal risk & clear extension points.

---
Generated to enhance engagement & readability; validate with Prompt Tester.
