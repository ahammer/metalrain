# Cluster Pop Paddle Transform Mechanic

## Objective
You WILL replace the current outward impulse + fade/despawn cluster pop behavior with a **scale animation** that transforms the tapped cluster into a temporary enlarged obstacle (enlarges then shrinks to zero). During the animation the cluster's balls:
- Continue normal physics motion (we do NOT freeze or zero velocities) so flow stays organic
- Keep active colliders (not sensors) so other balls can still collide with their *scaled* shape
- First grow to a peak scale (>1.0) then optionally hold, then shrink smoothly to 0 (despawn at end)
- Provide predictable, non‑chaotic interaction (no random outward explosion velocity)

## High-Level Behavior
1. User tap selects a qualifying cluster (existing selection thresholds remain).
2. All entities in that cluster enter a **PaddleLifecycle** with phases:
   - GROW: scale from 1.0 → `peak_scale`
   - HOLD (optional): maintain `peak_scale` (optional feature; can be zero duration)
   - SHRINK: scale from `peak_scale` → 0.0 (despawn at completion)
3. While in any phase, entities:
   - Retain their natural velocities (no forced freezing) to preserve fluid motion.
   - Retain colliders; collider radius follows visual scale (so collisions reflect new size).
4. Other balls collide normally, treating the enlarged cluster as a static obstacle that slowly disappears.
5. No outward impulses are applied; system is deterministic.

## Configuration Additions (cluster_pop section)
Add / replace fields inside `ClusterPopConfig` (remove prior impulse/outward_bonus usage):
```
cluster_pop: (
    enabled: true,
    min_ball_count: 4,
    min_total_area: 1200.0,
    peak_scale: 1.8,          // multiplier applied to radius at peak
    grow_duration: 0.25,      // seconds to grow 1.0 -> peak_scale
    hold_duration: 0.10,      // seconds to hold at peak (can be 0.0)
    shrink_duration: 0.40,    // seconds to shrink peak_scale -> 0
    collider_scale_curve: 1,  // 0 = linear, 1 = smoothstep, 2 = ease-out (enum/int)
   // freeze_mode REMOVED: we allow natural motion
    fade_alpha: true,         // reuse existing flag (alpha fade optional)
    fade_curve: 1,            // curve for alpha (same encoding as collider_scale_curve)
    aabb_pad: 4.0,            // unchanged (selection aid)
    tap_radius: 32.0,
    despawn_delay: 0.0,       // (unused now; can deprecate or keep for legacy, set to 0)
    exclude_from_new_clusters: true,
    collider_shrink: false,   // (superseded; can deprecate)
   velocity_damping: 0.0,    // (now unused; can remove or ignore)
    spin_jitter: 0.0,         // (unused; remove or ignore)
)
```

### Deprecations / Removals
Remove or ignore: `impulse`, `outward_bonus`, `velocity_damping`, `spin_jitter`, `collider_shrink`, `fade_scale_end`, `despawn_delay` (if fully replaced by shrink phase), `freeze_mode`, and any lingering fields tied to impulse logic.

## Data Structures
Introduce:
```rust
#[derive(Component)]
pub struct PaddleLifecycle {
   pub elapsed: f32,
   pub grow_duration: f32,
   pub hold_duration: f32,
   pub shrink_duration: f32,
   pub peak_scale: f32,
   pub base_radius: f32,     // original BallRadius
   pub fade_alpha: bool,
   pub alpha_base: f32,      // captured first update
}
```
(If enum serialization needed: implement `Deserialize` via string or numeric code matching config's `freeze_mode`).

## System Flow
1. `handle_tap_cluster_pop` (renamed or extended) now:
   - Select cluster.
   - For each ball:
   - Insert `PaddleLifecycle` with timings & peak_scale.
   - Capture & store original radius (BallRadius component value).
   - Emit `ClusterPopped` event (semantic rename optional: still indicates selection).
2. `update_paddle_lifecycle` (new system, runs in `Update` after selection, ideally before physics so collider size matches current frame collisions):
   - For each entity with `PaddleLifecycle` compute phase progress.
   - Derive normalized time `t` in [0,1] per phase:
     - If `elapsed < grow_duration`: phase=GROW, local_t = elapsed/grow_duration.
     - Else if within hold: phase=HOLD.
     - Else SHRINK: local_t = (elapsed - grow - hold)/shrink_duration.
   - Compute scale curve:
     - Let `curve(x)` depend on `collider_scale_curve` (0 linear: x, 1 smoothstep: x*x*(3-2x), 2 ease-out: 1 - (1-x)^3).
     - For GROW: `factor = lerp(1.0, peak_scale, curve(local_t))`.
     - For HOLD: `factor = peak_scale`.
     - For SHRINK: `factor = lerp(peak_scale, 0.0, curve(local_t))`.
   - Update child transform scale (visual diameter = radius * 2 * factor).
   - Update collider: `Collider::ball(base_radius * factor)` (skip if factor very tiny to avoid churn; despawn soon).
    - Alpha fade (if enabled): use same or separate `fade_curve`. For SHRINK phase only (or entire lifecycle) => alpha = base_alpha * (1 - shrink_progress_weighted).
    - No velocity mutation: allow natural drift and collisions for a more organic feel.
3. Completion: when `elapsed >= grow+hold+shrink`, `despawn(entity)`.

## Physics Considerations
- We intentionally do NOT freeze or override velocities; motion continues organically.
- Collider scale changes each frame; ensure not too large a `peak_scale` to avoid tunneling or performance hits (add validation: warn if `peak_scale > 3.0`).
- Avoid negative or NaN scale; clamp durations to > 0 except hold.

## Validation Logic Updates (`GameConfig::validate()`)
Add warnings:
- `peak_scale <= 1.0` (warn: effect visually subtle; still allowed).
- `peak_scale > 3.0` (warn: large scale may cause broad-phase stress).
- `grow_duration <= 0` (clamp to 0.01).
- `shrink_duration <= 0` (clamp to 0.05 minimal visual readability).
- `hold_duration < 0` (treat as 0).
- Unknown legacy fields (impulse, outward_bonus, freeze_mode, etc.) – single warning: "Ignoring legacy cluster_pop fields: impulse, outward_bonus, freeze_mode,...".

## Removal / Refactor Steps
1. Delete all impulse application code inside existing cluster pop system.
2. Replace fade logic component (`PoppingBall`) with `PaddleLifecycle` (or merge if reusing alpha fade code – new is cleaner).
3. Remove `velocity_damping` & spin jitter application (legacy / unused now).
4. System ordering: ensure `update_paddle_lifecycle` runs before main physics step so collider size is authoritative (e.g., in `PrePhysicsSet`).
5. Confirm removal of unused imports and any freeze-related code or enums.

## Despawn Timing
Despawn at end of SHRINK. Do NOT early despawn if factor hits near zero early—shrink is controlled by duration.

## Optional Extensions (Keep Out of Initial Scope Unless Needed)
- Particle burst at peak.
- Score / combo event emission at GROW completion.
- Sound event triggers per phase.
- Distinct collision category filtering (e.g., ignore other cluster pop lifecycles).

## Testing Strategy
- Unit test: scaling factors at midpoints (grow 50%, hold, shrink 50%).
- Ensure no NaN when durations extremely small.
- Validate config parser ignores legacy impulse & freeze_mode fields.
- Visual manual test: cluster scales smoothly while still drifting naturally with physics.

## Performance Notes
- Re-scaling colliders per frame: monitor for perf regression at large cluster counts; early exit if `factor` unchanged within epsilon.
- Avoid allocations inside per-frame loop; reuse RNG removal (no randomness now; eliminate `rand` dependency inside this system if not needed elsewhere).

## Migration Summary (for MIGRATION.md)
Add section:
```
Replaced impulse-based cluster pop with paddle transform animation (grow → hold → shrink) using ONLY a scale curve (no physics freezing). Removed fields: impulse, outward_bonus, velocity_damping, spin_jitter, collider_shrink, fade_scale_end, freeze_mode. Added: peak_scale, grow_duration, hold_duration, shrink_duration, collider_scale_curve, fade_curve.
```

## Validation Checklist
- [ ] No references to removed impulse-based fields in cluster pop logic.
- [ ] New config fields default properly & validate.
- [ ] PaddleLifecycle scaling matches time phases.
- [ ] Colliders resize & deflect other balls (observe visually).
- [ ] Balls retain natural motion (no artificial freezing applied).
- [ ] Despawn after full duration.
- [ ] Legacy config with impulse prints single warning.
- [ ] No clippy warnings introduced.

## Example Code Snippets
### Component Insert (Selection System)
```rust
commands.entity(e).insert(PaddleLifecycle {
    elapsed: 0.0,
    grow_duration: cp.grow_duration.max(0.01),
    hold_duration: cp.hold_duration.max(0.0),
    shrink_duration: cp.shrink_duration.max(0.05),
    peak_scale: cp.peak_scale.max(0.1),
    base_radius: radius.0,
    fade_alpha: cp.fade_alpha,
    alpha_base: -1.0,
});
```

### Update Lifecycle Core
```rust
let total = plc.grow_duration + plc.hold_duration + plc.shrink_duration;
plc.elapsed += dt;
let mut phase = LifecyclePhase::Grow;
let mut local_t;
if plc.elapsed < plc.grow_duration { local_t = plc.elapsed / plc.grow_duration; }
else if plc.elapsed < plc.grow_duration + plc.hold_duration { phase = LifecyclePhase::Hold; local_t = 0.0; }
else { phase = LifecyclePhase::Shrink; local_t = (plc.elapsed - plc.grow_duration - plc.hold_duration) / plc.shrink_duration; }
// curve(local_t) -> factor
```

## Edge Cases
| Case | Handling |
|------|----------|
| peak_scale < 1.0 | Allow but warn (effect subtle) |
| All durations tiny | Clamp ensures visible progression |
| Tap on overlapping clusters | Keep existing selection heuristic (largest count, distance tiebreak) |
| Ball already in lifecycle tapped again | Ignore (idempotent) |
| Legacy fade settings present | Map fade_alpha; ignore fade_scale_end |

## Success Definition
Mechanic yields stable, predictable obstacle effect: cluster visually inflates into a paddle, persists briefly, then collapses smoothly without random ejections.

---
<!-- End of Paddle Transform Prompt -->
