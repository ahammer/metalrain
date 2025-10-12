# Sprint 4: Game World Elements (Walls, Targets, Hazards)

## Sprint Goal

Create the visual representation of game world elements (walls, targets, hazards) by establishing the `widget_renderer` crate and integrating it with the existing rendering pipeline from Sprint 3.

## Current State

- ✅ Core rendering pipeline with 5 layers established (Sprint 3)
- ✅ Physics system with ball spawning and movement (Sprint 2)
- ✅ Metaball renderer for fluid visuals (Sprint 3)
- ⚠️ No `widget_renderer` crate exists yet
- ⚠️ No visual representation for walls, targets, or hazards
- ⚠️ Physics playground needs game elements to be useful

## Deliverables

### 1. Widget Renderer Crate Setup

- [x] Create `widget_renderer` crate structure with Cargo.toml
- [x] Add to workspace members
- [x] Set up basic plugin architecture
- [x] Configure integration with Layer 1 (GameWorld) (RenderLayers layer 1)
- [x] Create component definitions for Wall, Target, Hazard (updated in `game_core` per spec)

### 2. Wall Rendering System

- [x] Simple line/rectangle rendering using Bevy sprites (mesh glow deferred)
- [ ] Basic glow effect using overlapping sprites (deferred)
- [x] Wall component with physics integration
- [ ] Debug visualization for collision boundaries (Rapier debug lines always enabled; toggle TBD)
- [x] Support for straight walls (curves deferred to future sprint)

### 3. Target Rendering

- [x] Circular/square targets (square sprite implementation for MVP)
- [x] Simple color system for target types (prototype colors)
- [x] Hit animation (scale & color flash)
- [x] Health visualization (opacity based)
- [x] Destruction animation (fade out + scale + cleanup)

### 4. Hazard Zone Rendering

- [x] Area visualization using transparent sprites
- [x] Simple pulsing warning effect
- [x] Clear visual distinction (red tint & alpha pulse)
- [x] Basic pit hazard type only (others deferred)

### 5. Physics Playground Enhancement

- [x] Update `physics_playground` to spawn walls, targets, hazards
- [x] Interactive element placement (RMB wall (2-click), MMB target, H hazard)
- [x] Visual feedback for collisions (target hit flash / animation)
- [ ] Performance metrics display (pending)
- [ ] Test arena with all element types (interactive spawning; scripted arena TBD)

## Technical Specifications

### Simplified Components (MVP Focus)

```rust
// Wall - Simple line segment for now
#[derive(Component)]
pub struct Wall {
    pub start: Vec2,
    pub end: Vec2,
    pub thickness: f32,
    pub color: Color,
}

// Target - Basic destructible object
#[derive(Component)]
pub struct Target {
    pub health: u8,
    pub max_health: u8,
    pub radius: f32,
    pub color: Color,
    pub state: TargetState,
}

// Hazard - Danger zone
#[derive(Component)]
pub struct Hazard {
    pub bounds: Rect,
    pub hazard_type: HazardType,
}

#[derive(Clone, Debug)]
pub enum TargetState {
    Idle,
    Hit(f32),      // animation progress
    Destroying(f32), // animation progress
}

#[derive(Clone, Debug)]
pub enum HazardType {
    Pit, // Only implement this for MVP
}
```

### Plugin Structure

```rust
pub struct WidgetRendererPlugin;

impl Plugin for WidgetRendererPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_widget_materials)
            .add_systems(Update, (
                spawn_wall_visuals,
                spawn_target_visuals,
                spawn_hazard_visuals,
                update_target_animations,
                update_hazard_pulse,
            ))
            .add_systems(PostUpdate, 
                sync_visuals_with_physics
                    .after(PhysicsSet::Writeback)
            );
    }
}
```

### Rendering Approach (Using Bevy Built-ins)

```rust
// Use Bevy's 2D primitives instead of custom shaders
fn spawn_wall_visuals(
    mut commands: Commands,
    walls: Query<(Entity, &Wall), Added<Wall>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, wall) in walls.iter() {
        // Calculate wall rectangle
        let direction = (wall.end - wall.start).normalize();
        let length = wall.start.distance(wall.end);
        let angle = direction.y.atan2(direction.x);
        
        commands.entity(entity).insert((
            MaterialMesh2dBundle {
                mesh: meshes.add(Rectangle::new(length, wall.thickness)).into(),
                material: materials.add(wall.color),
                transform: Transform::from_translation(
                    ((wall.start + wall.end) / 2.0).extend(0.0)
                ).with_rotation(Quat::from_rotation_z(angle)),
                ..default()
            },
            RenderLayers::layer(1), // GameWorld layer
        ));
    }
}
```

## Physics Integration Requirements

### Collision Setup

```rust
// Walls need colliders
commands.spawn((
    Wall { start, end, thickness, color },
    RigidBody::Fixed,
    Collider::cuboid(length / 2.0, thickness / 2.0),
));

// Targets need sensors
commands.spawn((
    Target { health: 3, max_health: 3, radius: 20.0, color, state },
    Sensor,
    Collider::ball(20.0),
    ActiveEvents::COLLISION_EVENTS,
));

// Hazards need trigger zones
commands.spawn((
    Hazard { bounds, hazard_type: HazardType::Pit },
    Sensor,
    Collider::cuboid(bounds.width() / 2.0, bounds.height() / 2.0),
));
```

## Demo Features (Physics Playground)

### Interactive Controls

- **Left Click**: Spawn ball at cursor
- **Right Click**: Place wall segment
- **Middle Click**: Place target
- **H Key**: Place hazard zone
- **C Key**: Clear all elements
- **R Key**: Reset demo
- **Space**: Pause physics
- **Tab**: Toggle debug visuals

### Test Arena Elements

- 10-15 wall segments forming boundaries
- 5-10 targets of varying health
- 2-3 hazard zones
- Ball spawn point indicator
- FPS and entity count display

## Acceptance Criteria

### Must Have (Week 1 Goals)

- ✅ Widget renderer crate created and compiles
- ✅ Walls render and block balls
- ✅ Targets render and detect hits
- ✅ Hazards render with danger indication
- ✅ Physics playground demo runs at 60 FPS

### Should Have (Week 2 Goals)

- ✅ Target hit animations work
- ✅ Hazard pulsing effect
- ✅ Debug visualization toggle
- ✅ Interactive element placement

### Could Have (If Time Permits)

- ⏸️ Glow effects on walls
- ⏸️ Particle effects on target destruction
- ⏸️ Multiple hazard types
- ⏸️ Curved walls

## Testing Strategy

### Unit Tests

- Component creation and defaults
- Animation state transitions
- Collision shape generation

### Integration Tests  

- Widget renderer plugin loads
- Elements spawn with correct layers
- Physics bodies align with visuals

### Manual Testing

- Visual appearance check
- Collision accuracy
- Performance monitoring
- Memory leak detection

## Performance Targets (Revised)

| Metric | Target | Current |
|--------|--------|---------|
| Wall segments | 50 | TBD |
| Active targets | 20 | TBD |
| Hazard zones | 5 | TBD |
| Frame time | <16ms | TBD |
| Widget render time | <3ms | TBD |

## Dependencies

### Internal Crates

```toml
[dependencies]
bevy = { workspace = true }
game_core = { path = "../game_core" }
game_physics = { path = "../game_physics" }
game_rendering = { path = "../game_rendering" }
```

### External Crates (Minimal)

```toml
rand = "0.8"  # For simple randomization
```

## Definition of Done

- [x] `widget_renderer` crate created and added to workspace
- [x] Wall, Target, and Hazard components defined
- [x] Basic rendering for all three element types
- [x] Physics integration working (collisions detected)
- [x] Enhanced physics playground demo (baseline; metrics + arena polish pending)
- [ ] Performance targets met (instrumentation not implemented yet)
- [x] Basic documentation in README
- [x] No regression in existing functionality (workspace builds)

## Notes for Next Sprint

Sprint 5 will add gameplay logic:

- Game state management (menu, playing, game over)
- Score tracking and UI
- Level progression
- Win/lose conditions
- Sound effects integration

Focus this sprint on getting the visual elements working and integrated with physics. Polish and effects can come later.
