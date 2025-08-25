# Cluster Pop Fade (Physics-Active Shrink & Optional Alpha) Prompt

## Purpose
Enhance the cluster pop so that popped balls optionally undergo a configurable physics-active fade (shrink + optional alpha fade + optional damping + optional collider shrink) before final despawn, providing juicy feedback and preserving physical interactions briefly.

Explosion & drag interactions have been removed. Cluster pop is the sole tap interaction. All impulse math is now based on an absolute `cluster_pop.impulse` (no explosion baseline, no impulse_scale).

## Config Additions / Fields (ClusterPopConfig)
Already present (documented here for clarity):
```
fade_enabled: true,          // Master toggle for fade-out. If false: immediate (or timed) despawn
fade_duration: 1.0,          // Seconds (>= 0.05 when enabled)
fade_scale_end: 0.0,         // Target visual scale at end (0..1)
fade_alpha: true,            // Lerp material alpha -> 0 if supported
exclude_from_new_clusters: true, // Exclude popping balls from new cluster computations
collider_shrink: false,      // If true, also shrink collider radius
collider_min_scale: 0.25,    // Minimum collider fraction if shrinking
velocity_damping: 0.0,       // Extra linear damping applied during fade (additive)
spin_jitter: 0.0,            // Random angular velocity magnitude applied at pop start
```

Primary pop impulse fields used earlier in selection system (for reference):
```
impulse: 500.0,              // Absolute outward impulse base magnitude
outward_bonus: 0.6,          // Multiplier factor: final_base = impulse * (1 + outward_bonus)
```

## Validation Rules (Enforced Elsewhere)
- Negative numeric fields → treated as 0 (warn).
- `fade_enabled && fade_duration < 0.05` → raise to 0.05 (warn).
- `fade_scale_end` clamped to [0,1].
- `collider_min_scale` clamped to [0,1].
- If `fade_enabled == false` but fade-specific fields differ from defaults → warn (ignored).
- If `impulse <= 0` and `outward_bonus <= 0` → warn (no outward motion).

## Data / Component
```
#[derive(Component, Debug)]
pub struct PoppingBall {
    pub elapsed: f32,
    pub duration: f32,
    pub start_radius: f32,
    pub end_scale: f32,
    pub fade_alpha: bool,
    pub collider_shrink: bool,
    pub collider_min_scale: f32,
    pub base_alpha: f32,      // captured first frame (-1 sentinel means uncaptured)
    pub added_damping: f32,   // damping injected (for later potential cleanup)
}
```

## Pop Impulse Formula
Applied once at pop start per ball:
```
base_impulse      = cluster_pop.impulse
magnitude_base    = base_impulse * (1.0 + cluster_pop.outward_bonus)
radius_factor     = (ball_radius / 10.0).max(0.1)
impulse_vector    = dir_from_centroid * magnitude_base * radius_factor
vel.linvel       += impulse_vector
if spin_jitter > 0:
    vel.angvel   += random(-spin_jitter .. spin_jitter)
```

## Systems

### handle_tap_cluster_pop (Upstream)
- Selects a qualifying cluster (largest ball_count among spatial candidates).
- Applies impulses as above.
- Inserts `PoppingBall` when fade path is required OR handles immediate/timed despawn if fade disabled.
- Adds damping if configured.
- Emits `ClusterPopped` event.

(Already implemented; ensure removal of any legacy explosion gating logic is complete.)

### update_popping_balls
Responsibilities per frame:
1. Advance `elapsed += dt`.
2. Compute normalized `t_raw = (elapsed / duration).clamp(0,1)`.
3. Easing (smoothstep):
   ```
   t = t_raw * t_raw * (3.0 - 2.0 * t_raw)
   ```
4. Visual scale factor:
   ```
   scale_factor = 1.0 + (end_scale - 1.0) * t
   ```
   Apply to child visual mesh (retain original diameter = radius * 2).
5. Alpha fade (if `fade_alpha`):
   - On first update capture original alpha into `base_alpha` if sentinel (<0).
   - New alpha = `base_alpha * (1.0 - t)`.
6. Collider shrink (if `collider_shrink`):
   - Physics scale target = `max(end_scale, collider_min_scale)`.
   - Interpolate:
     ```
     phys_scale = 1.0 + (target - 1.0) * t
     new_radius = start_radius * phys_scale
     ```
   - Replace collider shape if changed meaningfully.
7. Despawn when `elapsed >= duration` (recursive).
8. (Optional future) Remove added damping or restore baseline if needed (currently left; added damping is small & entity despawns anyway).

## Ordering
- `handle_tap_cluster_pop` runs in `PrePhysicsSet`.
- `update_popping_balls` runs after `PrePhysicsSet` in same frame (so scaling visuals happens post-impulse but before broad-phase of next physics tick), acceptable for minor collider recreation cost.

## Performance Considerations
- Only entities with `PoppingBall` are processed.
- Per-entity work is constant; typical cluster sizes moderate.
- Collider recreation each frame only occurs when shrinking is enabled (config default avoids cost).
- Alpha & scale updates limited to a few property writes.

## Edge Cases
| Scenario | Behavior |
|----------|----------|
| fade_disabled & despawn_delay == 0 | Immediate despawn (no `PoppingBall`). |
| fade_disabled & despawn_delay > 0  | Timer-based minimal path (could reuse component with duration but no scaling). |
| collider_shrink true & end_scale near 0 | Radius clamped to `collider_min_scale * start_radius`. |
| spin_jitter large | Accept; high angular velocities left to physics (consider future clamp if instability). |
| impulse == 0 | Balls only shrink/fade in place (warn earlier). |
| exclude_from_new_clusters true | Clustering system skips entities with `PoppingBall` (ensures no re-pop). |

## Testing Checklist
- Pop qualifies: outward movement + fade (scale → 0, alpha → 0 if enabled).
- Pop with fade_disabled: Immediate disappearance.
- collider_shrink toggled: verify colliders shrink (debug features).
- spin_jitter > 0: angular variation observed.
- velocity_damping > 0: linear damping visibly slows fade.
- Large cluster vs small cluster threshold difference.
- WASM build parity (no platform APIs).

## Potential Future Enhancements
- Particle system spawn at pop centroid (subscribe to `ClusterPopped`).
- Audio cue & haptic feedback (mobile).
- Screen shake magnitude scaled by total_area.
- Optional chromatic or radial flash material effect during first 0.1s.

## Summary
`PoppingBall` provides a flexible, configurable fade pipeline for cluster pop: outward impulse first, then a configurable, visually rich decay phase that maintains physics interactions until final despawn. Explosion and drag mechanics are removed; the system is now self-contained and simpler.

---
Updated for post-explosion/drag removal refactor (absolute cluster_pop.impulse).
