# Color Fusion - North Star Technical Structure

## 1. Architecture Overview

### Core Philosophy
- **Modular Crate Architecture**: Separate concerns into focused, reusable crates
- **Demo-Driven Development**: Each major system has a corresponding demo for validation
- **Data-First Design**: Components and resources define behavior, systems orchestrate
- **Progressive Enhancement**: Core loop first, features layer additively

### Workspace Structure
```toml
[workspace]
members = [
    # Core game systems
    "crates/game_core",              # Core components, events, resources
    "crates/game_physics",           # Physics integration & clustering
    "crates/game_rendering",         # Rendering orchestration & coordination
    "crates/game_gameplay",          # Game rules, win/lose conditions
    "crates/game_levels",            # Level loading & management
    "crates/game_input",             # Input mapping & gesture recognition
    "crates/game_audio",             # Audio systems (future)
    
    # Rendering subsystems
    "crates/metaball_renderer",      # Metaball/blob rendering (existing)
    "crates/widget_renderer",        # In-world game elements (targets, walls)
    "crates/background_renderer",    # Environment & backdrop effects
    "crates/ui_renderer",            # HUD, menus, overlays
    
    # Integration layer
    "crates/game",                   # Main game plugin combining all systems
    
    # Demos & tests
    "demos/physics_playground",      # Physics tuning & validation
    "demos/rendering_test",          # Composite rendering experiments
    "demos/level_editor",            # Visual level creation tool
    "demos/input_test",             # Input system validation
]
```

## 2. Crate Specifications

### `game_core` - Shared Foundation
**Purpose**: Define all shared components, events, and resources used across systems

**Key Exports**:
```rust
// Components
pub struct Ball { pub velocity: Vec2, pub radius: f32, pub color: Color }
pub struct Wall { pub segments: Vec<LineSegment> }
pub struct Target { pub health: u8, pub color: Option<Color> }
pub struct Hazard { pub zone_type: HazardType }

// Resources  
pub struct GameState { pub balls_remaining: u32, pub targets_remaining: u32 }
pub struct ArenaConfig { pub bounds: Rect, pub gravity: Vec2 }

// Events
pub struct BallSpawned(Entity);
pub struct TargetDestroyed(Entity);
pub struct BallLost(Entity);
pub struct GameWon;
pub struct GameLost;
```

**Coupling Rules**:
- Zero dependencies on other game crates
- Only depends on `bevy` and standard library
- All types must be `Component`, `Resource`, or `Event` compatible

### `game_physics` - Physics Integration
**Purpose**: Handle Rapier integration, collision detection, clustering behavior

**Key Systems**:
- `sync_physics_to_balls`: Update Ball components from RigidBody
- `apply_clustering_forces`: Implement metaball attraction/repulsion
- `handle_collision_events`: Convert Rapier events to game events

**Dependencies**:
- `game_core`
- `bevy_rapier2d`

**Config Integration**:
```toml
[physics]
pixels_per_meter = 50.0
gravity = [0.0, -500.0]
ball_restitution = 0.95
ball_friction = 0.1
clustering_strength = 100.0
clustering_radius = 150.0
```

### `game_rendering` - Rendering Orchestration
**Purpose**: Coordinate all rendering subsystems, manage render layers, camera setup

**Key Systems**:
- `setup_render_layers`: Configure rendering order and compositing
- `manage_camera`: Handle viewport, zoom, shake effects
- `coordinate_renderers`: Ensure proper draw order between subsystems
- `screen_effects`: Global post-processing (flash, transitions)

**Render Layers**:
```rust
pub enum RenderLayer {
    Background = 0,  // Backdrops, environment
    GameWorld = 1,   // Walls, targets, hazards
    Metaballs = 2,   // Ball entities
    Effects = 3,     // Particles, temporary visuals
    UI = 4,          // HUD, menus
}
```

**Dependencies**:
- `game_core`
- `metaball_renderer`
- `widget_renderer`
- `background_renderer`
- `ui_renderer`

### `metaball_renderer` - Blob Rendering (Existing)
**Purpose**: GPU-accelerated metaball rendering for ball entities

**Key Components**:
```rust
pub struct Metaball {
    pub position: Vec2,
    pub radius: f32,
    pub color: Color,
}
```

**Integration Points**:
- Reads `Ball` components from `game_core`
- Renders to `RenderLayer::Metaballs`
- Provides shader hot-reload via existing architecture

### `widget_renderer` - Game Element Rendering
**Purpose**: Render in-world game objects (targets, walls, hazards)

**Key Systems**:
- `render_walls`: Draw collision boundaries with subtle glow
- `render_targets`: Fragile objects with hit animations
- `render_hazards`: Warning zones with animated edges
- `render_particles`: Target destruction effects

**Visual Style**:
```rust
pub struct WidgetStyle {
    pub wall_color: Color,
    pub wall_thickness: f32,
    pub target_base_color: Color,
    pub target_pulse_rate: f32,
    pub hazard_edge_color: Color,
    pub hazard_fill_alpha: f32,
}
```

**Dependencies**:
- `game_core`
- `bevy` render features

### `background_renderer` - Environment Rendering
**Purpose**: Dynamic backgrounds, environmental effects, arena atmosphere

**Key Features**:
- Gradient backgrounds with subtle animation
- Parallax layers for depth
- Ambient particle systems
- Arena boundary visualization

**Config**:
```toml
[background]
gradient_start = [0.05, 0.05, 0.1]
gradient_end = [0.02, 0.02, 0.05]
particle_count = 50
parallax_layers = 2
edge_glow_intensity = 0.3
```

**Systems**:
- `update_background_gradient`: Animate color shifts
- `update_parallax`: Respond to camera movement
- `spawn_ambient_particles`: Floating dust/stars

### `ui_renderer` - Interface Overlays
**Purpose**: HUD elements, menus, debug overlays

**Key Components**:
```rust
pub struct HudConfig {
    pub show_ball_count: bool,
    pub show_target_count: bool,
    pub position: HudPosition,
    pub style: HudStyle,
}
```

**UI Elements**:
- Ball counter (pips or number)
- Target counter
- Win/lose splash screens
- Restart prompt
- Debug overlay (FPS, physics visualization)

**Dependencies**:
- `game_core`
- `bevy_egui` (for debug UI only)

### `game_gameplay` - Game Rules
**Purpose**: Win/lose conditions, score tracking, game flow

**Key Systems**:
- `check_win_condition`: Monitor targets_remaining == 0
- `check_lose_condition`: Monitor balls_remaining == 0  
- `handle_target_destruction`: Process hits, update counters
- `handle_ball_elimination`: Process hazard contact

**Dependencies**:
- `game_core`

**State Machine**:
```rust
#[derive(States, Default)]
enum GamePhase {
    #[default]
    Setup,
    Playing,
    Won,
    Lost,
}
```

### `game_levels` - Level Management
**Purpose**: Load, parse, and instantiate level data

**Level Format** (TOML):
```toml
[meta]
name = "Tutorial Arena"
version = "1.0"

[arena]
width = 1280
height = 720

[background]
preset = "gradient_blue"
particles = true

[[balls]]
position = [640, 360]
velocity = [200, 150]
radius = 20
color = "blue"

[[targets]]
position = [100, 100]
health = 1

[[walls]]
points = [[0, 0], [1280, 0], [1280, 720], [0, 720], [0, 0]]

[[hazards]]
type = "pit"
bounds = { x = 500, y = 0, width = 280, height = 50 }
```

**Key Systems**:
- `load_level_file`: Parse TOML to entities
- `validate_level_data`: Ensure playability
- `spawn_level_entities`: Create game objects

### `game_input` - Input Handling
**Purpose**: Map raw input to game actions via configurable bindings

**Action Definitions**:
```rust
#[derive(ActionLike)]
enum GameAction {
    Restart,
    Pause,
    DebugToggle,
    // Future: LaunchBall, AimPaddle
}
```

**Config Format**:
```toml
[input.bindings]
restart = ["R", "Gamepad_North"]
pause = ["Escape", "P", "Gamepad_Start"]
debug_toggle = ["F3"]
```

### `game` - Integration Plugin
**Purpose**: Combine all systems into cohesive game experience

**Plugin Structure**:
```rust
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Core systems
            .add_plugins(GameCorePlugin)
            .add_plugins(GamePhysicsPlugin)
            .add_plugins(GameRenderingPlugin)
            .add_plugins(GameGameplayPlugin)
            .add_plugins(GameLevelsPlugin)
            .add_plugins(GameInputPlugin)
            
            // Rendering subsystems (via GameRenderingPlugin)
            .add_plugins(MetaballRendererPlugin)
            .add_plugins(WidgetRendererPlugin)
            .add_plugins(BackgroundRendererPlugin)
            .add_plugins(UiRendererPlugin)
            
            // System ordering
            .configure_sets(Update, (
                PhysicsSet.before(GameplaySet),
                GameplaySet.before(RenderingSet),
                RenderingSet.has_subsets((
                    BackgroundRenderSet,
                    WidgetRenderSet,
                    MetaballRenderSet,
                    UiRenderSet,
                )).chain(),
            ));
    }
}
```

## 3. Rendering Architecture

### Layer Composition Strategy
```rust
// Each renderer writes to its designated layer
// game_rendering crate handles composition

pub struct RenderPipeline {
    pub background: Handle<Image>,  // Background renderer output
    pub game_world: Handle<Image>,  // Widget renderer output
    pub metaballs: Handle<Image>,   // Metaball renderer output
    pub effects: Handle<Image>,     // Particle/effect output
    pub ui: Handle<Image>,          // UI overlay output
}

// Composition order (back to front):
// 1. Background (environment, gradients)
// 2. Game World (walls, targets, hazards)
// 3. Metaballs (balls with GPU blending)
// 4. Effects (particles, explosions)
// 5. UI (HUD, menus)
```

### Renderer Communication
```rust
// Shared render resources in game_rendering
pub struct RenderTargets {
    pub main: Handle<Image>,
    pub layers: HashMap<RenderLayer, Handle<Image>>,
}

// Each renderer subscribes to relevant components
// metaball_renderer: Query<&Ball>
// widget_renderer: Query<&Target>, Query<&Wall>, Query<&Hazard>
// background_renderer: Res<ArenaConfig>
// ui_renderer: Res<GameState>
```

## 4. Demo Specifications

### `physics_playground`
- Interactive ball spawning with mouse
- Real-time parameter tweaking (speed, restitution, gravity)
- Clustering force visualization
- Wall angle experiments

### `rendering_test`
- Test all four rendering layers simultaneously
- Layer toggle controls (show/hide each)
- Blend mode experiments
- Performance profiling with all renderers active

### `level_editor`
- Visual placement of walls, targets, hazards
- Background preset selection
- TOML export/import
- Playtest mode
- Grid snapping

### `input_test`
- Display active inputs
- Binding configuration UI
- Gesture recognition testing
- Gamepad support validation

## 5. Shared Models & Data Contracts

### Color System
```rust
// Predefined palette only (no mixing in MVP)
enum GameColor {
    Blue,
    Red,
    Yellow,
    Green,
    White,
}

// Rendering-specific color extensions
impl GameColor {
    pub fn to_metaball_color(&self) -> Vec4 { ... }
    pub fn to_widget_color(&self) -> Color { ... }
    pub fn to_ui_color(&self) -> egui::Color32 { ... }
}
```

### Physics Constants
```rust
const MIN_BALL_RADIUS: f32 = 15.0;
const MAX_BALL_RADIUS: f32 = 30.0;
const MIN_BALL_SPEED: f32 = 100.0;
const MAX_BALL_SPEED: f32 = 500.0;
```

### Asset Paths
```
assets/
├── config/
│   ├── game.toml         # Main game configuration
│   ├── input.toml        # Input bindings
│   └── rendering.toml    # Renderer settings
├── levels/
│   ├── tutorial.toml     # First level
│   ├── classic/          # Core level set
│   └── community/        # User-created levels
├── shaders/
│   ├── metaball/         # Metaball shaders (existing)
│   ├── widgets/          # Target, wall, hazard shaders
│   ├── background/       # Environment shaders
│   └── effects/          # Particle, transition shaders
└── audio/                # Future audio assets
    ├── sfx/
    └── music/
```

## 6. Coding Policies

### Naming Conventions
- **Crates**: `category_purpose` format (e.g., `game_physics`, `widget_renderer`)
- **Systems**: `verb_noun` format (e.g., `spawn_balls`, `check_collisions`)
- **Components**: PascalCase nouns (e.g., `Ball`, `Target`)
- **Resources**: PascalCase, descriptive (e.g., `GameState`, `LevelData`)
- **Events**: PascalCase, past or present tense (e.g., `BallSpawned`, `GameWon`)
- **Bundles**: `EntityTypeBundle` (e.g., `BallBundle`, `TargetBundle`)

### Module Structure
```rust
// Each module follows this pattern
pub mod components;
pub mod systems;
pub mod resources;
pub mod events;
pub mod bundles;

// Re-export at crate root
pub use components::*;
pub use systems::*;
// etc.
```

### System Organization
```rust
// Group by lifecycle phase
app.add_systems(Startup, (setup_camera, load_config))
   .add_systems(OnEnter(GamePhase::Playing), spawn_level)
   .add_systems(Update, (
       // Input first
       handle_input,
       // Physics
       update_physics,
       apply_forces,
       // Gameplay
       check_collisions,
       update_game_state,
       // Rendering (ordered by layer)
       render_background,
       render_widgets,
       sync_metaballs,
       render_effects,
       render_ui,
   ).chain())
   .add_systems(OnExit(GamePhase::Playing), cleanup_level);
```

### Error Handling
```rust
// Use Result for fallible operations
fn load_level(path: &str) -> Result<LevelData, LevelError> { ... }

// Log warnings for non-critical issues
if config.ball_speed > MAX_BALL_SPEED {
    warn!("Ball speed {} exceeds maximum {}", config.ball_speed, MAX_BALL_SPEED);
    config.ball_speed = MAX_BALL_SPEED;
}

// Panic only for programmer errors
assert!(ball_count > 0, "Level must have at least one ball");
```

### Component Patterns
```rust
// Marker components for queries
#[derive(Component)]
struct Player;

// Data components are Copy when possible
#[derive(Component, Copy, Clone)]
struct Velocity(Vec2);

// Use bundles for common spawning patterns
#[derive(Bundle)]
struct BallBundle {
    ball: Ball,
    body: RigidBody,
    collider: Collider,
    velocity: Velocity,
    metaball: Metaball,  // For renderer
    #[bundle]
    spatial: SpatialBundle,
}
```

## 7. Testing Strategy

### Unit Tests
- Each crate has `tests/` module
- Test data parsing, math functions, state transitions
- Mock Bevy resources/events where needed

### Integration Tests
- Demo projects serve as integration tests
- Each demo validates specific subsystem
- `demos/full_game_test` validates complete loop

### Performance Benchmarks
```rust
// Track critical metrics
#[bench]
fn bench_metaball_sync_100_balls(b: &mut Bencher) { ... }

#[bench]
fn bench_widget_render_complex_level(b: &mut Bencher) { ... }

#[bench]
fn bench_collision_detection_dense(b: &mut Bencher) { ... }
```

## 8. Platform Considerations

### Web (WASM)
- Keep asset sizes minimal (< 10MB total)
- Use `.webp` for textures
- Async asset loading required
- Target 60 FPS on integrated graphics

### Native
- Support hot-reload for all assets
- Debug overlays behind feature flag
- Uncapped FPS option

## 9. Configuration Philosophy

### Principle: Everything Tunable
```toml
[game]
target_fps = 60
vsync = true

[gameplay]
balls_per_level = 3
target_health = 1
hazard_kills_instantly = true

[rendering]
metaball_smoothness = 0.85
widget_line_thickness = 2.0
background_opacity = 0.8
ui_scale = 1.0

[visuals]
screen_shake_intensity = 0.2

[debug]
show_physics = false
show_fps = true
show_render_layers = false
immortal_balls = false
```

### Config Loading Priority
1. Defaults in code
2. `assets/config/*.toml`
3. User overrides (`user://config.toml`)
4. Command-line arguments
5. Runtime UI (debug mode)

## 10. Dependency Management

### Approved External Crates
- `bevy = "0.15"` - Core engine
- `bevy_rapier2d` - Physics
- `leafwing-input-manager` - Input mapping
- `bevy_egui` - Debug UI only
- `serde`, `toml` - Config/level serialization

### Versioning Policy
- Lock minor versions in `Cargo.toml`
- Update quarterly unless critical fixes
- Test all demos after updates

## 11. Future-Proofing Hooks

### Reserved Component Fields
```rust
pub struct Ball {
    pub velocity: Vec2,
    pub radius: f32,
    pub color: Color,
    // Reserved for future features
    pub color_mixing: Option<ColorMixState>,
    pub powerup: Option<PowerupType>,
    pub trail_effect: Option<TrailConfig>,
}
```

### Extension Points
- `LevelData::custom_scripts` - For future scripting
- `GameAction::Custom(String)` - For modded actions  
- Additional render layers can be added
- New renderer crates follow same pattern

## 12. Documentation Standards

### Code Documentation
```rust
/// Spawns initial balls for the level.
/// 
/// Reads ball configurations from `LevelData` resource and creates
/// entities with physics bodies and metaball renderers.
/// 
/// # Panics
/// Panics if no `LevelData` resource exists.
pub fn spawn_balls(
    mut commands: Commands,
    level: Res<LevelData>,
) { ... }
```

### README Requirements
Each crate must have README with:
- Purpose statement
- Usage example
- Public API reference
- Dependencies listed
- License header

## 13. Release Checklist

### Per-Release Validation
- [ ] All demos run without crashes
- [ ] All render layers composite correctly
- [ ] Level 1 completable in under 60 seconds
- [ ] WASM build under 10MB
- [ ] No clippy warnings
- [ ] Documentation builds
- [ ] Config backwards compatible

### Performance Targets
- 60 FPS with 50 balls on integrated graphics
- < 100ms level load time
- < 16MB RAM usage (native)
- < 32MB RAM usage (WASM)
- All renderers combined < 5ms frame time

## 14. Success Metrics (Technical)

### Code Quality
- Compile time < 30s (incremental)
- Zero unsafe blocks outside FFI
- Test coverage > 70% for gameplay logic
- All systems < 100 lines

### Architecture Health  
- No circular dependencies between crates
- Each crate compilable standalone
- Clear separation of concerns
- Consistent patterns across modules
- Renderer crates remain decoupled

## 15. Mantra Alignment

> "Bounce. Break. Breathe. Repeat."

**Technical Translation**:
- **Bounce**: Physics must feel responsive (< 16ms latency)
- **Break**: Target destruction must be satisfying (visual + audio feedback across all render layers)
- **Breathe**: Clean state transitions, no memory leaks, smooth rendering
- **Repeat**: Fast restart (< 500ms), persistent settings
