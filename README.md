# Ball Matcher / Bouncing Balls Sandbox

Modular Bevy 0.14 playground showcasing a clean plugin + RON configuration layout. Uses Rapier 2D physics for gravity and wall collisions. Spawns many colored balls that bounce within the window bounds.

## Features
- Modular plugin architecture (`camera`, `spawn`, `rapier_physics`, aggregated by `GamePlugin`)
- External `RON` configuration (`assets/config/game.ron`) for window, gravity, bounce, and spawn ranges
- Randomized radius, color, position, and initial velocity per ball
- Rapier 2D gravity + elastic wall collisions (configurable restitution)
- Debug physics overlay via `bevy_rapier2d` (enabled by default; can be disabled by removing `debug-render-2d` feature)
- Shared circle mesh reused by all entities
- Ready for extension into scenes / state machines

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
	spawn.rs         # BallSpawnPlugin (Startup spawning) + Rapier body/collider setup
	rapier_physics.rs# RapierPhysicsPlugin (gravity, dynamic resizing arena walls)
	game.rs          # Aggregates sub-plugins
assets/config/game.ron  # Runtime configuration
```

### Component Set
- `Ball` – tag component (Rapier supplies `Velocity`, `RigidBody`, `Collider`, etc.)

### System Flow
1. Startup: `CameraPlugin` spawns camera; `BallSpawnPlugin` spawns balls with Rapier dynamic bodies & circle colliders; `GameRapierPlugin` sets gravity & creates arena walls.
2. Update: Rapier steps simulation automatically; resize system rebuilds walls if window size changes.

## Coordinate System
(0,0) is window center. Bounds recomputed every frame from the primary window (so resize works).

## Performance / Build Notes
Bevy on Windows can hit the MSVC linker object limit when using the `dynamic_linking` feature. We removed it here after encountering LNK1189. If you want faster incremental builds and are not hitting the limit you can try enabling it again (update the version if you upgrade Bevy):
```toml
bevy = { version = "0.14", features = ["dynamic_linking"] }
```
If you see `LNK1189: library limit of 65535 objects exceeded`, remove that feature.

### Bevy / Rapier Version Alignment
`bevy_rapier2d 0.27` targets Bevy 0.14. If you upgrade Bevy, be sure to select a matching `bevy_rapier2d` release to avoid multiple Bevy versions in the dependency graph (which produced the earlier compile errors).

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
- Configure Rapier integration parameters (substeps, CCD) for high-speed stability
- Add per-ball mass / density variations
- Introduce game states / scenes (loading screen, menu, simulation)
- Ball spawning/despawning via input
- Asset-driven color palettes or texture mapping
- Hot-reload config via `AssetServer::watch_for_changes`
- Diagnostics overlay (FPS, entity count)
- Spatial partitioned queries for gameplay rules

## License
MIT (adjust as you wish).
