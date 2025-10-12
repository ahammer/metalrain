# Sprint 5: Gameplay Loop & Win/Lose Conditions

## Sprint Goal
Implement the core game loop with win/lose conditions, ball-target collision handling, hazard elimination mechanics, and game state management. Create a fully playable game experience.

## Deliverables

### 1. Gameplay Systems Crate (`game_gameplay`)
- [ ] Create `game_gameplay` crate structure
- [ ] Implement collision event processing
- [ ] Add win/lose condition checking
- [ ] Create game state machine
- [ ] Build round management system

### 2. Collision Response Systems
- [ ] Ball-to-target hit detection
- [ ] Target health reduction and destruction
- [ ] Ball-to-hazard elimination
- [ ] Ball-to-wall bounce validation
- [ ] Collision feedback events

### 3. Game State Management
- [ ] Setup → Playing → Won/Lost flow
- [ ] Ball counter tracking
- [ ] Target counter tracking
- [ ] Round timer (optional)
- [ ] State transition animations

### 4. Win/Lose Mechanics
- [ ] Victory: All targets destroyed, balls remain
- [ ] Defeat: No balls left, targets remain
- [ ] Immediate feedback on outcome
- [ ] Quick restart capability
- [ ] Statistics tracking

### 5. Demo: Complete Game Loop
- [ ] Full round from start to finish
- [ ] Win and lose scenarios
- [ ] Restart functionality
- [ ] Different difficulty setups
- [ ] Gameplay metrics display

## Technical Specifications

### Game States
```rust
#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GamePhase {
    #[default]
    Setup,      // Initial load, asset preparation
    Ready,      // Level loaded, waiting to start
    Playing,    // Active gameplay
    Won,        // Victory screen
    Lost,       // Defeat screen
    Paused,     // Gameplay suspended
}

pub struct GameState {
    pub balls_remaining: u32,
    pub balls_lost: u32,
    pub targets_remaining: u32,
    pub targets_destroyed: u32,
    pub play_time: f32,
    pub phase: GamePhase,
}
```

### Collision Event Processing
```rust
pub fn handle_ball_target_collision(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut targets: Query<&mut Target>,
    balls: Query<Entity, With<Ball>>,
    mut game_state: ResMut<GameState>,
    mut target_destroyed: EventWriter<TargetDestroyed>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Check if ball hit target
            if let (Ok(_ball), Ok(mut target)) = 
                (balls.get(*e1), targets.get_mut(*e2)) {
                
                target.health = target.health.saturating_sub(1);
                
                if target.health == 0 {
                    // Destroy target
                    commands.entity(*e2).despawn_recursive();
                    game_state.targets_remaining -= 1;
                    game_state.targets_destroyed += 1;
                    target_destroyed.send(TargetDestroyed(*e2));
                    
                    // Trigger visual feedback
                    spawn_destruction_particles(&mut commands, target.position);
                    trigger_screen_shake(0.2);
                }
            }
        }
    }
}
```

### Hazard Elimination System
```rust
pub fn handle_ball_hazard_contact(
    mut commands: Commands,
    hazards: Query<&Hazard>,
    balls: Query<(Entity, &Transform), With<Ball>>,
    mut game_state: ResMut<GameState>,
    mut ball_lost: EventWriter<BallLost>,
) {
    for (ball_entity, ball_transform) in balls.iter() {
        for hazard in hazards.iter() {
            if hazard.bounds.contains(ball_transform.translation.xy()) {
                // Remove ball
                commands.entity(ball_entity).despawn_recursive();
                game_state.balls_remaining -= 1;
                game_state.balls_lost += 1;
                ball_lost.send(BallLost(ball_entity));
                
                // Visual feedback
                spawn_elimination_effect(&mut commands, ball_transform.translation);
            }
        }
    }
}
```

### Win/Lose Condition Checking
```rust
pub fn check_win_condition(
    game_state: Res<GameState>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut game_won: EventWriter<GameWon>,
) {
    if game_state.targets_remaining == 0 && game_state.balls_remaining > 0 {
        next_phase.set(GamePhase::Won);
        game_won.send(GameWon {
            balls_saved: game_state.balls_remaining,
            time_taken: game_state.play_time,
        });
    }
}

pub fn check_lose_condition(
    game_state: Res<GameState>,
    mut next_phase: ResMut<NextState<GamePhase>>,
    mut game_lost: EventWriter<GameLost>,
) {
    if game_state.balls_remaining == 0 && game_state.targets_remaining > 0 {
        next_phase.set(GamePhase::Lost);
        game_lost.send(GameLost {
            targets_left: game_state.targets_remaining,
            balls_lost: game_state.balls_lost,
        });
    }
}
```

### Round Management
```rust
pub struct RoundManager;

impl RoundManager {
    pub fn start_round(
        mut commands: Commands,
        mut game_state: ResMut<GameState>,
        level_data: Res<LevelData>,
    ) {
        // Reset state
        *game_state = GameState {
            balls_remaining: level_data.initial_balls,
            targets_remaining: level_data.targets.len() as u32,
            ..default()
        };
        
        // Spawn entities
        spawn_level_entities(&mut commands, &level_data);
        
        // Start gameplay
        commands.insert_resource(NextState(GamePhase::Playing));
    }
    
    pub fn restart_round(
        mut commands: Commands,
        entities: Query<Entity, Or<(With<Ball>, With<Target>, With<Hazard>)>>,
    ) {
        // Clean up existing entities
        for entity in entities.iter() {
            commands.entity(entity).despawn_recursive();
        }
        
        // Trigger new round start
        commands.insert_resource(NextState(GamePhase::Setup));
    }
}
```

### Gameplay Configuration
```toml
[gameplay]
# Ball settings
initial_balls = 3
ball_spawn_delay = 0.5

# Target settings
target_base_health = 1
target_score_value = 100

# Hazard settings
hazard_instant_kill = true
hazard_pull_strength = 200.0

# Timing
round_time_limit = 0  # 0 = unlimited
restart_delay = 1.0

# Feedback
screen_shake_enabled = true
particle_effects_enabled = true
```

## Demo Features

### Test Scenarios
1. **Quick Win**: Few targets, many balls
2. **Challenge**: Many targets, limited balls
3. **Hazard Heavy**: Multiple danger zones
4. **Time Pressure**: Optional timer mode
5. **Perfect Run**: Complete without losing balls

### Debug Commands
- **F1**: Instant win
- **F2**: Instant lose
- **F3**: Add extra ball
- **F4**: Destroy random target
- **F5**: Toggle god mode (balls immune)
- **F6**: Show collision boxes

### Metrics Display
```
╭─────────────────╮
│ Balls: ●●● (3)  │
│ Targets: 12     │
│ Time: 0:23      │
│ Lost: 2         │
╰─────────────────╯
```

## Success Criteria

- ✅ Complete game loop functions correctly
- ✅ Win condition triggers appropriately
- ✅ Lose condition triggers appropriately
- ✅ Collision responses feel satisfying
- ✅ Quick restart works smoothly
- ✅ No stuck states or soft-locks

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Edge case soft-locks | High | Extensive state testing |
| Collision misses | High | Continuous collision detection |
| State transition bugs | Medium | Clear state machine, logging |
| Gameplay balance | Medium | Tunable parameters, playtesting |

## Dependencies

### From Previous Sprints
- Sprint 1: Core components and events
- Sprint 2: Physics and collision detection
- Sprint 3: Rendering for visual feedback
- Sprint 4: Widget elements (targets, hazards)

### External Crates
- Already included via Bevy

### Assets
- Victory/defeat sound placeholders
- Particle effect textures

## Definition of Done

- [ ] Full game loop playable start to finish
- [ ] Win scenario works correctly
- [ ] Lose scenario works correctly
- [ ] All collisions handled properly
- [ ] State transitions smooth
- [ ] Restart functionality immediate
- [ ] Demo shows various scenarios
- [ ] No gameplay bugs or soft-locks
- [ ] README documents game flow

## Notes for Next Sprint

Sprint 6 will add level system:
- Level data format definition
- Level loading from files
- Multiple level progression
- Level selection menu
- Difficulty progression

The gameplay loop from this sprint forms the core experience that levels will build upon.
