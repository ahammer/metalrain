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
