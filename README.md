# Ball Matcher / Bouncing Balls Sandbox

Modular Bevy playground (Bevy 0.14 / Rapier 2D) showcasing a clean, extensible plugin layout + RON-driven configuration. Spawns and continually emits colored balls that bounce within an arena with optional contact-based separation to reduce visual overlap.

## Features
- Modular plugin architecture (all aggregated by `GamePlugin`)
	- `camera`: 2D camera setup
	- `spawn`: initial batch spawning
	- `emitter`: timed continual spawning until coverage threshold
	- `materials`: defines a 6-color palette + simple physics restitution variants
	- `rapier_physics`: Rapier integration + dynamic arena walls + gravity
	- `separation`: optional contact-based positional correction & velocity damping
- External RON config (`assets/config/game.ron`) for window, gravity, bounce, spawn, and separation tuning
- Randomized radius, color, position, and velocity (initial & emitted)
- Rapier 2D gravity + elastic wall collisions (configurable restitution)
- Optional Rapier debug physics overlay (enable via config `rapier_debug: true`)
- Shared unit circle mesh reused for all balls (reduced GPU/CPU overhead; can hide via `draw_circles: false`)
 - Optional cluster AABB gizmo rendering (enable via `draw_cluster_bounds: true`)
- Clear radius semantics: `BallRadius` equals collider radius (render scale = `radius * 2.0`)
- Extensible: drop-in new plugins; config-first for tunables

## Requirements
- Latest stable Rust (Bevy MSRV tracks latest stable)  
- Windows: Make sure you have the Microsoft C++ Build Tools / Visual Studio C++ workload installed.

## Run
```powershell
cargo run
```

## Configuration (RON)
Tweak values in `assets/config/game.ron`:
```ron
(
	window: (width: 1280.0, height: 720.0, title: "Bevy Bouncing Balls"),
	gravity: (y: -600.0),
	bounce: (restitution: 0.85),
	balls: (
		count: 150,
		radius_range: (min: 5.0, max: 25.0),
		x_range: (min: -576.0, max: 576.0),
		y_range: (min: -324.0, max: 324.0),
		vel_x_range: (min: -200.0, max: 200.0),
		vel_y_range: (min: -50.0, max: 350.0),
    ),
	separation: (enabled: true, overlap_slop: 0.98, push_strength: 0.5, max_push: 10.0, velocity_dampen: 0.2),
	rapier_debug: false,          // set true to visualize physics colliders / velocities
	draw_circles: true,           // set false to hide per-ball circle meshes (still simulated & can rely on metaballs)
	metaballs_enabled: true,      // set false to skip metaball overlay updates
	draw_cluster_bounds: false,   // set true to draw per-cluster bounding boxes (gizmos)
)
```
Restart the app after edits (simple explicit reload for now). An asset-loader based hot reload could be added later.

## Code Structure
```
src/
	main.rs          # Minimal: load config, set window, add GamePlugin
	config.rs        # GameConfig + RON deserialization
	components.rs    # Components (Ball)
	camera.rs        # CameraPlugin
	spawn.rs         # BallSpawnPlugin (startup batch) + reusable spawn helper
	emitter.rs       # BallEmitterPlugin (time-based emission until area coverage)
	separation.rs    # SeparationPlugin (optional contact overlap mitigation)
	rapier_physics.rs# RapierPhysicsPlugin (gravity, dynamic resizing arena walls)
	game.rs          # Aggregates sub-plugins
assets/config/game.ron  # Runtime configuration
```

### Component Set
- `Ball` – tag component (parent entity).
- `BallRadius(f32)` – logical & collider radius (mesh scaled to diameter). 

### System Flow / Ordering
Custom system sets make ordering explicit:
```
PrePhysicsSet        # (future) manual forces / tweaks before Rapier step
Rapier (plugin)      # physics integration
PostPhysicsAdjustSet # separation corrections after contact events
```
1. Startup: `camera` spawns camera; `spawn` creates initial batch; `rapier_physics` sets gravity & arena walls.
2. Update: Rapier advances the world; `emitter` periodically adds balls (until coverage cap); `separation` (in `PostPhysicsAdjustSet`) applies gentle overlap correction; resize system rebuilds walls on window events.

## Coordinate System
(0,0) is window center. Bounds recomputed every frame from the primary window (so resize works).

## Performance / Build Notes
Bevy on Windows can hit the MSVC linker object limit when using the `dynamic_linking` feature. We removed it here after encountering LNK1189. If you want faster incremental builds and are not hitting the limit you can try enabling it again:
```toml
bevy = { version = "0.16", features = ["dynamic_linking"] }
```
If you see `LNK1189: library limit of 65535 objects exceeded`, remove that feature.

### Faster Compiles (Optional)
You can add to Cargo.toml profiles (already partially enabled):
```toml
[profile.dev]
opt-level = 1
```
Other common options (use if desired):
```toml
[profile.dev.package."*"]
opt-level = 0
```

### Release Build
```powershell
cargo run --release
```

## Possible Extensions / Next Steps
- Palette: adjust colors or physics restitution by editing `materials.rs` (extend to load from RON/asset in future)
- Configure Rapier integration parameters (substeps, CCD) for high-speed stability
- Add per-ball mass / density variations
- Introduce game states / scenes (loading screen, menu, simulation)
- Ball spawning/despawning via input
- Asset-driven color palettes or texture mapping
- Hot-reload config via `AssetServer::watch_for_changes`
- Diagnostics overlay (FPS, entity count)
- Spatial partitioned queries for gameplay rules

## Collision Separation / Overlap Prevention
Real-time overlap mitigation (optional) is enabled via the `SeparationPlugin`, listening to Rapier `CollisionEvent::Started` events (so only actual contacts generate work). On contact:

1. Compute target distance = (r1 + r2) * `overlap_slop`.
2. If centers are closer than target, compute overlap amount and push each body half the corrective distance along the normal (clamped by `max_push` and scaled by `push_strength`).
3. Optionally damp the velocity component along the normal by `velocity_dampen` (fraction removed).

Config block (in `assets/config/game.ron`):
```ron
separation: (
	enabled: true,
	overlap_slop: 0.98,     // <1.0: pre-emptive push before visible overlap; >1.0: allow a little penetration
	push_strength: 0.5,     // fraction of overlap resolved per contact event
	max_push: 10.0,         // positional correction clamp to avoid tunneling / jitter
	velocity_dampen: 0.2,   // 0..1 fraction of normal velocity removed (stability)
),
```

Tuning tips:
- Persistent overlap: lower `overlap_slop` (e.g. 0.95) or raise `push_strength`.
- Shaky / jittery motion: reduce `push_strength` or `max_push`, or increase `velocity_dampen` modestly.
- Balls sticking together: `velocity_dampen` may be too high; lower it.
- Performance (many contacts): system iterates only over collision events, so scale is roughly proportional to actual contacts, not O(n^2). If needed, batch multiple pushes or use accumulated contact manifolds.

Disable by setting `enabled: false` for A/B comparison vs pure Rapier response.

## Metaball Cluster Visualization (Shader-Based)

A full-screen WGSL pass blends physically simulated balls (not meshes) into smooth metaball blobs for each cluster of touching same-color balls.

Current implementation highlights:
- Per-ball field evaluation using a bounded Wyvill kernel f(d) = (1 - (d/R)^2)^3 (d <= R)
- Analytic gradient for lighting-like shading (pseudo normal, adjustable Z scale via `normal_z_scale`)
- Iso-surface threshold (`iso`) calibrated so visible contour matches physical collider radius (CPU-computed `radius_scale`)
- Cluster color blending (first matching cluster per ball) with soft additive mix
- Anti-aliased edge based on signed field difference to iso
- Uniform packing limits: MAX_BALLS=1024, MAX_CLUSTERS=256 (adjust in code + shader together)

Toggles / Controls:
- `metaballs_enabled` (config) initializes `MetaballsToggle` resource
- `MetaballsToggle(false)` at runtime halts material updates (cheap pause)
- Disable circle meshes with `draw_circles: false` to view pure metaball surface
- Use `rapier_debug: true` alongside metaballs to inspect collider alignment

Performance Considerations:
- Fragment cost: O(N) with N = visible ball count (capped). Large windows + many balls increase cost; consider dynamic LOD or tile culling if scaling further.
- Memory: Single uniform struct (<64KB) updated each frame (positions, radii, indices, colors).

Extending:
- Add per-ball color weighting
- Introduce thickness visualization (store field strength in alpha)
- Depth layering with 3D illusions using parallax normals
- Compute clusters GPU-side with compute shader and indirect draws (future).

## License
MIT (adjust as you wish).
