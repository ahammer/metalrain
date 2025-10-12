# Sprint 6: Level System & Data Pipeline

## Sprint Goal
Create a flexible level system with TOML-based level definitions, implement level loading and validation, and establish a content pipeline for level creation. Enable progression through multiple levels.

## Deliverables

### 1. Level Management Crate (`game_levels`)
- [ ] Create `game_levels` crate structure
- [ ] Define level data format (TOML)
- [ ] Implement level parser and validator
- [ ] Create level loading system
- [ ] Build level transition logic

### 2. Level Data Format
- [ ] Arena dimensions and boundaries
- [ ] Ball spawn configurations
- [ ] Target placement and properties
- [ ] Wall segment definitions
- [ ] Hazard zone specifications
- [ ] Background and visual settings

### 3. Level Loading Pipeline
- [ ] File-based level loading
- [ ] Level validation and error handling
- [ ] Entity spawning from level data
- [ ] Hot-reload support for development
- [ ] Level pack organization

### 4. Level Progression
- [ ] Sequential level advancement
- [ ] Level selection menu (basic)
- [ ] Progress persistence
- [ ] Difficulty curve implementation
- [ ] Tutorial level design

### 5. Demo: Level Editor
- [ ] Visual level creation tool
- [ ] Drag-and-drop entity placement
- [ ] Real-time preview
- [ ] TOML export/import
- [ ] Validation feedback

## Technical Specifications

### Level Data Format (TOML)
```toml
# levels/tutorial_01.toml
[meta]
name = "Getting Started"
description = "Learn the basics"
version = "1.0"
author = "Game Team"
difficulty = 1
par_time = 30.0

[arena]
width = 1280
height = 720
boundary_type = "solid"  # solid, bounce, wrap

[background]
preset = "gradient_blue"
particle_density = 0.3
animation_speed = 1.0

[[balls]]
spawn_time = 0.0
position = [640, 500]
velocity = [200, -150]
radius = 20.0
color = "blue"

[[balls]]
spawn_time = 0.5
position = [400, 500]
velocity = [-150, -200]
radius = 20.0
color = "blue"

[[targets]]
position = [200, 200]
size = [40, 40]
health = 1
color = "cyan"
value = 100

[[targets]]
position = [1080, 200]
size = [40, 40]
health = 1
color = "cyan"
value = 100

[[walls]]
# Perimeter walls
segments = [
    [[0, 0], [1280, 0]],      # Top
    [[1280, 0], [1280, 720]], # Right
    [[1280, 720], [0, 720]],  # Bottom
    [[0, 720], [0, 0]]        # Left
]
thickness = 4.0
material = "default"

[[hazards]]
type = "pit"
bounds = { x = 540, y = 0, width = 200, height = 60 }
instant_kill = true
```

### Level Data Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelData {
    pub meta: LevelMeta,
    pub arena: ArenaConfig,
    pub background: BackgroundConfig,
    pub balls: Vec<BallSpawn>,
    pub targets: Vec<TargetSpawn>,
    pub walls: Vec<WallDefinition>,
    pub hazards: Vec<HazardDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMeta {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub difficulty: u8,  // 1-10
    pub par_time: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallSpawn {
    pub spawn_time: f32,
    pub position: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub color: GameColor,
}
```

### Level Loading System
```rust
pub struct LevelLoader;

impl LevelLoader {
    pub fn load_from_file(path: &str) -> Result<LevelData, LevelError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| LevelError::IoError(e))?;
        
        let level: LevelData = toml::from_str(&contents)
            .map_err(|e| LevelError::ParseError(e))?;
        
        level.validate()?;
        Ok(level)
    }
    
    pub fn validate(level: &LevelData) -> Result<(), LevelError> {
        // Check arena bounds
        if level.arena.width <= 0.0 || level.arena.height <= 0.0 {
            return Err(LevelError::InvalidArenaSize);
        }
        
        // Verify at least one ball
        if level.balls.is_empty() {
            return Err(LevelError::NoBalls);
        }
        
        // Verify at least one target
        if level.targets.is_empty() {
            return Err(LevelError::NoTargets);
        }
        
        // Check entity positions within bounds
        for ball in &level.balls {
            if !level.arena.contains_point(ball.position) {
                return Err(LevelError::EntityOutOfBounds);
            }
        }
        
        Ok(())
    }
}
```

### Level Spawning
```rust
pub fn spawn_level(
    mut commands: Commands,
    level_data: Res<LevelData>,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
) {
    // Clear existing level
    cleanup_level(&mut commands);
    
    // Spawn arena boundaries
    spawn_walls(&mut commands, &level_data.walls);
    
    // Spawn targets
    for target_data in &level_data.targets {
        commands.spawn(TargetBundle {
            target: Target {
                health: target_data.health,
                max_health: target_data.health,
                color: target_data.color,
                ..default()
            },
            transform: Transform::from_translation(
                target_data.position.extend(0.0)
            ),
            ..default()
        });
    }
    
    // Spawn hazards
    for hazard_data in &level_data.hazards {
        commands.spawn(HazardBundle {
            hazard: Hazard {
                zone_type: hazard_data.hazard_type,
                bounds: hazard_data.bounds,
                ..default()
            },
            ..default()
        });
    }
    
    // Schedule ball spawns
    for ball_data in &level_data.balls {
        commands.spawn(BallSpawnTimer {
            spawn_at: ball_data.spawn_time,
            data: ball_data.clone(),
        });
    }
    
    // Update game state
    game_state.targets_remaining = level_data.targets.len() as u32;
    game_state.balls_remaining = level_data.balls.len() as u32;
}
```

### Level Progression Manager
```rust
pub struct LevelProgression {
    pub current_level: usize,
    pub completed_levels: HashSet<String>,
    pub level_sequence: Vec<String>,
}

impl LevelProgression {
    pub fn load_next_level(&mut self) -> Option<String> {
        self.current_level += 1;
        self.level_sequence.get(self.current_level).cloned()
    }
    
    pub fn mark_completed(&mut self, level_name: &str) {
        self.completed_levels.insert(level_name.to_string());
    }
    
    pub fn is_unlocked(&self, level_index: usize) -> bool {
        // Linear progression: unlock if previous is completed
        if level_index == 0 {
            return true;
        }
        
        level_index <= self.completed_levels.len()
    }
}
```

### Level Editor Features
```rust
pub struct LevelEditor {
    pub mode: EditorMode,
    pub selected_tool: EditorTool,
    pub current_level: LevelData,
    pub grid_snap: bool,
    pub grid_size: f32,
}

#[derive(Debug, Clone)]
pub enum EditorTool {
    PlaceBall,
    PlaceTarget,
    DrawWall,
    PlaceHazard,
    Eraser,
    Select,
}

impl LevelEditor {
    pub fn handle_mouse_click(&mut self, world_pos: Vec2) {
        match self.selected_tool {
            EditorTool::PlaceTarget => {
                self.current_level.targets.push(TargetSpawn {
                    position: self.snap_to_grid(world_pos),
                    health: 1,
                    color: GameColor::Cyan,
                    size: Vec2::new(40.0, 40.0),
                    value: 100,
                });
            }
            // Handle other tools...
        }
    }
    
    pub fn export_to_toml(&self) -> String {
        toml::to_string_pretty(&self.current_level).unwrap()
    }
}
```

## Level Design Guidelines

### Difficulty Progression
1. **Tutorial (Levels 1-3)**: Simple layouts, few hazards
2. **Easy (Levels 4-7)**: Introduction of complex walls
3. **Medium (Levels 8-12)**: Multiple hazard types
4. **Hard (Levels 13-18)**: Limited balls, many targets
5. **Expert (Levels 19+)**: Precision required

### Level Validation Rules
- Minimum 1 ball, maximum 10
- Minimum 1 target, maximum 50
- Hazards cannot overlap targets
- Walls must form closed boundaries
- Spawn positions must be safe

## Success Criteria

- ✅ Levels load from TOML files correctly
- ✅ Invalid levels are rejected with clear errors
- ✅ Level progression works smoothly
- ✅ Hot-reload works in development
- ✅ Level editor exports valid TOML
- ✅ 20+ levels created for launch

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Invalid level data | High | Comprehensive validation |
| Level balance issues | Medium | Playtesting, par times |
| File loading errors | High | Fallback to embedded levels |
| Editor complexity | Medium | Start with basic features |

## Dependencies

### From Previous Sprints
- Sprint 1: Core components
- Sprint 4: Game world elements
- Sprint 5: Gameplay systems

### External Crates
- `toml = "0.8"`
- `serde = { version = "1.0", features = ["derive"] }`

### Assets
- Level files in `assets/levels/`
- Editor UI icons
- Grid textures

## Definition of Done

- [ ] Level loader parses TOML correctly
- [ ] Validation catches all error cases
- [ ] 10+ playable levels created
- [ ] Level progression system works
- [ ] Hot-reload functions in development
- [ ] Level editor can create valid levels
- [ ] Export/import cycle verified
- [ ] README documents level format

## Notes for Next Sprint

Sprint 7 will add input systems:
- Configurable key bindings
- Gamepad support
- Touch/mouse input for future
- Gesture recognition basics
- Input remapping UI

The level system provides the content framework that players will experience through the input system.
