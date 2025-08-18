# Copilot Project Instructions

Purpose: 2D Bevy (0.16) sandbox demonstrating modular plugin structure + RON-driven config that spawns and simulates many bouncing balls.

## Architecture / Key Modules
- `main.rs`: Loads `GameConfig` from `assets/config/game.ron`, configures the primary window (size/title dynamic from config), then adds `GamePlugin`. Supports optional timed auto-exit via `window.autoClose` (seconds; 0 = disabled).
- `game.rs` (`GamePlugin`): Aggregates sub-plugins in one place. Extend by adding new plugins here (keep ordering explicit). Includes `AutoClosePlugin` which requests app exit once the configured timer elapses.
- `config.rs`: Strongly typed `GameConfig` (serde + RON). All tunables (window, gravity, bounce, spawn ranges). Loading returns `Result<String>` errors; mimic pattern for new config sections. New field: `window.autoClose` (f32) auto-closes after N seconds (default/<=0 disables).
	* `interactions` sub-struct: `explosion` (enabled, impulse, radius, falloff_exp) & `drag` (enabled, grab_radius, pull_strength, max_speed) configure pointer gestures.
- `components.rs`: Minimal ECS data: `Ball` tag + `Velocity(Vec2)` newtype (uses `Deref`/`DerefMut` for ergonomic `.x/.y`). Add new per-ball data here or create new component files if domain grows.
- `spawn.rs` (`BallSpawnPlugin`): Startup-only system `spawn_balls` that creates: shared circle mesh (radius 0.5, scaled per-entity to diameter via `Transform::scale`), randomized Transform, Velocity, Material. Reuses one mesh handle; add new geometry by following same pattern (one mesh, multiple entities).
- `physics.rs` (`PhysicsPlugin`): Update-stage systems: gravity integration, translation update, window-bound collision w/ restitution + velocity damping threshold (`length_squared() < 1.0` -> zero-out). Bounds recalculated every frame from primary window (resizes honored automatically).
- `camera.rs` (`CameraPlugin`): Startup camera spawn (`Camera2d`).
 - `background.rs` (`BackgroundPlugin`): Renders a full-screen world-space grid (shader `bg_worldgrid.wgsl`) as the implicit background. The camera uses `ClearColorConfig::None`; this quad is drawn first (z=-100) instead of clearing.
- `cluster.rs` (`ClusterPlugin`): Recomputes per-frame connected components ("clusters") of touching same-color balls using spatial hashing + union-find; exposes `Clusters` resource and draws debug AABBs with gizmos (used later for metaball aggregation).
 - `metaballs.rs` (`MetaballsPlugin` + WGSL in `assets/shaders/metaballs.wgsl`): Fullscreen post-style pass rendering true metaballs from individual ball positions & radii via a bounded Wyvill kernel. Packs per-ball data into a uniform buffer (single draw) and analytically derives normals for simple lighting.
 - `fluid_sim.rs` (`FluidSimPlugin` + WGSL in `assets/shaders/fluid_sim.wgsl`): 2D stable-fluids style simulation on a fixed grid (default 256x256). Compute passes (add_force, advect_velocity, compute_divergence, jacobi_pressure (N), project_velocity, advect_dye) run in the render world using storage textures. A fullscreen quad material displays the dye texture. Resolution & solver params are hot-reloadable via `GameConfig.fluid_sim`. Ping-pong strategy keeps front (*_a) textures stable for sampling.
 - `debug` (feature gated): Runtime stats overlay (top-left) plus bottom-left config snapshot (current `GameConfig` summary). Toggle visibility with `F1`.
 - Metaballs shading supports PBR-ish highlights and selectable color blending:
 	 * Config section `metaballs` in `game.ron` defines: `iso`, `normal_z_scale`, `metallic`, `roughness`, `env_intensity`, `spec_intensity`, `hard_cluster_boundaries` (bool), `color_blend_exponent` (f32).
 	 * When `hard_cluster_boundaries` = true: nearest contributing ball/cluster defines color (bubble look).
 	 * When false: colors smoothly blend weighted by field contribution^`color_blend_exponent` (1.0 = linear, >1 tightens local color, <1 (not recommended <0.5) increases wash).
 	 * Runtime `MetaballsParams` mirrors these and can still be adjusted by keys.
 	 * Spherical-ish normals: mixes field gradient with reconstructed sphere normal for more convex highlight rolloff.
 	 * Keys: `[`/`]` iso, `M` metallic toggle extremes, `-`/`=` roughness, `E` env intensity toggle, `P` specular toggle (color blending mode currently driven via config only).

## Data / Flow Summary
Startup: load config -> insert resource -> add plugins -> `spawn_balls` & `setup_camera` run.
Per-frame (Update simplified): Pre-physics forces (`apply_radial_gravity`, tap explosion, drag pull) in `PrePhysicsSet` -> Rapier -> post adjustments (separation) -> rendering. Ensure new velocity-modifying systems join `PrePhysicsSet`.

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
 - Metaballs: To tweak look, adjust `MetaballsParams` (iso threshold & normal_z_scale). For more advanced shading (specular, environment), extend WGSL; keep array packing (Vec4) for alignment.
 - Pointer interactions (`input_interaction.rs`):
	 * Tap: outward impulse to nearby balls using configurable falloff.
	 * Drag: continuous pull acceleration toward pointer with optional speed cap.
	 * Extend by adding new fields to `InteractionConfig` + systems in the plugin.

## Build & Run
- Dev: `cargo run` (uses `[profile.dev] opt-level=1`).
- Release: `cargo run --release` (thin LTO, stripped, small size).
- If linker error `LNK1189` on Windows appears, ensure `bevy` dependency does NOT enable `dynamic_linking`.

## Testing
- Config parsing test in `config.rs`.
- Separation plugin placeholder (ignored) test scaffold.
- Cluster tests (`cluster.rs`): singleton, chain merge, different-color separation.
Add new system tests by spinning up a minimal `App`, inserting needed resources/components, running `app.update()`, and asserting over world state/resources.

## Common Pitfalls
- Forgetting to register new plugin inside `GamePlugin` -> systems never run.
- Scaling: radius is half of scale.x (entity scaled to diameter). When computing collisions with walls or other entities, derive radius as `transform.scale.x * 0.5`.
- Ensure system ordering when adding new motion-altering systems (gravity -> modify velocity, then movement, then collision). Use explicit sets / ordering if tuple order insufficient.
- Clusters recompute every frame; avoid heavy per-entity allocations inside the loop (reuse vectors, keep algorithm near O(n)). If extending clustering data, prefer amortized allocations.

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

### Metaballs Notes
The metaball overlay is purely shader-based:
* Uniform layout packs up to 1024 balls (Vec4: x,y,radius,cluster_index) and a color table of up to 256 cluster colors.
* Kernel: `f = (1 - (d/R)^2)^3` for `d < R`, else 0. Field & gradient accumulated for all balls.
* Iso-surface threshold (`iso`), pseudo-normal Z scale (`normal_z_scale`), plus metallic shading params: `metallic` (0-1), `roughness` (0.04-1), `env_intensity`, `spec_intensity`. Defaults currently: metallic=0.5, roughness=0.5, env_intensity=0.0, spec_intensity=0.5.
* Color mode: choose between smooth blending (default) or hard boundaries (see config `hard_cluster_boundaries`).
* Lighting: GGX-like specular + simple hemi environment blend; environment & spec scaled by params.
* Anti-aliasing: edge mask computed from signed distance approximation `(field - iso)/|grad|` with a derivative-based smoothing band.
* Lighting: simple lambert + hemisphere; modify `metaballs.wgsl` to customize.
* To disable/enable at runtime, toggle `MetaballsToggle(bool)` resource.

Add a system to change parameters (example):
```rust
fn tweak_metaballs(mut params: ResMut<MetaballsParams>, keys: Res<Input<KeyCode>>) {
	if keys.just_pressed(KeyCode::BracketLeft) { params.iso = (params.iso - 0.05).max(0.2); }
	if keys.just_pressed(KeyCode::BracketRight) { params.iso = (params.iso + 0.05).min(1.5); }
}
```
Register after the plugin so it runs each frame.


Keep instructions concise; reflect real patterns. Update this file when adding modules, config fields, or system ordering constraints.
