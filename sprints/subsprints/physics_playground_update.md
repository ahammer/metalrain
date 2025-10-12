# Physics Playground Enhancement: Event-Driven Interactive Sandbox

**Status**: Planning  
**Priority**: Medium  
**Estimated Effort**: 3-5 days  
**Dependencies**: event_core, widget_renderer, game_core

## Overview

Transform the `physics_playground` demo from a passive observation environment into a fully interactive sandbox with event-driven controls. Users will be able to place widgets, spawn balls, manipulate entities, and experiment with physics in real-time through both keyboard shortcuts and an optional UI overlay.

## Current State Analysis

### What Works
- ✅ Physics simulation with Rapier2D integration
- ✅ Metaball rendering with clustering
- ✅ Basic board setup with walls
- ✅ Initial ball spawning with random properties
- ✅ Velocity clamping and physics constraints

### What''s Missing
- ❌ Interactive widget placement (walls, targets, hazards, paddles, spawn points)
- ❌ Ball spawning at cursor position
- ❌ Entity selection and deletion
- ❌ Tool/mode switching system
- ❌ Visual feedback for current mode and selected entities
- ❌ Event-driven input handling (currently direct spawning only)
- ❌ UI overlay showing available controls
- ❌ Mouse input capture and world position conversion

## Goals

### Primary Objectives
1. **Event-Driven Architecture**: Integrate `event_core` for all user interactions (deterministic, testable, traceable)
2. **Interactive Controls**: Enable keyboard and mouse-based control of the playground
3. **Clear User Interface**: Provide both text-based instructions and optional button-based UI
4. **Widget Placement**: Support placing all game widget types at cursor position
5. **Entity Manipulation**: Allow selection, movement, and deletion of placed entities

### Success Criteria
- All user interactions flow through `event_core` event pipeline
- Can place walls, targets, hazards, paddles, and spawn points via mouse clicks
- Can spawn balls with Space key or dedicated spawn tool
- Can select and delete placed entities
- Can switch between tools using keyboard number keys (1-9)
- Visual feedback shows current mode and selected entities
- UI overlay displays available controls and current tool
- Physics simulation continues correctly with dynamically placed widgets
- Event journal is available for debugging and potential replay

## Architecture Design

### Event Flow
```
User Input (Keyboard/Mouse)
    ↓
InputEvent (KeyDown, MouseClick, MouseMove)
    ↓
KeyMappingMiddleware (map keys to actions)
    ↓
Debounce/Cooldown Middleware
    ↓
EventQueue (frame-atomic)
    ↓
PlaygroundHandlers (process events, mutate world)
    ↓
ECS World Mutations (spawn/despawn entities)
```

### New Components

#### PlaygroundMode Resource
```rust
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaygroundMode {
    #[default]
    SpawnBall,      // Space = spawn ball at cursor
    PlaceWall,      // Click = place wall segment
    PlaceTarget,    // Click = place target
    PlaceHazard,    // Click = place hazard
    PlacePaddle,    // Click = place paddle
    PlaceSpawnPoint,// Click = place spawn point
    Select,         // Click = select entity, drag to move
    Delete,         // Click = delete entity
}
```

#### PlaygroundState Resource
```rust
#[derive(Resource, Default)]
pub struct PlaygroundState {
    pub cursor_world_pos: Option<Vec2>,
    pub selected_entity: Option<Entity>,
    pub preview_entity: Option<Entity>,
    pub ball_spawn_preset: BallSpawnPreset,
}

#[derive(Default, Clone)]
pub struct BallSpawnPreset {
    pub color: GameColor,
    pub radius_range: (f32, f32),
    pub speed_range: (f32, f32),
}
```

### New Events

Extend `GameEvent` enum in `event_core`:

```rust
pub enum GameEvent {
    // Existing events...
    SpawnBall,
    ResetLevel,
    PauseGame,
    ResumeGame,
    
    // New playground events
    SpawnBallAtCursor { position: Vec2, preset: BallSpawnPreset },
    PlaceWidget { widget_type: WidgetType, position: Vec2 },
    SelectEntity { entity: Option<Entity> },
    DeleteEntity { entity: Entity },
    MoveEntity { entity: Entity, position: Vec2 },
    ClearArena,
    TogglePhysics,
    ChangeTool { mode: PlaygroundMode },
}

pub enum WidgetType {
    Wall { start: Vec2, end: Vec2, thickness: f32 },
    Target { health: u8, radius: f32 },
    Hazard { bounds: Rect },
    Paddle,
    SpawnPoint,
}
```

### New Handlers

#### WidgetPlacementHandler
Handles `PlaceWidget` events, spawning the appropriate entity with physics components.

```rust
impl EventHandler for WidgetPlacementHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        match ev {
            GameEvent::PlaceWidget { widget_type, position } => {
                // Spawn entity based on widget_type
                // Add physics components (Collider, RigidBody)
                // Widget renderer will handle visuals automatically
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
```

#### PlaygroundBallSpawnHandler
Handles `SpawnBallAtCursor` events, spawning balls with specified presets.

#### SelectionHandler
Manages entity selection, movement, and visual highlighting.

#### DeletionHandler
Handles entity despawning when in Delete mode.

#### ClearArenaHandler
Despawns all dynamic entities (balls, placed widgets) while preserving boundaries.

### Input Systems

#### Mouse Position Tracking System
```rust
fn track_mouse_position(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<PlaygroundState>,
) {
    // Convert screen coordinates to world coordinates
    // Update state.cursor_world_pos
}
```

#### Mouse Click Handler System
```rust
fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    mut queue: ResMut<EventQueue>,
    frame: Res<FrameCounter>,
) {
    // On click, inject appropriate event based on current mode
    // e.g., PlaceWidget, SelectEntity, DeleteEntity, SpawnBall
}
```

#### Tool Switching System
```rust
fn handle_tool_switching(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<PlaygroundMode>,
) {
    // KeyCode::Digit1 => SpawnBall
    // KeyCode::Digit2 => PlaceWall
    // KeyCode::Digit3 => PlaceTarget
    // etc.
}
```

### UI Overlay

#### Control Instructions Overlay
Display persistent overlay showing:
- Current tool/mode (highlighted)
- Keyboard shortcuts for tool switching
- Mouse controls for current tool
- Special actions (Clear, Reset, Pause)

```rust
fn spawn_instructions_overlay(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Text::new("PHYSICS PLAYGROUND\n\n"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        InstructionsText,
    ));
}

fn update_instructions_overlay(
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    mut query: Query<&mut Text, With<InstructionsText>>,
) {
    // Update text based on current mode
    // Show relevant shortcuts and mouse actions
}
```

#### Optional Button-Based UI
Spawn clickable buttons for tool switching (alternative to keyboard):

```rust
fn spawn_tool_buttons(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            right: px(12),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
        children![
            tool_button("Spawn Ball", PlaygroundMode::SpawnBall),
            tool_button("Place Wall", PlaygroundMode::PlaceWall),
            tool_button("Place Target", PlaygroundMode::PlaceTarget),
            tool_button("Place Hazard", PlaygroundMode::PlaceHazard),
            tool_button("Select", PlaygroundMode::Select),
            tool_button("Delete", PlaygroundMode::Delete),
        ],
    ));
}

fn tool_button(label: &str, mode: PlaygroundMode) -> impl Bundle {
    (
        Button,
        Node {
            padding: UiRect::all(px(8)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
        ToolButton { mode },
        children![(
            Text::new(label),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::WHITE),
        )],
    )
}
```

### Widget Placement Preview System

Show semi-transparent preview of widget before placement:

```rust
fn preview_widget_placement(
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    mut commands: Commands,
    preview_q: Query<Entity, With<PlacementPreview>>,
) {
    // Despawn old preview
    for entity in &preview_q {
        commands.entity(entity).despawn_recursive();
    }
    
    // Spawn new preview if in placement mode and cursor is in world
    if let Some(pos) = state.cursor_world_pos {
        match *mode {
            PlaygroundMode::PlaceWall => { /* spawn wall preview */ },
            PlaygroundMode::PlaceTarget => { /* spawn target preview */ },
            // etc.
            _ => {}
        }
    }
}
```

## Implementation Plan

### Phase 1: Event Core Integration (Day 1)
**Goal**: Wire up event pipeline without breaking existing functionality

- [ ] Add `event_core` dependency to `demos/physics_playground/Cargo.toml`
- [ ] Add `EventCorePlugin` to app configuration
- [ ] Define `PlaygroundMode` and `PlaygroundState` resources
- [ ] Configure `KeyMappingMiddleware` for basic controls
- [ ] Add `DebounceMiddleware` and `CooldownMiddleware` as needed
- [ ] Test: Ensure demo still runs and displays correctly

**Key Mapping Configuration**:
```rust
let mut km = KeyMappingMiddleware::empty();
km.map(KeyCode::Space, KeyMappingOutput::Game(GameEvent::SpawnBallAtCursor))
  .map(KeyCode::KeyC, KeyMappingOutput::Game(GameEvent::ClearArena))
  .map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel))
  .map(KeyCode::KeyP, KeyMappingOutput::Game(GameEvent::TogglePhysics))
  .map(KeyCode::Digit1, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::SpawnBall }))
  .map(KeyCode::Digit2, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceWall }))
  .map(KeyCode::Digit3, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceTarget }))
  .map(KeyCode::Digit4, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceHazard }))
  .map(KeyCode::Digit5, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlacePaddle }))
  .map(KeyCode::Digit6, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceSpawnPoint }))
  .map(KeyCode::Digit7, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::Select }))
  .map(KeyCode::Digit8, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::Delete }));
```

### Phase 2: Input Capture (Day 1-2)
**Goal**: Capture mouse input and convert to world coordinates

- [ ] Implement `track_mouse_position` system
- [ ] Implement `handle_mouse_clicks` system (inject events to queue)
- [ ] Implement `handle_tool_switching` system
- [ ] Add systems to `Update` schedule in proper order
- [ ] Test: Log events to verify input capture works

### Phase 3: Ball Spawning Handler (Day 2)
**Goal**: Enable interactive ball spawning at cursor

- [ ] Extend `GameEvent` with `SpawnBallAtCursor` variant
- [ ] Implement `PlaygroundBallSpawnHandler`
- [ ] Register handler in app
- [ ] Move existing `spawn_ball` logic into handler
- [ ] Test: Space key spawns ball at cursor position

### Phase 4: Widget Placement (Day 2-3)
**Goal**: Enable placing widgets via mouse clicks

- [ ] Extend `GameEvent` with `PlaceWidget` variant and `WidgetType` enum
- [ ] Implement `WidgetPlacementHandler`
- [ ] Add logic for each widget type (Wall, Target, Hazard, Paddle, SpawnPoint)
- [ ] Integrate `WidgetRendererPlugin` for automatic visuals
- [ ] Test: Click to place each widget type

**Widget Spawning Details**:
- **Wall**: Click and drag to define start/end points, or fixed-size segments
- **Target**: Single click, radius based on preset
- **Hazard**: Click and drag to define bounds
- **Paddle**: Single click, default size
- **SpawnPoint**: Single click, default radius

### Phase 5: Selection and Manipulation (Day 3)
**Goal**: Enable selecting and moving entities

- [ ] Extend `GameEvent` with `SelectEntity` and `MoveEntity` variants
- [ ] Implement `SelectionHandler`
- [ ] Add selection highlighting (extend `widget_renderer` or use outline)
- [ ] Implement drag-to-move logic
- [ ] Test: Click to select, drag to move

### Phase 6: Deletion and Clear (Day 3)
**Goal**: Enable removing entities

- [ ] Extend `GameEvent` with `DeleteEntity` and `ClearArena` variants
- [ ] Implement `DeletionHandler`
- [ ] Implement `ClearArenaHandler`
- [ ] Test: Delete selected entity, clear all dynamic entities

### Phase 7: UI Overlay - Instructions (Day 4)
**Goal**: Show text-based instructions and current mode

- [ ] Implement `spawn_instructions_overlay` system
- [ ] Implement `update_instructions_overlay` system
- [ ] Display current tool/mode with highlighting
- [ ] List keyboard shortcuts for tool switching
- [ ] Show mouse controls for current tool
- [ ] Test: Overlay updates when mode changes

**Instructions Content**:
```
PHYSICS PLAYGROUND
------------------
Current Tool: [Spawn Ball]  ← Highlighted

TOOLS (Number Keys):
1: Spawn Ball    5: Place Paddle
2: Place Wall    6: Place Spawn Point
3: Place Target  7: Select Entity
4: Place Hazard  8: Delete Entity

ACTIONS:
Space: Spawn Ball (when in Spawn Ball mode)
Left Click: Action for current tool
C: Clear Arena
R: Reset Level
P: Pause/Resume Physics
H: Toggle this UI
```

### Phase 8: UI Overlay - Buttons (Day 4-5, Optional)
**Goal**: Provide clickable button alternative to keyboard shortcuts

- [ ] Implement `spawn_tool_buttons` system
- [ ] Implement `tool_button` helper function
- [ ] Handle button click interactions
- [ ] Update button visual states based on current mode
- [ ] Test: Click buttons to switch tools

### Phase 9: Placement Preview (Day 5)
**Goal**: Show preview of widget before placement

- [ ] Add `PlacementPreview` marker component
- [ ] Implement `preview_widget_placement` system
- [ ] Render semi-transparent preview at cursor position
- [ ] Update preview when mode or cursor position changes
- [ ] Test: Preview appears and updates correctly

### Phase 10: Polish and Testing (Day 5)
**Goal**: Refine UX and ensure robustness

- [ ] Add collision detection to prevent overlapping widgets
- [ ] Implement grid snapping (optional)
- [ ] Add visual feedback for invalid placements
- [ ] Test all tool modes comprehensively
- [ ] Verify physics simulation remains stable
- [ ] Check event journal for correctness
- [ ] Performance testing with many entities
- [ ] Documentation and code comments

## Technical Considerations

### Physics Integration
- All placed widgets must have appropriate `Collider` components
- Use `RigidBody::Fixed` for walls and static widgets
- Use `RigidBody::Dynamic` for movable entities (if implemented)
- Ensure collision layers are correct (use `RenderLayer::GameWorld`)

### Event Payload Design
- Keep events minimal but complete (avoid unnecessary data)
- Use owned data for simplicity (avoid entity references in events when possible)
- Consider serialization needs for future replay feature

### Performance
- Use entity commands for batch operations
- Avoid per-frame allocations in input systems
- Limit preview entity updates to when cursor moves
- Despawn preview entities when not needed

### User Experience
- Clear visual distinction between modes
- Immediate feedback for actions (e.g., sound effects, visual flash)
- Undo/redo support (future enhancement, requires event replay)
- Save/load playground state (future enhancement)

## Testing Strategy

### Manual Testing Checklist
- [ ] Can spawn balls at cursor with Space key
- [ ] Can place each widget type (Wall, Target, Hazard, Paddle, SpawnPoint)
- [ ] Can switch between tools using number keys
- [ ] Can switch between tools using UI buttons (if implemented)
- [ ] Can select placed entities
- [ ] Can move selected entities (drag)
- [ ] Can delete selected entities
- [ ] Can clear entire arena
- [ ] UI overlay shows current mode correctly
- [ ] UI overlay shows correct keyboard shortcuts
- [ ] Preview system shows widget before placement
- [ ] Physics simulation works with dynamically placed widgets
- [ ] Event journal captures all user actions

### Automated Testing
- [ ] Unit tests for handlers (spawn, place, select, delete)
- [ ] Integration tests for event flow
- [ ] Test middleware chain with mock inputs
- [ ] Verify event serialization/deserialization (future)

## Future Enhancements

### Advanced Features
- **Undo/Redo**: Leverage event journal for action reversal
- **Save/Load**: Serialize playground state to file
- **Grid Snapping**: Optional alignment for precise placement
- **Widget Properties Editor**: Edit properties of selected entities
- **Copy/Paste**: Duplicate widgets
- **Multi-Select**: Select and manipulate multiple entities
- **Replay Mode**: Play back recorded input sequences

### UI Improvements
- **Context Menu**: Right-click for entity-specific actions
- **Property Panel**: Show and edit selected entity properties
- **Tool Palette**: Visual selector for tools with icons
- **Status Bar**: Show cursor position, entity count, FPS

### Gameplay Features
- **Templates**: Pre-configured widget layouts (ramps, tunnels, mazes)
- **Challenges**: Goal-based scenarios (get ball to target, avoid hazards)
- **Recording**: Record and share gameplay sessions
- **Time Manipulation**: Slow-motion, fast-forward, rewind

## References

### Codebase Files
- `demos/physics_playground/src/lib.rs` - Current playground implementation
- `crates/event_core/` - Event pipeline and handlers
- `crates/game_core/src/components.rs` - Widget components
- `crates/widget_renderer/` - Widget visual systems
- `crates/game_physics/` - Physics integration

### Research Documents
- `.copilot-tracking/research/20250930-physics-playground-event-driven-enhancement-research.md`

### External Examples
- `external/bevy/examples/ui/standard_widgets.rs` - Bevy UI patterns
- `external/bevy/examples/state/custom_transitions.rs` - State management
- `src/physics/gravity/widgets.rs` - Widget toggle patterns
- `src/gameplay/spawn_widgets.rs` - Spawn widget implementation

## Success Metrics

### Functionality
- ✅ All planned tool modes work correctly
- ✅ Event-driven architecture fully integrated
- ✅ UI overlay provides clear guidance
- ✅ Physics simulation remains stable

### Code Quality
- ✅ Clean separation of concerns (input, events, handlers, rendering)
- ✅ Handler logic is testable in isolation
- ✅ Events are well-documented with clear semantics
- ✅ No direct input polling in gameplay code

### User Experience
- ✅ Intuitive tool switching (keyboard or buttons)
- ✅ Clear visual feedback for all actions
- ✅ Smooth, responsive interactions
- ✅ Helpful instructions visible at all times

## Conclusion

This sprint transforms the physics playground from a passive demo into a powerful interactive sandbox, showcasing the project''s event-driven architecture while providing a fun, educational tool for physics experimentation. The clear UI options (both text and buttons) ensure accessibility for all users, while the event journal enables future features like replay and undo/redo.
