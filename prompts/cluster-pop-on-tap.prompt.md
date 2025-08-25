# Cluster Pop (Tap to Burst & Clear) Prompt

## User Story
As a player, I can pop a same‑color cluster once it’s big enough so clearing feels obvious and impactful.

## Objective
Implement / maintain a tap (pointer release) interaction that, when released on (or very near) a qualifying same‑color cluster, triggers a pop: an outward impulse burst (physical feedback) followed by removal (despawn or fade/despawn sequence) of that cluster’s balls, and emission of a `ClusterPopped` event for downstream systems (scoring, timers, FX).

The legacy generic explosion and drag interactions have been fully removed. Cluster pop is now the ONLY tap interaction.

## Current Relevant Systems & Data
- Clustering computed each frame: `physics::clustering::cluster::Clusters` (`Vec<Cluster { color_index, entities, min, max, centroid, total_area }>`).
- Tap release handled by `interaction::cluster_pop::handle_tap_cluster_pop`.
- Configuration: `GameConfig.interactions.cluster_pop`.

## Config (`ClusterPopConfig`)
```
cluster_pop: (
    enabled: true,
    min_ball_count: 4,        // cluster must have at least this many balls
    min_total_area: 1200.0,   // OR area threshold (set 0 to ignore)
    impulse: 500.0,           // absolute impulse magnitude applied outward
    outward_bonus: 0.6,       // multiplicative (1 + outward_bonus)
    despawn_delay: 0.0,       // seconds; 0 = immediate (or fade system may override)
    aabb_pad: 4.0,            // selection AABB padding
    tap_radius: 32.0,         // fallback radial hit test
    fade_enabled: true,       // fade subsystem (see fade prompt)
    fade_duration: 1.0,
    fade_scale_end: 0.0,
    fade_alpha: true,
    exclude_from_new_clusters: true,
    collider_shrink: false,
    collider_min_scale: 0.25,
    velocity_damping: 0.0,
    spin_jitter: 0.0,
)
```

## Selection Logic
1. On pointer/tap release, compute world position.
2. Consider each cluster:
   - Inside padded AABB (min - pad .. max + pad) OR
   - Distance to centroid ≤ `tap_radius`.
3. Among candidates choose the one with largest `ball_count`; tie‑break by smaller centroid distance.

## Qualification
A candidate cluster qualifies if:
- `ball_count >= min_ball_count`
- AND (`min_total_area == 0` OR `total_area >= min_total_area`)
- AND `cluster_pop.enabled`.

## Impulse Formula
Outward impulse per ball (added to linear velocity):
```
base_impulse = cluster_pop.impulse
magnitude_base = base_impulse * (1.0 + cluster_pop.outward_bonus)
radius_factor = (ball_radius / 10.0).max(0.1)
applied_impulse = dir_from_centroid * magnitude_base * radius_factor
```
Optional spin:
```
if spin_jitter > 0:
    vel.angvel += random(-spin_jitter .. spin_jitter)
```

## Event
Emit:
```rust
ClusterPopped {
    color_index,
    ball_count,
    total_area,
    centroid,
}
```

## Despawn / Fade
- If `fade_enabled` (see dedicated fade prompt) balls transition via `PoppingBall` component (physics-active shrink/fade).
- Else:
  - If `despawn_delay <= 0`: despawn immediately.
  - Else: attach a lightweight timer component and despawn on expiry (or reuse fade structure with alpha/scale disabled).

## Validation Rules
Warn if:
- `min_ball_count < 1` (treated as 1).
- Any negative numeric field (treated as 0).
- `impulse <= 0` (no outward motion).
- `fade_enabled == false` but fade-related fields differ from defaults (ignored).
- `collider_min_scale` outside [0,1] (clamped).
- `fade_duration < 0.05` when enabled (raised to 0.05).

## System Overview
- `handle_tap_cluster_pop` (in `PrePhysicsSet`) handles selection, qualification, impulse application, fade/despawn scheduling, and event emission.
- `update_popping_balls` (after `PrePhysicsSet`) progresses fades and final despawns (if fade path active).

## Non‑Goals
- Reintroducing explosion or drag mechanics.
- Scoring / combo logic.
- Particle or audio FX (subscribe to event later).

## Edge Cases
| Case | Behavior |
|------|----------|
| Tap empty space | No effect (no fallback explosion). |
| Multiple overlapping clusters qualify | Largest `ball_count` wins; tie by centroid distance. |
| Config impulse = 0 and outward_bonus = 0 | Pop has no outward motion (warn). |
| Very small cluster below thresholds | Ignored. |
| fade_disabled, despawn_delay > 0 | Simple delayed despawn (or treat as minimal fade if using unified component). |

## Future Extensions
- Particle / sound FX on `ClusterPopped`.
- Screen shake scaled by `total_area`.
- Score & combo systems.
- Highlight prospective cluster under pointer hover.

## Summary
Cluster pop is the single tap interaction: select cluster, apply an outward impulse using an absolute `cluster_pop.impulse`, and remove its entities (immediately or via fade). No generic explosion fallback remains.

---
Updated after removal of legacy Explosion & Drag interactions.
