# widget_renderer

Sprint 4/4.5 crate responsible for visualizing world elements (Walls, Targets, Hazards, Paddles, SpawnPoints).

## Responsibilities

- Provide `WidgetRendererPlugin` that spawns render primitives for `Wall`, `Target`, `Hazard`, `Paddle`, and `SpawnPoint` components defined in `game_core`.
- Keep visuals loosely coupled from physics: gameplay / physics spawn logical entities + colliders; this crate adds meshes & simple animations.
- Minimal effects (flat colored meshes) – glow / particles deferred.

## Systems

- `spawn_wall_visuals` – Adds a rectangle mesh for each new `Wall`.
- `spawn_target_visuals` – Adds a circle mesh & initial material for each new `Target`.
- `spawn_hazard_visuals` – Adds a pulsing rectangle for each new `Hazard`.
- `spawn_paddle_visuals` – Cyan rectangle sized from paddle half‑extents.
- `spawn_spawnpoint_visuals` – Inner disc + ring; active points pulse scale/alpha.
- (Kinematic paddles: physics handled in `game_physics`; this crate only supplies visuals.)
- `update_target_animations` – Handles hit & destroy scale / color flashes.
- `update_hazard_pulse` – Simple sinusoidal alpha pulse.
- `update_active_spawnpoint_pulse` – Scale + alpha modulation when active.
- `update_selected_highlight` – Tint newly selected entities.
- `sync_visuals_with_physics` – Placeholder hook for later transform reconciliation.

## Layering

Currently all visuals use `RenderLayers::layer(1)` (GameWorld). Integrate with a formal layering API in `game_rendering` in a later sprint.

## Future Extensions

- Glow / outline multi‑sprite walls
- Particle burst on target destruction
- Additional hazard types
- GPU instancing for large counts
