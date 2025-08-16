# Ball Matcher / Bouncing Balls Sandbox

Modular Bevy 0.16 playground showcasing a clean plugin + RON configuration layout. Spawns many colored balls that bounce within the window bounds.

## Features
- Modular plugin architecture (`camera`, `spawn`, `physics`, aggregated by `GamePlugin`)
- External `RON` configuration (`assets/config/game.ron`) for window, gravity, bounce, and spawn ranges
- Randomized radius, color, position, and velocity per ball
- Simple gravity + elastic wall collisions (configurable restitution)
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
	components.rs    # Components (Ball, Velocity)
	camera.rs        # CameraPlugin
	spawn.rs         # BallSpawnPlugin (Startup spawning)
	physics.rs       # PhysicsPlugin (gravity, movement, bounce)
	game.rs          # Aggregates sub-plugins
assets/config/game.ron  # Runtime configuration
```

### Component Set
- `Ball` – tag component
- `Velocity(Vec2)` – linear velocity integrated manually

### System Flow
1. Startup: `CameraPlugin` spawns camera; `BallSpawnPlugin` spawns balls from config
2. Update: `PhysicsPlugin` applies gravity → integrates position → handles boundary bounce

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
- Add ball-ball collision broadphase & narrowphase
- Introduce game states / scenes (loading screen, menu, simulation)
- Asset-driven color palettes or texture mapping
- Hot-reload config via `AssetServer::watch_for_changes`
- Diagnostics overlay (FPS, entity count)
- Input handling to spawn / remove balls interactively

## License
MIT (adjust as you wish).
