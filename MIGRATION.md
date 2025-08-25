# Migration Guide: Removal of Explosion & Drag Interactions

## Summary (BREAKING)
Explosion and Drag interactions have been removed. The only tap interaction is now Cluster Pop. A new absolute magnitude field `cluster_pop.impulse` replaces the former dependency on an explosion baseline (previous `explosion.impulse` + any `impulse_scale` multiplier).

## Required User Actions
1. Delete any `explosion: (...)` or `drag: (...)` blocks under `interactions` in your `game.ron`.
2. Ensure `interactions` contains only:
   ```
   interactions: (
       cluster_pop: (
           enabled: true,
           min_ball_count: 4,
           min_total_area: 1200.0,
           impulse: 500.0,
           outward_bonus: 0.6,
           despawn_delay: 0.0,
           aabb_pad: 4.0,
           tap_radius: 32.0,
           fade_enabled: true,
           fade_duration: 1.0,
           fade_scale_end: 0.0,
           fade_alpha: true,
           exclude_from_new_clusters: true,
           collider_shrink: false,
           collider_min_scale: 0.25,
           velocity_damping: 0.0,
           spin_jitter: 0.0,
       ),
   )
   ```
3. Remove any Explosion / Drag input action bindings from `assets/config/input.toml`. Only `PrimaryTap` is needed for cluster popping.
4. If you previously tuned an `impulse_scale`, convert your desired effective value into the absolute `impulse` directly (old: `explosion.impulse * cluster_pop.impulse_scale` → new: set `cluster_pop.impulse` to that product).

## Legacy Config Handling
Old config files containing `interactions.explosion` or `interactions.drag` keys are still loadable. They are ignored and a single warning is emitted:
```
Ignoring legacy interactions keys removed: explosion, drag
```
Remove those blocks to silence the warning.

## Rationale
- Simplifies interaction model.
- Eliminates dead maintenance surface (separate explosion & drag systems, resources, validation, prompts).
- Clarifies tuning: one direct impulse parameter.

## Validation Changes
`GameConfig::validate()` now checks:
- `cluster_pop.impulse` > 0 for outward motion (warns if ≤ 0).
- Field-specific clamps for fade & collider shrink parameters.

## Removed Symbols
Eliminated types / systems / labels (no longer present in codebase):
- `ExplosionConfig`
- `DragConfig`
- `ActiveDrag`
- `TapExplosionSet`
- `handle_tap_explosion` and all drag lifecycle systems

## Testing
A test ensures:
- `GameConfig::default().interactions.cluster_pop.impulse > 0`
- Legacy keys trigger the expected single warning string.

## Migration Example
Old (partial):
```
interactions: (
  explosion: ( impulse: 480.0, radius: 96.0 ),
  drag: ( enabled: false ),
  cluster_pop: ( enabled: true, min_ball_count: 4, impulse_scale: 1.1 )
)
```
New:
```
interactions: (
  cluster_pop: (
    enabled: true,
    min_ball_count: 4,
    min_total_area: 1200.0,
    impulse: 528.0, // 480.0 * 1.1
    outward_bonus: 0.6,
    ...
  ),
)
```

## Notes
All documentation & prompts now reflect the unified Cluster Pop model. Do not reintroduce removed concepts.

---

## Paddle Transform Cluster Pop (BREAKING)

Replaced impulse-based outward explosion + fade mechanic with a deterministic paddle transform animation (grow → optional hold → shrink → despawn). The tapped cluster now enlarges (colliders scale) and acts as a temporary stationary paddle that deflects other balls, then smoothly shrinks to zero.

### Added Fields (interactions.cluster_pop)
```
peak_scale: 1.8
grow_duration: 0.25
hold_duration: 0.10
shrink_duration: 0.40
collider_scale_curve: 1        # 0=linear,1=smoothstep,2=ease-out
freeze_mode: 0                  # 0=ZeroVelEachFrame,1=Kinematic,2=Fixed
fade_alpha: true                # (retained semantic, applied during SHRINK)
fade_curve: 1                   # curve for alpha
aabb_pad: 4.0                   # unchanged
tap_radius: 32.0                # unchanged
exclude_from_new_clusters: true # unchanged
```

### Removed / Legacy (now ignored with warning)
```
impulse
outward_bonus
velocity_damping
spin_jitter
collider_shrink
fade_scale_end
fade_duration
despawn_delay
collider_min_scale
```
Legacy fields are still deserializable (optional in config struct) and trigger a single validation warning:
```
Ignoring legacy cluster_pop fields: impulse, outward_bonus, ...
```

### New Component / Systems
- PaddleLifecycle { elapsed, grow/hold/shrink_duration, peak_scale, freeze_mode, base_radius, fade_alpha, fade_curve, collider_scale_curve }
- handle_tap_cluster_pop: now inserts PaddleLifecycle instead of impulses.
- update_paddle_lifecycle: runs pre-physics, scales visual + collider, freezes velocities, applies alpha fade on shrink, despawns at completion.

### Behavior Changes
- No outward impulses; balls in lifecycle remain spatially fixed (velocities zeroed each frame).
- Collider radius scales every frame to match animation for deterministic deflection.
- Alpha fades only during SHRINK (simpler readability).
- Deterministic (no RNG spin jitter).

### Validation Updates
Warnings added for:
- peak_scale <= 1.0 (subtle)
- peak_scale > 3.0 (performance risk)
- grow_duration <= 0 (clamped 0.01)
- shrink_duration <= 0 (clamped 0.05)
- hold_duration < 0 (treated 0)
- Unknown curve / freeze_mode codes (>2) -> treated as defaults
- Presence of any legacy fields (single aggregated warning)

### Migration Steps
1. Remove old fields from game.ron (or leave temporarily; they will be ignored with warning).
2. Add new required paddle fields (peak_scale + durations + curves + freeze_mode).
3. Re-tune peak_scale & durations for desired feel (e.g. shorter hold or no hold by setting hold_duration: 0.0).
4. Remove any gameplay logic or docs referring to impulses / outward_bonus / spin jitter.
5. Update tests (impulse-based assertions replaced with peak_scale and timing).

### Example Old → New (partial)
Old:
```
cluster_pop: (
  enabled: true,
  min_ball_count: 4,
  min_total_area: 1200.0,
  impulse: 500.0,
  outward_bonus: 0.6,
  fade_duration: 1.0,
  fade_scale_end: 0.0,
  fade_alpha: true,
)
```
New:
```
cluster_pop: (
  enabled: true,
  min_ball_count: 4,
  min_total_area: 1200.0,
  peak_scale: 1.8,
  grow_duration: 0.25,
  hold_duration: 0.10,
  shrink_duration: 0.40,
  collider_scale_curve: 1,
  freeze_mode: 0,
  fade_alpha: true,
  fade_curve: 1,
  aabb_pad: 4.0,
  tap_radius: 32.0,
  exclude_from_new_clusters: true,
)
```

### Rationale
- Predictable interaction surface improves skill expression.
- Eliminates chaotic outward impulse cluster fragmentation.
- Simplifies balance (time-phase + scale only).

### Testing Guidance
- Unit tests assert default peak_scale > 1.
- Phase midpoint scale correctness (grow mid > 1, end shrink ≈ 0).
- Validation emits legacy warning when old fields present.
- Visual QA: deflection occurs with enlarged collider, no translation drift.

### Performance
- Collider rebuild each frame; curve-based growth limited (warn >3.0 scale).
