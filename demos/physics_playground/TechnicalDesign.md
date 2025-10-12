# Physics Playground - Technical Design Document

**Demo Name:** `physics_playground`  
**Version:** 1.0  
**Last Updated:** October 2, 2025  
**Status:** Implementation Ready

---

## 1. Executive Summary

The Physics Playground is an interactive demonstration environment that showcases the complete integration of all metalrain game systems. It serves as the canonical reference implementation for how to compose `game_core`, `game_physics`, `metaball_renderer`, `game_rendering`, `event_core`, and `widget_renderer` into a cohesive, playable experience.

**Key Characteristics:**

- **Minimal Custom Code**: Almost all functionality comes from crate composition
- **Real-Time Configuration**: Physics parameters adjustable via UI without restart
- **Complete System Integration**: Every major crate is utilized and showcased
- **Visual Debugging**: Velocity vectors, performance metrics, and state visualization
- **Deterministic Behavior**: Event-driven architecture ensures reproducible results

---

## 2. Architecture Overview

### 2.1 System Composition

The playground achieves rich functionality through strategic plugin composition:

```
physics_playground (binary/library)
    ├── Bevy DefaultPlugins (windowing, rendering, input)
    ├── game_core::GameCorePlugin
    │   └── Core components, events, resources
    ├── game_physics::GamePhysicsPlugin
    │   └── Rapier2D integration, forces, constraints
    ├── metaball_renderer::MetaballRendererPlugin
    │   └── GPU compute rendering, coordinate mapping
    ├── game_rendering::GameRenderingPlugin
    │   └── Multi-layer compositor, camera management
    ├── event_core::EventCorePlugin
    │   └── Input pipeline, middleware, handlers
    ├── widget_renderer::WidgetRendererPlugin
    │   └── Entity visuals, animations
    └── background_renderer::BackgroundRendererPlugin
        └── Background rendering
```

### 2.2 Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ User Input (Mouse, Keyboard)                                 │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│ event_core Pipeline                                          │
│   KeyMapping → Debounce → Cooldown → EventQueue             │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│ Event Handlers (PostUpdate)                                  │
│   Spawn balls, Reset simulation, Toggle pause               │
└─────────────┬───────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────┐
│ ECS World Mutations                                          │
│   game_core components spawned/modified                      │
└─────────────┬───────────────────────────────────────────────┘
              │
        ┌─────┴─────┬─────────────┬──────────────┐
        ▼           ▼             ▼              ▼
  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌──────────────┐
  │ Physics │ │ Metaball │ │  Widget   │ │   Rendering  │
  │ System  │ │ Renderer │ │ Renderer  │ │  Compositor  │
  └────┬────┘ └────┬─────┘ └─────┬─────┘ └──────┬───────┘
       │           │             │               │
       └───────────┴─────────────┴───────────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │   Final Composited     │
              │   Frame to Screen      │
              └────────────────────────┘
```

### 2.3 Coordinate Space Management

The playground operates across multiple coordinate spaces:

| Space | Description | Example Bounds |
|-------|-------------|----------------|
| **World Space** | Game logic coordinates | (-256, -256) to (256, 256) |
| **Metaball Texture** | GPU compute target pixels | (0, 0) to (512, 512) |
| **Screen Space** | Window pixel coordinates | (0, 0) to (window_width, window_height) |
| **UV Space** | Texture sampling coordinates | (0.0, 0.0) to (1.0, 1.0) |

**Coordinate Mapper**: `MetaballCoordinateMapper` handles all transformations automatically based on configured world bounds.

---

## 3. Core Systems Design

### 3.1 Physics Simulation System

**Responsibility**: Realistic ball physics with clustering forces

**Key Components**:

```rust
// From game_core
Ball {
    velocity: Vec2,
    radius: f32,
    color: GameColor,
}

// Automatically added by game_physics
RigidBody::Dynamic
Collider::ball(radius)
Velocity::linear(velocity)
```

**Configuration Resource**:

```rust
PhysicsConfig {
    gravity: Vec2,              // Global gravity vector
    min_ball_speed: f32,        // Velocity floor
    max_ball_speed: f32,        // Velocity ceiling
    clustering_strength: f32,   // Attraction force multiplier
    clustering_radius: f32,     // Attraction effective range
    paddle_speed: f32,          // Paddle movement speed
    paddle_bounds: Rect,        // Paddle constraint area
}
```

**Systems Execution Order** (Update schedule):

1. `attach_paddle_kinematic_physics` - Convert paddles to kinematic bodies
2. `spawn_physics_for_new_balls` - Attach physics to new balls
3. `drive_paddle_velocity` - Input → paddle velocity
4. `apply_clustering_forces` - N² force calculation between nearby balls
5. `apply_config_gravity` - Apply configured gravity
6. Rapier physics step (Bevy Rapier plugin)
7. `sync_physics_to_balls` - Copy Rapier velocity → Ball.velocity
8. `clamp_velocities` - Enforce min/max speed limits
9. `clamp_paddle_positions` - Keep paddles in bounds

**Performance Characteristics**:

- Physics: O(n log n) via Rapier's spatial partitioning
- Clustering: O(n²) - consider disabling for >100 balls
- Velocity clamping: O(n)

### 3.2 Metaball Rendering System

**Responsibility**: GPU-accelerated blob visuals

**Pipeline Architecture**:

```
Compute Pass 1 (compute_metaballs.wgsl):
    Input: PackedMetaballData buffer
    Output: Field texture + Initial albedo texture
    
Compute Pass 2 (compute_3d_normals.wgsl):
    Input: Field texture
    Output: Normal-mapped albedo texture
    
Present Pass (present_fullscreen.wgsl):
    Input: Albedo texture
    Output: Rendered to RenderLayers::layer(2)
```

**Coordinate Mapping**:

```rust
// Game logic spawns in world space
commands.spawn((
    Transform::from_xyz(100.0, 50.0, 0.0),
    MetaBall { radius_world: 20.0 },
));

// Packing system (runs when Transform changes):
let tex_pos = mapper.world_to_metaball(transform.translation);
let tex_radius = mapper.world_radius_to_tex(metaball.radius_world);
// → Packed into GPU buffer
```

**Configuration**:

```rust
MetaballRenderSettings {
    world_bounds: Rect::from_center_size(Vec2::ZERO, Vec2::new(800.0, 600.0)),
    texture_width: 1920,
    texture_height: 1080,
    present_via_quad: true,
    presentation_layer: Some(2),
}
```

**Texture Outputs**:

- **Field Texture** (R16Float): Metaball field values for physics queries
- **Albedo Texture** (Rgba8UnormSrgb): Final rendered output with lighting

### 3.3 Multi-Layer Rendering System

**Responsibility**: Compositor blending of all visual layers

**Layer Assignment**:

| Layer | Content | Render Target | Blend Mode |
|-------|---------|---------------|------------|
| 0 | Background | `background_target` | Normal |
| 1 | GameWorld (widgets) | `game_world_target` | Normal |
| 2 | Metaballs | `metaball_target` | Additive |
| 3 | UI | `ui_target` | Normal |

**Compositor Shader** (`compositor.wgsl`):

```wgsl
// Pseudo-code representation
fn composite(uv: Vec2) -> Vec4 {
    let bg = sample(background_layer, uv);
    let game_world = sample(game_world_layer, uv);
    let metaballs = sample(metaball_layer, uv);
    let ui = sample(ui_layer, uv);
    
    var result = bg;
    result = blend(result, game_world, BlendMode::Normal);
    result = blend(result, metaballs, BlendMode::Additive);
    result = blend(result, ui, BlendMode::Normal);
    
    return result;
}
```

**Camera System**:

- **GameCamera** component tracks shake and zoom state
- **Derived Cameras**: Each layer gets its own camera derived from GameCamera
- **Effects**:
  - Shake: Procedural offset with decay
  - Zoom: Smooth interpolation to target scale

### 3.4 Event Processing System

**Responsibility**: Deterministic input handling and game state mutations

**Pipeline Stages**:

```
Raw Input → KeyMappingMiddleware → DebounceMiddleware 
    → CooldownMiddleware → EventQueue (frame N)
    → reducer_system (PostUpdate) → Event Handlers
    → ECS World Mutations
```

**Middleware Configuration**:

```rust
// Key mapping
KeyMappingMiddleware::with_default_gameplay()
    .map(KeyCode::Space, GameEvent::SpawnBall)
    .map(KeyCode::KeyR, GameEvent::ResetLevel)

// Debounce: ignore events within 3 frames of same event
DebounceMiddleware::new(3)

// Cooldown: minimum 5 frames between same event types
CooldownMiddleware::new(5)
```

**Frame Atomicity**:

- All events enqueued during frame N are processed in PostUpdate of frame N
- Handler-emitted events are deferred to frame N+1
- Eliminates reentrancy hazards and ensures deterministic ordering

**Event Handlers**:

```rust
impl EventHandler for BallSpawnHandler {
    fn handle(&self, envelope: &EventEnvelope, world: &mut World) {
        match &envelope.payload {
            EventPayload::Game(GameEvent::SpawnBall) => {
                // Spawn ball at cursor position
                world.spawn(BallBundle::new(...));
            }
            _ => {}
        }
    }
}
```

### 3.5 Widget Rendering System

**Responsibility**: Visual representation of non-ball entities

**Automatic Visual Spawning**:

```rust
// System: spawn_wall_visuals
Query<Entity, (With<Wall>, Without<Sprite>)>
// → Adds Sprite, RenderLayers::layer(1)

// System: spawn_target_visuals
Query<Entity, (With<Target>, Without<Sprite>)>
// → Adds Sprite + animation state
```

**Visual Specifications**:

| Entity Type | Shape | Color Logic | Animations |
|-------------|-------|-------------|------------|
| Wall | Rectangle | `Wall.color` | None |
| Target | Circle | `Target.color`, alpha based on health | Hit flash, destruction fade |
| Paddle | Rectangle | Cyan (0.1, 0.85, 0.95) | None |
| Hazard | Rectangle | Red, low alpha | Sinusoidal pulse |
| SpawnPoint | Dual circles | Yellow | Scale pulse when active |

**Animation Systems**:

- `update_target_animations`: Hit flash (1.2× scale, white tint, 1.0s) and destruction (1.4× scale, fade out, 0.5s)
- `update_hazard_pulse`: Alpha oscillates 0.1-0.5 at 2Hz
- `update_active_spawnpoint_pulse`: Scale oscillates 1.0-1.15 at 3.5Hz

---

## 4. Component Design

### 4.1 Entity Bundles

**BallBundle** (Primary gameplay entity):

```rust
pub struct BallBundle {
    pub ball: Ball,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

// Automatically augmented by systems:
// - game_physics adds: RigidBody, Collider, Velocity
// - metaball_renderer creates visual representation
// - Optional: widget_renderer can add velocity gizmo
```

**PaddleBundle** (Player/AI control):

```rust
pub struct PaddleBundle {
    pub paddle: Paddle,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

// Augmented by:
// - game_physics adds: RigidBody::KinematicPositionBased, Collider
// - widget_renderer adds: Sprite, RenderLayers::layer(1)
```

**TargetBundle** (Destructible objectives):

```rust
pub struct TargetBundle {
    pub target: Target,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

// Augmented by:
// - widget_renderer adds: Sprite with health-based opacity, animation state
```

### 4.2 Configuration Resources

**PhysicsConfig** (Runtime-mutable):

- Exposed to UI sliders for real-time tuning
- Changes take effect immediately (no restart required)
- Persists across scene reloads (optional save/load)

**GameState** (Global state):

```rust
pub struct GameState {
    pub score: u32,
    pub balls_remaining: u32,
    pub is_paused: bool,
}
```

**ArenaConfig** (Level definition):

```rust
pub struct ArenaConfig {
    pub bounds: Rect,
    pub spawn_cooldown: f32,
}
```

**MetaballRenderSettings** (Visual quality):

- World bounds: Must encompass all gameplay area
- Resolution: Trade-off between quality and performance
- Presentation layer: Integration with compositor

**CompositorSettings** (Layer toggling):

```rust
pub struct CompositorSettings {
    pub background_enabled: bool,
    pub game_world_enabled: bool,
    pub metaballs_enabled: bool,
    pub ui_enabled: bool,
    pub global_opacity: f32,
}
```

---

## 5. User Interface Design

### 5.1 Control Panel Layout

**Left Side Panel** (Physics Configuration):

```
┌──────────────────────────┐
│ Physics Configuration     │
├──────────────────────────┤
│ Gravity X:  [----●----]  │
│             -500 → +500   │
│                           │
│ Gravity Y:  [----●----]  │
│             -1000 → 0     │
│                           │
│ Restitution: [----●---]  │
│             0.0 → 1.0     │
│                           │
│ Friction:   [---●-----]  │
│             0.0 → 1.0     │
│                           │
│ Clustering  [-------●-]  │
│ Strength:   0 → 500       │
│                           │
│ Clustering  [----●----]  │
│ Radius:     20 → 200      │
│                           │
│ Max Ball    [-----●---]  │
│ Speed:      500 → 2000    │
│                           │
│ Min Ball    [--●------]  │
│ Speed:      0 → 200       │
└──────────────────────────┘
```

**Top Bar** (Metrics and State):

```
FPS: 60.0 | Balls: 23 | Targets: 5 | Score: 150 | [Paused]
```

**Bottom Bar** (Controls Help):

```
Left Click: Spawn Ball | R: Reset | P: Pause | Space: Primary Action | 1-4: Toggle Layers
```

### 5.2 Visual Feedback

**Velocity Gizmos**:

- Yellow lines originating from ball centers
- Length proportional to velocity magnitude
- Rendered in GameWorld layer (layer 1)
- Toggle with `V` key

**Performance Overlay**:

- FPS counter (smoothed over 60 frames)
- Entity counts by type
- Frame time breakdown (physics, rendering, events)
- Memory usage (optional)

**Layer Visualization**:

- Toggle individual layers with keys 1-4
- Visual indicator shows which layers are active
- Useful for debugging render order and blend modes

---

## 6. Input Mapping

### 6.1 Keyboard Controls

| Key | Event | Handler Behavior |
|-----|-------|------------------|
| **Mouse Left Click** | `GameEvent::SpawnBall` | Spawn ball at cursor with random velocity |
| **R** | `GameEvent::ResetLevel` | Clear all balls, reset score, respawn targets |
| **P** | `GameEvent::PauseGame` | Toggle `GameState.is_paused` |
| **Space** | `PlayerAction::PrimaryAction` | Context-dependent (e.g., launch ball from paddle) |
| **W/S** or **Up/Down** | `PlayerAction::MoveUp/Down` | Move paddle vertically |
| **A/D** or **Left/Right** | `PlayerAction::MoveLeft/Right` | Move paddle horizontally |
| **V** | Toggle velocity gizmos | Show/hide velocity visualization |
| **1-4** | Toggle layers | Show/hide individual render layers |
| **B** | Cycle blend modes | Iterate through blend mode options for metaballs layer |
| **Escape** | `GameEvent::PauseGame` | Pause and show menu |

### 6.2 Mouse Controls

| Action | Event | Behavior |
|--------|-------|----------|
| **Left Click** | `GameEvent::SpawnBall` | Spawn ball at cursor position |
| **Right Click** | (Optional) | Select entity under cursor |
| **Scroll Wheel** | `CameraZoomCommand` | Zoom camera in/out |
| **Middle Drag** | (Optional) | Pan camera |

### 6.3 UI Interactions

| Element | Interaction | Effect |
|---------|-------------|--------|
| **Sliders** | Drag or Click | Modify `PhysicsConfig` values immediately |
| **Checkboxes** | Click | Toggle boolean flags (e.g., clustering enabled) |
| **Buttons** | Click | Trigger one-shot events (reset, spawn preset, export config) |

---

## 7. Performance Optimization

### 7.1 Target Performance

**Desktop (Primary Target)**:

- 60 FPS with 100 balls
- 30 FPS with 500 balls
- 1920×1080 metaball texture resolution

**Mobile (Secondary Target)**:

- 60 FPS with 50 balls
- 30 FPS with 150 balls
- 1280×720 metaball texture resolution

### 7.2 Bottleneck Analysis

**Physics System**:

- Rapier2D: O(n log n) broad phase, O(contacts) narrow phase
- Clustering forces: O(n²) - disable for high ball counts
- Velocity clamping: O(n) - negligible cost

**Metaball Rendering**:

- Compute Pass 1: O(balls × pixels)
- Compute Pass 2: O(pixels)
- Texture resolution has largest impact
- Consider dynamic LOD scaling

**Widget Rendering**:

- Sprite batching by Bevy 2D renderer
- Minimal per-entity cost
- Animation systems: O(animated entities)

**Event System**:

- O(events per frame × middleware count)
- Negligible unless hundreds of events per frame
- Journal ring buffer prevents unbounded growth

### 7.3 Optimization Strategies

**Clustering Optimization**:

```rust
// For ball counts > 100, use spatial partitioning
if ball_count > 100 {
    config.clustering_strength = 0.0; // Disable
}
```

**Dynamic Metaball Resolution**:

```rust
// Scale texture resolution based on ball count
let resolution = match ball_count {
    0..=50 => (1920, 1080),
    51..=150 => (1280, 720),
    _ => (854, 480),
};
```

**Render Layer Culling**:

```rust
// Disable expensive layers when performance drops
if fps < 30.0 {
    compositor_settings.metaballs_enabled = false;
}
```

---

## 8. Testing Strategy

### 8.1 Integration Tests

**System Composition Test**:

- Verify all plugins load without errors
- Check resource initialization
- Validate system registration

**Coordinate Mapping Test**:

```rust
#[test]
fn world_to_metaball_round_trip() {
    let mapper = MetaballCoordinateMapper::new(world_bounds, tex_size);
    let world_pos = Vec3::new(100.0, 50.0, 0.0);
    let tex_pos = mapper.world_to_metaball(world_pos);
    let uv = mapper.metaball_to_uv(tex_pos);
    // Verify UVs are within [0, 1] and mapping is consistent
}
```

**Event Pipeline Test**:

```rust
#[test]
fn event_determinism() {
    // Replay same input sequence twice
    // Assert identical game state
}
```

### 8.2 Performance Tests

**Ball Spawning Stress Test**:

- Spawn 500 balls over 10 seconds
- Measure FPS degradation curve
- Identify performance cliffs

**Clustering Performance Test**:

- Compare frame times with/without clustering
- Measure O(n²) scaling empirically

**Metaball Resolution Test**:

- Render same scene at multiple resolutions
- Profile GPU compute time vs. resolution

### 8.3 Visual Regression Tests

**Snapshot Testing**:

```rust
#[test]
fn metaball_render_snapshot() {
    // Spawn fixed ball configuration
    // Render frame
    // Compare against reference screenshot
    // Assert pixel difference below threshold
}
```

**Layer Composition Test**:

- Enable/disable each layer individually
- Verify compositor blend modes produce expected output
- Check for Z-fighting or layering issues

### 8.4 User Interaction Tests

**Manual Test Protocol**:

1. Launch playground
2. Spawn 50 balls via left click
3. Adjust each slider and verify visual changes
4. Press R to reset - verify clean slate
5. Pause with P - verify physics stops
6. Toggle layers 1-4 - verify correct visibility
7. Check velocity gizmos with V
8. Test camera shake on collision
9. Verify no crashes after 5 minutes of interaction

---

## 9. Build and Deployment

### 9.1 Build Configurations

**Development Build**:

```bash
cargo run -p physics_playground
```

Features:

- Shader hot reload enabled
- Debug UI with detailed metrics
- Logging level: DEBUG
- Fast compile times (minimal optimizations)

**Release Build**:

```bash
cargo run -p physics_playground --release
```

Features:

- Full optimizations
- Shader hot reload disabled
- Logging level: WARN
- Smaller binary size

**WASM Build** (Web deployment):

```bash
cargo build -p physics_playground --target wasm32-unknown-unknown --release
wasm-bindgen --out-dir web/pkg --target web target/wasm32-unknown-unknown/release/physics_playground.wasm
```

Considerations:

- Disable Rapier parallel features
- Lower default texture resolutions
- Touch controls for mobile browsers
- Test on Safari, Chrome, Firefox

### 9.2 Asset Management

**Asset Directory Structure**:

```
assets/
├── fonts/
│   └── FiraMono-Medium.ttf
└── shaders/
    ├── background.wgsl
    ├── compositor.wgsl
    ├── compute_3d_normals.wgsl
    ├── compute_metaballs.wgsl
    └── present_fullscreen.wgsl
```

**Asset Loading**:

```rust
use game_assets::configure_demo;

let mut app = App::new();
configure_demo(&mut app); // Sets correct asset paths
```

Handles:

- Workspace root execution
- Demo directory execution
- Test execution
- WASM bundling

### 9.3 Cargo Configuration

**Cargo.toml**:

```toml
[package]
name = "physics_playground"
version = "0.1.0"
edition = "2021"

[lib]
name = "physics_playground"
path = "src/lib.rs"

[[bin]]
name = "physics_playground"
path = "src/main.rs"

[dependencies]
bevy = { workspace = true }
game_core = { path = "../../crates/game_core" }
game_physics = { path = "../../crates/game_physics" }
metaball_renderer = { path = "../../crates/metaball_renderer" }
game_rendering = { path = "../../crates/game_rendering" }
event_core = { path = "../../crates/event_core" }
widget_renderer = { path = "../../crates/widget_renderer" }
background_renderer = { path = "../../crates/background_renderer" }
game_assets = { path = "../../crates/game_assets" }
```

---

## 10. Implementation Plan

### Phase 1: Core Plugin Integration (Week 1)

**Tasks**:

1. Set up Cargo.toml with all crate dependencies
2. Create main.rs with plugin registration
3. Configure asset loading via game_assets
4. Add minimal startup system to spawn camera
5. Verify application launches and displays empty window

**Acceptance Criteria**:

- Application builds without errors
- Window opens with default background
- No runtime errors in console

### Phase 2: Physics System Integration (Week 1)

**Tasks**:

1. Add GamePhysicsPlugin with default configuration
2. Create mouse click handler to spawn balls
3. Verify balls fall under gravity
4. Add arena walls using WallBundle
5. Tune PhysicsConfig for desired feel

**Acceptance Criteria**:

- Balls spawn at cursor position on left click
- Balls bounce off walls realistically
- Gravity pulls balls downward
- No balls escape arena boundaries

### Phase 3: Metaball Rendering (Week 2)

**Tasks**:

1. Add MetaballRendererPlugin with configured world bounds
2. Ensure world bounds match arena size
3. Add MetaBall component to spawned balls
4. Configure presentation layer for compositor
5. Verify smooth blob rendering

**Acceptance Criteria**:

- Balls render as organic blobs
- Multiple balls merge visually when close
- No coordinate mapping errors
- Metaballs respect arena boundaries

### Phase 4: Multi-Layer Rendering (Week 2)

**Tasks**:

1. Add GameRenderingPlugin
2. Configure compositor settings
3. Assign widgets to layer 1, metaballs to layer 2
4. Add BackgroundRendererPlugin for layer 0
5. Verify layer blending and composition

**Acceptance Criteria**:

- Background renders on layer 0
- Widgets visible on layer 1
- Metaballs blend additively on layer 2
- Correct visual stacking order

### Phase 5: Event System Integration (Week 3)

**Tasks**:

1. Add EventCorePlugin with middleware chain
2. Register KeyMappingMiddleware for gameplay controls
3. Add DebounceMiddleware and CooldownMiddleware
4. Implement event handlers for spawn, reset, pause
5. Wire keyboard shortcuts to events

**Acceptance Criteria**:

- R key resets simulation
- P key pauses physics
- Space bar triggers primary action
- Events process deterministically

### Phase 6: UI Controls (Week 3)

**Tasks**:

1. Create UI panel with egui or custom Bevy UI
2. Add sliders for all PhysicsConfig parameters
3. Wire slider changes to resource mutations
4. Add performance overlay (FPS, entity counts)
5. Add control hints panel

**Acceptance Criteria**:

- Sliders modify physics in real-time
- UI is responsive and intuitive
- Performance metrics update every frame
- No UI lag or stuttering

### Phase 7: Visual Polish (Week 4)

**Tasks**:

1. Add velocity gizmo rendering system
2. Implement camera shake on ball collisions
3. Add layer toggle controls (keys 1-4)
4. Fine-tune colors and visual clarity
5. Add particle effects for destruction (optional)

**Acceptance Criteria**:

- Velocity vectors clearly visible
- Camera shake enhances impact feel
- Layer toggling works smoothly
- Visuals are polished and professional

### Phase 8: Testing and Optimization (Week 4)

**Tasks**:

1. Run performance profiling with 500 balls
2. Implement clustering optimization for high ball counts
3. Add dynamic resolution scaling
4. Write integration tests
5. Create manual test protocol document

**Acceptance Criteria**:

- 60 FPS with 100 balls on target hardware
- Graceful degradation at high ball counts
- All integration tests pass
- Manual test protocol completed successfully

---

## 11. Known Limitations and Future Work

### 11.1 Current Limitations

**Clustering Performance**:

- O(n²) algorithm becomes prohibitive above 100-150 balls
- No spatial partitioning or optimization
- Requires manual disabling via UI

**Metaball Resolution**:

- Fixed at startup based on configuration
- No dynamic LOD scaling
- Lower resolutions reduce visual quality noticeably

**Camera System**:

- No multi-camera support
- Limited to single viewport
- Pan/zoom controls are basic

**Event Replay**:

- Journal stores events but no replay UI
- No save/load of event sequences
- Debugging relies on manual inspection

### 11.2 Future Enhancements

**Preset Scenarios**:

```rust
enum Preset {
    Empty,           // Clean arena
    Maze,            // Complex wall arrangement
    Pinball,         // Targets and bumpers
    Orbital,         // Zero gravity, high clustering
}
```

**Recording and Playback**:

- Serialize event journal to JSON
- Replay sequences from file
- Side-by-side comparison mode

**Level Editor Mode**:

- Click-and-drag wall placement
- Target and hazard positioning
- Save/load arena configurations
- Export to JSON for use in actual game

**Advanced Diagnostics**:

- Real-time velocity distribution histogram
- Clustering density heatmap
- Frame time breakdown by system
- GPU profiling integration

**Performance Profiling UI**:

- In-app profiler with flame graph
- System execution times
- Memory allocation tracking
- GPU compute utilization

**Multi-Camera Support**:

- Split-screen mode for A/B parameter testing
- Picture-in-picture for layer visualization
- Follow-cam mode tracking specific ball

**Shader Live Reload**:

- Watch shader files for changes
- Hot-reload compute and present shaders
- Preserve simulation state during reload

---

## 12. Dependencies and Versions

### 12.1 External Crate Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `bevy` | Workspace | Core engine |
| `bevy_rapier2d` | Via game_physics | 2D physics simulation |
| `bytemuck` | Via metaball_renderer | Zero-copy buffer packing |

### 12.2 Internal Crate Dependencies

| Crate | Version | Integration Point |
|-------|---------|-------------------|
| `game_core` | Workspace | Core components, events |
| `game_physics` | Workspace | Physics simulation |
| `metaball_renderer` | Workspace | Blob rendering |
| `game_rendering` | Workspace | Multi-layer compositor |
| `event_core` | Workspace | Input and event pipeline |
| `widget_renderer` | Workspace | Entity visuals |
| `background_renderer` | Workspace | Background rendering |
| `game_assets` | Workspace | Asset management |

### 12.3 Feature Flags

**Enabled by Default**:

- `metaball_renderer/present` - Built-in presentation quad
- `metaball_renderer/shader_hot_reload` - Dev-time shader reloading

**Optional**:

- `event_core/serde` - Event serialization for replay
- `tracy` - Tracy profiler integration (via Bevy)

---

## 13. Success Metrics

### 13.1 Functional Requirements

✅ All crates integrate without errors  
✅ Physics parameters adjustable in real-time  
✅ Balls spawn on click with deterministic behavior  
✅ Metaballs render smoothly with organic merging  
✅ Multi-layer compositor produces correct visual output  
✅ Event system processes input deterministically  
✅ UI controls respond immediately  
✅ Reset/pause functionality works correctly  

### 13.2 Performance Requirements

✅ 60 FPS @ 100 balls (desktop)  
✅ 30 FPS @ 500 balls (desktop)  
✅ 60 FPS @ 50 balls (mobile target)  
✅ <16ms frame time under normal conditions  
✅ No memory leaks over 1 hour runtime  

### 13.3 User Experience Requirements

✅ Application launches in <3 seconds  
✅ Controls are intuitive without instruction  
✅ Visual feedback is immediate and clear  
✅ No perceptible lag when adjusting sliders  
✅ Crashes/errors do not occur during normal use  

### 13.4 Code Quality Requirements

✅ Playground code is <500 lines (excluding UI boilerplate)  
✅ No crate-specific business logic in playground  
✅ All systems come from plugin composition  
✅ Code is well-commented and serves as reference  
✅ Integration tests verify system interactions  

---

## 14. Conclusion

The Physics Playground demonstrates that **modularity works** - complex, feature-rich systems can emerge from simple, well-designed components. By composing `game_core`, `game_physics`, `metaball_renderer`, `game_rendering`, `event_core`, and `widget_renderer`, we achieve:

- **Zero duplication** - All logic lives in shared crates
- **Maximum reusability** - Other games/demos can use the same crates
- **Clear architecture** - Reading the playground code teaches the system
- **Rapid iteration** - Tweaking is immediate, rebuilds are fast

This design document serves as both a specification and a guide for implementation. Follow the phased approach, verify each phase's acceptance criteria, and the result will be a polished, performant, and instructive demonstration of the metalrain architecture.

**When someone asks "How do I use these crates together?"**, the answer is simple: **"Look at the Physics Playground."**

---

*Document Version: 1.0*  
*Last Updated: October 2, 2025*  
*Status: Ready for Implementation*
