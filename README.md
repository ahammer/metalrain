# Bevy Bouncing Balls Harness

A minimal Bevy 0.16 playground that spawns a bunch of colored balls which bounce inside the window bounds.

## Features
- Randomized radius, color, initial position and velocity
- Simple gravity and elastic collisions with window edges (configurable restitution)
- Single shared circle mesh for all balls (instancing friendly)
- Easily tweak constants at top of `src/main.rs`

## Requirements
- Latest stable Rust (Bevy MSRV tracks latest stable)  
- Windows: Make sure you have the Microsoft C++ Build Tools / Visual Studio C++ workload installed.

## Run
```powershell
cargo run
```

## Tweak Parameters
Edit the constants near the top of `src/main.rs`:
- `BALL_COUNT`
- `GRAVITY` (negative = downward)
- `RESTITUTION` (0-1 energy retained after bounce)
- `WINDOW_WIDTH` / `WINDOW_HEIGHT`

Hot reloading: Just stop and re-run (for faster iteration see Compile Speed section).

## How It Works
Each ball entity has:
- `Mesh2d` + `MeshMaterial2d` using a single unit circle mesh scaled per-entity
- `Transform` (translation + uniform scale = diameter)
- `Velocity(Vec2)` component updated each frame with gravity and wall bounce logic

Systems (Update schedule):
1. `apply_gravity` modifies velocities
2. `move_balls` integrates position
3. `bounce_on_bounds` clamps to window edges and flips velocity components with restitution

## Coordinate System
(0,0) is window center. Bounds are computed every frame from the actual window size so resizing still works.

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

## Possible Extensions
- Add simple ball-ball collision detection
- Display FPS counter (`FrameTimeDiagnosticsPlugin` + UI text)
- Spawn/despawn with mouse clicks

## License
MIT (adjust as you wish).
