# Copilot Project Instructions

Purpose: 2D Bevy (0.16) sandbox demonstrating modular plugin structure + RON-driven config that spawns and simulates many bouncing balls.

## Architecture / Key Modules
- `main.rs`: Loads `GameConfig` from `assets/config/game.ron`, configures the primary window (size/title dynamic from config), then adds `GamePlugin`.
- `game.rs` (`GamePlugin`): Aggregates sub-plugins in one place. Extend by adding new plugins here (keep ordering explicit).
- `config.rs`: Strongly typed `GameConfig` (serde + RON). All tunables (window, gravity, bounce, spawn ranges). Loading returns `Result<String>` errors; mimic pattern for new config sections.
- `components.rs`: Minimal ECS data: `Ball` tag + `Velocity(Vec2)` newtype (uses `Deref`/`DerefMut` for ergonomic `.x/.y`). Add new per-ball data here or create new component files if domain grows.
- `spawn.rs` (`BallSpawnPlugin`): Startup-only system `spawn_balls` that creates: shared circle mesh (radius 0.5, scaled per-entity to diameter via `Transform::scale`), randomized Transform, Velocity, Material. Reuses one mesh handle; add new geometry by following same pattern (one mesh, multiple entities).
- `physics.rs` (`PhysicsPlugin`): Update-stage systems: gravity integration, translation update, window-bound collision w/ restitution + velocity damping threshold (`length_squared() < 1.0` -> zero-out). Bounds recalculated every frame from primary window (resizes honored automatically).
- `camera.rs` (`CameraPlugin`): Startup camera spawn (`Camera2d`).

## Data / Flow Summary
Startup: load config -> insert resource -> add plugins -> `spawn_balls` & `setup_camera` run.
Per-frame (Update): `apply_gravity` -> `move_balls` -> `bounce_on_bounds`. System ordering relies on tuple insertion order; if adding systems that depend on updated positions, append after `move_balls` (before bounce if they need pre-bounce positions).

## Conventions / Patterns
- All new functionality should be a small focused `Plugin` in its own file, then registered inside `GamePlugin`.
- Use RON + serde for user-tunable parameters; extend `GameConfig` instead of hardcoding constants. Remember to update both the struct and `assets/config/game.ron` example.
- Reuse shared meshes/material patterns: create once, clone handles when spawning many entities.
- Coordinate system: (0,0) centered window. Compute dynamic bounds each frame from `Window` for resize support.
- Velocity integration is manual (no physics engine). Add forces by mutating `Velocity` before `move_balls` runs.

## Extensibility Tips
- Adding ball-ball collisions: create new system after `move_balls` but before `bounce_on_bounds` (or after, depending on desired order), maybe stage with `SystemSet::after` labels if ordering gets complex.
- Adding new spawn variants (e.g., different shapes): duplicate mesh generation, maybe refactor spawn to iterate config-driven shape list.
- Config hot-reload: implement a system using `AssetServer::watch_for_changes` and reapply resource values cautiously (window changes via `Window` mut).

## Build & Run
- Dev: `cargo run` (uses `[profile.dev] opt-level=1`).
- Release: `cargo run --release` (thin LTO, stripped, small size).
- If linker error `LNK1189` on Windows appears, ensure `bevy` dependency does NOT enable `dynamic_linking`.

## Testing
- Current tests: only config parsing (`config.rs`). If adding config structs, mirror test style with inline RON sample + assertions.

## Common Pitfalls
- Forgetting to register new plugin inside `GamePlugin` -> systems never run.
- Scaling: radius is half of scale.x (entity scaled to diameter). When computing collisions with walls or other entities, derive radius as `transform.scale.x * 0.5`.
- Ensure system ordering when adding new motion-altering systems (gravity -> modify velocity, then movement, then collision). Use explicit `in_schedule` or system ordering if tuple order insufficient.

## Examples
Spawn custom component:
```rust
commands.spawn((Ball, Velocity(Vec2::ZERO), MyNewComponent));
```
Add new plugin in `game.rs`:
```rust
app.add_plugins((CameraPlugin, BallSpawnPlugin, PhysicsPlugin, MyNewPlugin));
```
Extend config:
```rust
#[derive(Debug, Deserialize, Clone)]
pub struct WindConfig { pub x: f32 }
// Add to GameConfig, RON file, and apply inside a new system.
```

Keep instructions concise; reflect real patterns. Update this file when adding modules, config fields, or system ordering constraints.
