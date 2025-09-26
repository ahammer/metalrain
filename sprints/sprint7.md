# Sprint 7: Input System & Player Control

## Sprint Goal
Implement a flexible input system with configurable bindings, support for keyboard and gamepad controls, and lay the foundation for future player agency features like paddle control.

## Deliverables

### 1. Input Management Crate (`game_input`)
- [ ] Create `game_input` crate structure
- [ ] Integrate leafwing-input-manager
- [ ] Define game action enums
- [ ] Implement input binding configuration
- [ ] Create input handling systems

### 2. Core Input Actions
- [ ] Restart round (R key)
- [ ] Pause/resume (ESC/P)
- [ ] Debug toggle (F3)
- [ ] Quick restart (Space when game over)
- [ ] Menu navigation

### 3. Configurable Bindings
- [ ] TOML-based input configuration
- [ ] Runtime rebinding support
- [ ] Default bindings fallback
- [ ] Multiple input sources per action
- [ ] Conflict detection

### 4. Gamepad Support
- [ ] Xbox/PlayStation controller mapping
- [ ] Analog stick handling
- [ ] Button mapping
- [ ] Vibration feedback hooks
- [ ] Hot-plug detection

### 5. Demo: Input Test
- [ ] Display active inputs
- [ ] Test all bindings
- [ ] Gamepad connection status
- [ ] Rebinding interface
- [ ] Input recording/playback

## Technical Specifications

### Action Definitions
```rust
use leafwing_input_manager::prelude::*;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum GameAction {
    // Core controls
    Restart,
    Pause,
    QuickRestart,
    
    // Debug
    ToggleDebug,
    ToggleFPS,
    SpawnBall,
    DestroyTarget,
    
    // Menu
    MenuUp,
    MenuDown,
    MenuSelect,
    MenuBack,
    
    // Future: Player control
    AimLeft,
    AimRight,
    LaunchBall,
    ActivatePower,
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum MenuAction {
    Navigate(Vec2),
    Select,
    Back,
    Tab,
}
```

### Input Configuration (TOML)
```toml
# assets/config/input.toml
[keyboard]
restart = ["R", "F5"]
pause = ["Escape", "P"]
quick_restart = ["Space"]
toggle_debug = ["F3"]
toggle_fps = ["F4"]

# Debug shortcuts
spawn_ball = ["B"]
destroy_target = ["T"]

# Menu navigation
menu_up = ["Up", "W"]
menu_down = ["Down", "S"]
menu_select = ["Return", "Space"]
menu_back = ["Escape", "Backspace"]

[gamepad]
restart = ["South"]         # A/X button
pause = ["Start"]
quick_restart = ["North"]    # Y/Triangle
toggle_debug = ["Select"]

# Menu with d-pad
menu_up = ["DPadUp"]
menu_down = ["DPadDown"]
menu_select = ["South"]
menu_back = ["East"]         # B/Circle

[mouse]
# Future: aim with mouse
aim_position = "Position"
launch = "LeftButton"
cancel = "RightButton"
```

### Input Binding System
```rust
pub struct InputConfig {
    pub keyboard: HashMap<GameAction, Vec<KeyCode>>,
    pub gamepad: HashMap<GameAction, Vec<GamepadButtonType>>,
    pub mouse: HashMap<GameAction, MouseButton>,
}

impl InputConfig {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: InputConfigToml = toml::from_str(&contents)?;
        Ok(config.into())
    }
    
    pub fn create_input_map(&self) -> InputMap<GameAction> {
        let mut input_map = InputMap::new();
        
        // Add keyboard bindings
        for (action, keys) in &self.keyboard {
            for key in keys {
                input_map.insert(*key, *action);
            }
        }
        
        // Add gamepad bindings
        for (action, buttons) in &self.gamepad {
            for button in buttons {
                input_map.insert(*button, *action);
            }
        }
        
        input_map
    }
}
```

### Input Handling Systems
```rust
pub fn handle_game_input(
    input: Res<ActionState<GameAction>>,
    mut game_state: ResMut<GameState>,
    mut next_state: ResMut<NextState<GamePhase>>,
    mut commands: Commands,
) {
    // Restart
    if input.just_pressed(GameAction::Restart) {
        commands.insert_resource(RestartRequested);
        next_state.set(GamePhase::Setup);
    }
    
    // Pause toggle
    if input.just_pressed(GameAction::Pause) {
        match game_state.phase {
            GamePhase::Playing => next_state.set(GamePhase::Paused),
            GamePhase::Paused => next_state.set(GamePhase::Playing),
            _ => {}
        }
    }
    
    // Quick restart on game over
    if input.just_pressed(GameAction::QuickRestart) {
        if matches!(game_state.phase, GamePhase::Won | GamePhase::Lost) {
            commands.insert_resource(RestartRequested);
            next_state.set(GamePhase::Setup);
        }
    }
}

pub fn handle_debug_input(
    input: Res<ActionState<GameAction>>,
    mut debug_settings: ResMut<DebugSettings>,
    mut commands: Commands,
    balls: Query<Entity, With<Ball>>,
    targets: Query<Entity, With<Target>>,
) {
    if input.just_pressed(GameAction::ToggleDebug) {
        debug_settings.show_physics = !debug_settings.show_physics;
    }
    
    if input.just_pressed(GameAction::SpawnBall) {
        spawn_debug_ball(&mut commands);
    }
    
    if input.just_pressed(GameAction::DestroyTarget) {
        if let Some(target) = targets.iter().next() {
            commands.entity(target).despawn_recursive();
        }
    }
}
```

### Gamepad Integration
```rust
pub struct GamepadManager {
    pub connected_gamepads: HashMap<usize, Gamepad>,
    pub primary_gamepad: Option<Gamepad>,
}

pub fn handle_gamepad_connections(
    mut gamepad_manager: ResMut<GamepadManager>,
    mut gamepad_events: EventReader<GamepadEvent>,
) {
    for event in gamepad_events.read() {
        match event.event_type {
            GamepadEventType::Connected(_) => {
                info!("Gamepad {} connected", event.gamepad.id);
                gamepad_manager.connected_gamepads.insert(
                    event.gamepad.id,
                    event.gamepad,
                );
                
                // Set as primary if first gamepad
                if gamepad_manager.primary_gamepad.is_none() {
                    gamepad_manager.primary_gamepad = Some(event.gamepad);
                }
            }
            GamepadEventType::Disconnected => {
                info!("Gamepad {} disconnected", event.gamepad.id);
                gamepad_manager.connected_gamepads.remove(&event.gamepad.id);
                
                // Find new primary if needed
                if gamepad_manager.primary_gamepad == Some(event.gamepad) {
                    gamepad_manager.primary_gamepad = 
                        gamepad_manager.connected_gamepads.values().next().copied();
                }
            }
            _ => {}
        }
    }
}
```

### Input Display System (Demo)
```rust
pub fn display_active_inputs(
    input: Res<ActionState<GameAction>>,
    gamepads: Res<Gamepads>,
    keys: Res<ButtonInput<KeyCode>>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    egui::Window::new("Input Status")
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Active Actions");
            
            for action in GameAction::variants() {
                if input.pressed(action) {
                    ui.label(format!("✓ {:?}", action));
                }
            }
            
            ui.separator();
            ui.heading("Raw Inputs");
            
            // Keyboard
            ui.label("Keyboard:");
            for key in keys.get_pressed() {
                ui.label(format!("  {:?}", key));
            }
            
            // Gamepad
            if let Some(gamepad) = gamepads.iter().next() {
                ui.label(format!("Gamepad {}", gamepad.id));
                // Show button states
            }
        });
}
```

## Future Input Features (Hooks)

### Paddle Control (Future Sprint)
```rust
#[derive(Component)]
pub struct Paddle {
    pub position: f32,      // -1.0 to 1.0
    pub speed: f32,
    pub width: f32,
}

pub fn handle_paddle_input(
    input: Res<ActionState<GameAction>>,
    mut paddle: Query<&mut Paddle>,
    time: Res<Time>,
) {
    if let Ok(mut paddle) = paddle.get_single_mut() {
        let move_amount = paddle.speed * time.delta_seconds();
        
        if input.pressed(GameAction::AimLeft) {
            paddle.position -= move_amount;
        }
        if input.pressed(GameAction::AimRight) {
            paddle.position += move_amount;
        }
        
        paddle.position = paddle.position.clamp(-1.0, 1.0);
    }
}
```

## Success Criteria

- ✅ All defined actions respond correctly
- ✅ Input configuration loads from file
- ✅ Gamepad support works seamlessly
- ✅ No input lag or missed inputs
- ✅ Rebinding works without restart
- ✅ Input test demo validates all features

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Input conflicts | Medium | Validation, priority system |
| Platform differences | High | Test on Windows/Mac/Linux |
| Gamepad compatibility | Medium | Standard mapping, remapping UI |
| Input lag | High | Direct polling, no buffering |

## Dependencies

### From Previous Sprints
- Sprint 5: Game states to control
- Sprint 1: Core architecture

### External Crates
- `leafwing-input-manager = "0.15"`
- `bevy_egui = "0.30"` (for demo UI)

### Assets
- Input configuration file
- Controller button icons (future)

## Definition of Done

- [ ] Input system responds to all actions
- [ ] Configuration loads from TOML
- [ ] Gamepad support verified
- [ ] Hot-reload of input config works
- [ ] No input lag or drops
- [ ] Demo shows all inputs clearly
- [ ] Rebinding interface functional
- [ ] README documents input system

## Notes for Next Sprint

Sprint 8 will add environmental effects:
- Background gradient system
- Parallax layers
- Ambient particles
- Arena atmosphere
- Dynamic lighting effects

The input system provides the player's connection to the game world that will be enhanced with atmospheric effects.
