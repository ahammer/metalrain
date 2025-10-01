<!-- markdownlint-disable-file -->
# Task Research Notes: Physics Playground Event-Driven Enhancement

## Research Executed

### File Analysis
- demos/physics_playground/src/lib.rs
  - Current implementation: Simple startup systems spawning balls and walls
  - Direct spawn_ball function without event-driven architecture
  - No widget placement or interactive controls beyond basic rendering
  
- crates/event_core/README.md & src/
  - Comprehensive event pipeline with middleware chain (KeyMapping, Debounce, Cooldown)
  - Handler registry pattern for game logic
  - Frame-atomic queue with deferral mechanism
  - Journal for event history tracking
  - Supports InputEvent → GameEvent → Handler flow

- crates/game_core/src/components.rs
  - Widget components available: Wall, Target, Hazard, Paddle, SpawnPoint
  - Selected marker component for entity selection
  - Components designed for placement and manipulation

- crates/widget_renderer/src/lib.rs
  - Visual systems for all widget types
  - Handles spawning visuals when components are added
  - Animation systems for targets, hazards, spawn points
  - Selection highlighting system

### Code Search Results
- PlayerAction|GameEvent|InputEvent in event_core
  - PlayerAction: Move(Direction2D), PrimaryAction, SecondaryAction, Confirm, Cancel, SelectNext, SelectPrevious
  - GameEvent: SpawnBall, ResetLevel, PauseGame, ResumeGame, PlayerAction(PlayerAction), etc.
  - InputEvent: KeyDown(KeyCode) with KeyMappingMiddleware for translation

- Handler patterns in vent_core/src/handlers/mod.rs
  - BallLifecycleHandler: Handles SpawnBall and BallLostToHazard events
  - TargetInteractionHandler: Handles TargetHit, TargetDestroyed, level reset
  - Resource-based counters for game state tracking

### External Research
- Bevy UI widget examples in xternal/bevy/examples/ui/
  - Standard widgets: buttons, sliders, checkboxes, radio buttons
  - Observer pattern with On<Activate>, On<ValueChange<T>>
  - Interaction handling via Interaction component and queries

### Project Conventions
- Standards referenced:
  - Plugin-based architecture (GameCorePlugin, EventCorePlugin, etc.)
  - System ordering: PreUpdate (input) → Update (logic) → PostUpdate (reduction)
  - RenderLayers for visual separation (Background=0, GameWorld=1, Metaballs=2, Effects=3, Ui=4)
  
- Instructions followed:
  - Event-driven architecture using vent_core for all user interactions
  - Separation of concerns: core components, physics, rendering, widgets
  - Builder pattern for plugin configuration

## Key Discoveries

### Project Structure
The project follows a modular crate architecture:
- vent_core: Event pipeline with middleware and handlers
- game_core: Core components (Ball, Wall, Target, Hazard, Paddle, SpawnPoint)
- game_physics: Rapier2D integration with PhysicsConfig
- game_rendering: Compositor and render layers
- widget_renderer: Visual systems for game widgets
- metaball_renderer: Specialized rendering for balls

### Implementation Patterns

#### Event-Driven Input Handling
`ust
// From event_core README
App::new()
    .add_plugins(EventCorePlugin { journal_capacity: 256 })
    .register_middleware(KeyMappingMiddleware::with_default_gameplay())
    .register_middleware(DebounceMiddleware::new(0))
    .register_middleware(CooldownMiddleware::new(2))
    .register_handler(CustomHandler)
`

#### Handler Implementation Pattern
`ust
// From event_core/src/handlers/mod.rs
pub struct BallLifecycleHandler;
impl EventHandler for BallLifecycleHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        match ev {
            GameEvent::SpawnBall => {
                // Spawn logic here
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
    fn name(&self) -> &'static str { "BallLifecycleHandler" }
}
`

#### Widget Spawning Pattern
`ust
// From widget_renderer/src/lib.rs - automatic visual spawning
fn spawn_target_visuals(mut commands: Commands, targets: Query<(Entity, &Target), Added<Target>>) {
    for (entity, target) in &targets {
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(target.color, size),
            Transform::from_translation(Vec3::Z),
        ));
    }
}
`

### Complete Examples

#### Current Physics Playground Structure
`ust
pub fn run_physics_playground() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(GameRenderingPlugin)
        .add_plugins(MetaballRendererPlugin::with(settings))
        .add_systems(Startup, (setup_board, spawn_initial_balls))
        .run();
}
`

#### Event Core Integration Example
`ust
// Middleware configuration
let mut km = KeyMappingMiddleware::empty();
km.map(KeyCode::Space, KeyMappingOutput::Action(PlayerAction::PrimaryAction))
  .map(KeyCode::KeyB, KeyMappingOutput::Game(GameEvent::SpawnBall))
  .map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel));
`

### API and Schema Documentation

#### Event Payload Types
- EventPayload::Input(InputEvent) - Raw input events
- EventPayload::Game(GameEvent) - High-level game events
- EventEnvelope - Wraps payload with metadata (frame, timestamp, source)

#### PlayerAction Enum
`ust
pub enum PlayerAction {
    PrimaryAction,      // Confirm/spawn/place
    SecondaryAction,    // Cancel/delete
    Move(Direction2D),  // Movement
    Confirm,
    Cancel,
    SelectNext,
    SelectPrevious,
}
`

#### GameEvent Enum Extensions Needed
Current events cover ball lifecycle and game state, but need widget-specific events:
- Widget placement events
- Widget selection events
- Widget deletion events
- Parameter adjustment events

### Configuration Examples

#### PhysicsConfig Resource
`ust
PhysicsConfig {
    pixels_per_meter: 50.0,
    gravity: (0.0, -500.0),
    ball_restitution: 0.95,
    ball_friction: 0.1,
    clustering_strength: 100.0,
    clustering_radius: 150.0,
    max_ball_speed: 500.0,
    min_ball_speed: 100.0,
}
`

#### RenderLayer Configuration
`ust
pub enum RenderLayer {
    Background = 0,
    GameWorld = 1,
    Metaballs = 2,
    Effects = 3,
    Ui = 4,
}
`

### Technical Requirements

#### Widget Placement System
- Mouse/touch input capture and world position conversion
- Grid snapping (optional)
- Preview visualization before placement
- Collision detection to prevent overlapping widgets

#### Selection and Manipulation
- Click/tap to select widgets
- Visual feedback for selected state (highlight, outline)
- Drag to move selected widgets
- Delete selected widgets
- Property editing for selected widgets

#### Interactive Controls Requirements
- Spawn ball at cursor position
- Spawn widgets (walls, targets, hazards, paddles, spawn points)
- Toggle widget properties (e.g., activate/deactivate spawn points)
- Clear all entities
- Reset arena
- Pause/resume physics

## Recommended Approach

Integrate event_core for all interactive controls while preserving the physics playground's sandbox nature.

### Architecture Overview

**Event Flow**: 
Input → KeyMapping → Debounce/Cooldown → EventQueue → PlaygroundHandlers → World Mutations

**Components**:
1. **Input Layer**: Capture mouse clicks, key presses, convert to InputEvents
2. **Middleware Layer**: Map keys to actions (Space=spawn ball, W=place wall, etc.)
3. **Handler Layer**: Implement playground-specific handlers for widget placement, selection, spawning
4. **UI Layer**: Optional overlay showing controls and selected widget type
5. **State Management**: Resources tracking current tool/mode (spawn, place, select, delete)

### Key Implementation Steps

1. **Add event_core integration**
   - Register EventCorePlugin
   - Configure KeyMappingMiddleware for playground controls
   - Add debounce/cooldown as needed

2. **Extend GameEvent enum** (or create PlaygroundEvent)
   - PlaceWidget { widget_type, position }
   - SelectWidget { entity }
   - DeleteWidget { entity }
   - SpawnBallAtCursor { position }
   - ClearArena
   - TogglePhysics

3. **Create PlaygroundHandlers**
   - WidgetPlacementHandler: Spawns widgets at cursor position
   - SelectionHandler: Manages selected entity state
   - BallSpawnHandler: Spawns balls with random properties
   - ClearHandler: Despawns all dynamic entities

4. **Implement Input Systems**
   - Mouse position tracking system
   - Click detection → EventQueue injection
   - Convert screen coords to world coords

5. **Add WidgetRendererPlugin**
   - Already exists, just integrate into playground
   - Provides automatic visuals for all widget components

6. **Create UI overlay** (optional)
   - Show available controls
   - Display current mode/tool
   - Show selected widget properties

7. **State Management Resources**
   - PlaygroundMode: enum for current tool (SpawnBall, PlaceWall, PlaceTarget, Select, Delete)
   - SelectedEntity: Option<Entity> for current selection
   - SpawnPreset: Configuration for ball spawning (color, size range)

### Integration Points

**Existing Systems to Preserve**:
- Physics simulation (Rapier2D)
- Metaball rendering
- Clustering forces
- Velocity clamping

**New Systems to Add**:
- Input capture (mouse position, clicks)
- Event injection into EventQueue
- Widget placement preview
- Selection highlighting (extend widget_renderer)
- Mode/tool switching

**Plugin Configuration**:
`ust
pub fn run_physics_playground() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(GameRenderingPlugin)
        .add_plugins(MetaballRendererPlugin::with(settings))
        .add_plugins(WidgetRendererPlugin)  // Add visuals for placed widgets
        .add_plugins(EventCorePlugin { journal_capacity: 256 })  // Event system
        .register_middleware(playground_key_mapping())
        .register_handler(WidgetPlacementHandler)
        .register_handler(BallSpawnHandler)
        .register_handler(SelectionHandler)
        .init_resource::<PlaygroundMode>()
        .init_resource::<SelectedEntity>()
        .add_systems(Startup, setup_board)
        .add_systems(Update, (
            track_mouse_position,
            handle_mouse_clicks,
            preview_widget_placement,
            tool_mode_switching,
        ))
        .run();
}
`

## Implementation Guidance

### Objectives
- Transform physics_playground into a robust sandbox with interactive widget placement
- Use event_core for all user interactions (deterministic, testable, traceable)
- Support multiple tools: spawn balls, place widgets (walls/targets/hazards/paddles/spawn points), select, delete
- Maintain existing physics simulation and metaball rendering

### Key Tasks
1. Integrate EventCorePlugin and configure middleware for playground controls
2. Create or extend GameEvent with playground-specific events (widget placement, selection, deletion)
3. Implement PlaygroundHandlers for each interactive feature
4. Add input systems for mouse tracking and click detection
5. Create PlaygroundMode resource for tool switching (keyboard 1-9 for different tools)
6. Add UI overlay showing available controls and current mode
7. Implement widget placement preview system
8. Extend widget_renderer selection highlighting for interactive feedback

### Dependencies
- event_core: Event pipeline and handler registration
- widget_renderer: Visual systems for placed widgets
- game_core: Widget components (Wall, Target, Hazard, Paddle, SpawnPoint)
- game_physics: Physics integration for spawned entities
- bevy_rapier2d: Collider creation for placed widgets

### Success Criteria
- All interactions go through event_core (no direct input polling in gameplay code)
- Can place walls, targets, hazards, paddles, and spawn points via mouse clicks
- Can select and delete placed widgets
- Can spawn balls with Space key or mouse click (depending on mode)
- Can switch between tools with keyboard number keys
- Visual feedback for current mode and selected entities
- Physics simulation continues to work correctly with dynamically placed widgets
- Event journal can be inspected for debugging and replay potential
