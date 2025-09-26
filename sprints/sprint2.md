# Sprint 2: Physics Foundation & Ball Behavior

## Sprint Goal
Implement core physics systems using Rapier2D, establish ball movement mechanics with proper bouncing and clustering behavior. Create a physics playground demo to validate and tune physics parameters.

## Deliverables

### 1. Physics Crate (`game_physics`)
- [ ] Create `game_physics` crate structure
- [ ] Integrate Rapier2D physics engine
- [ ] Implement physics-to-Ball component sync
- [ ] Add clustering force system for metaball attraction
- [ ] Create collision event handling

### 2. Ball Movement Systems
- [ ] Velocity-based movement with Rapier RigidBody
- [ ] Configurable restitution (bounciness)
- [ ] Friction and damping parameters
- [ ] Gravity influence (configurable per arena)
- [ ] Speed clamping to maintain readability

### 3. Clustering Behavior
- [ ] Distance-based attraction between balls
- [ ] Configurable clustering strength and radius
- [ ] Visual feedback through metaball renderer
- [ ] Performance optimization for many balls

### 4. Demo: Physics Playground
- [ ] Interactive ball spawning with mouse click
- [ ] Real-time parameter adjustment UI
- [ ] Visual debugging for forces and velocities
- [ ] Stress test with 50+ balls
- [ ] Wall collision testing environment

## Technical Specifications

### Physics Configuration
```rust
pub struct PhysicsConfig {
    pub pixels_per_meter: f32,        // Default: 50.0
    pub gravity: Vec2,                // Default: (0.0, -500.0)
    pub ball_restitution: f32,        // Default: 0.95
    pub ball_friction: f32,           // Default: 0.1
    pub clustering_strength: f32,     // Default: 100.0
    pub clustering_radius: f32,       // Default: 150.0
    pub max_ball_speed: f32,          // Default: 500.0
    pub min_ball_speed: f32,          // Default: 100.0
}
```

### System Pipeline
```rust
// Update order
app.add_systems(Update, (
    // Input
    handle_spawn_input,
    
    // Physics
    apply_clustering_forces,
    apply_external_forces,
    
    // Rapier steps (automatic)
    // ...
    
    // Sync
    sync_physics_to_balls,
    clamp_velocities,
    
    // Rendering
    sync_balls_to_metaballs,
).chain());
```

### Clustering Algorithm
```rust
fn apply_clustering_forces(
    mut balls: Query<(&Transform, &mut ExternalForce), With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    let positions: Vec<Vec2> = balls.iter().map(|(t, _)| t.translation.xy()).collect();
    
    for (i, (transform, mut force)) in balls.iter_mut().enumerate() {
        let my_pos = transform.translation.xy();
        let mut cluster_force = Vec2::ZERO;
        
        for (j, other_pos) in positions.iter().enumerate() {
            if i != j {
                let distance = my_pos.distance(*other_pos);
                if distance < config.clustering_radius {
                    let direction = (*other_pos - my_pos).normalize();
                    let strength = (1.0 - distance / config.clustering_radius) 
                                   * config.clustering_strength;
                    cluster_force += direction * strength;
                }
            }
        }
        
        force.force = cluster_force;
    }
}
```

### Ball Bundle
```rust
#[derive(Bundle)]
pub struct BallBundle {
    // Core
    pub ball: Ball,
    
    // Physics
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub restitution: Restitution,
    pub friction: Friction,
    pub external_force: ExternalForce,
    pub collision_groups: CollisionGroups,
    
    // Rendering
    pub metaball: Metaball,
    
    // Transform
    #[bundle]
    pub spatial: SpatialBundle,
}
```

## Physics Playground Features

### Interactive Controls
- **Left Click**: Spawn ball at cursor with random velocity
- **Right Click**: Spawn ball with velocity toward cursor
- **Space**: Pause/resume physics
- **R**: Reset all balls
- **G**: Toggle gravity
- **Arrow Keys**: Adjust gravity direction

### Debug Visualizations
- Velocity vectors (arrows)
- Clustering force fields (gradient circles)
- Collision normals (on impact)
- Speed indicators (color coding)
- FPS and physics step counter

### Parameter Sliders (egui)
- Gravity strength (-1000 to 1000)
- Restitution (0.0 to 1.0)
- Friction (0.0 to 1.0)
- Clustering strength (0 to 500)
- Clustering radius (50 to 300)
- Time scale (0.1 to 2.0)

## Performance Requirements

- Maintain 60 FPS with 50 balls
- Physics step: < 8ms
- Clustering calculation: < 2ms
- Total frame time: < 16ms

## Success Criteria

- ✅ Balls bounce naturally off walls
- ✅ Clustering creates organic blob movement
- ✅ No physics explosions or instability
- ✅ Speed remains readable (100-500 px/s)
- ✅ Parameters adjustable in real-time
- ✅ Stress test passes with 50+ balls

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Physics instability | High | Conservative timestep, extensive testing |
| Performance with many balls | High | Spatial partitioning, optimization |
| Unrealistic bouncing | Medium | Careful tuning, playtesting |
| Clustering causes clumping | Medium | Force limits, separation distance |

## Dependencies

### From Sprint 1
- `game_core` crate with Ball component
- Workspace structure established

### External Crates
- `bevy_rapier2d = "0.27"`
- `bevy_egui = "0.30"` (for debug UI)

### Assets
- Test arena layouts (walls)
- Debug font for UI

## Definition of Done

- [ ] Physics playground demo runs at 60 FPS
- [ ] 50+ balls can be spawned without crashes
- [ ] Clustering behavior looks natural
- [ ] All physics parameters are tunable
- [ ] No physics explosions or glitches
- [ ] Debug visualizations work correctly
- [ ] Code follows established patterns from Sprint 1
- [ ] README.md documents physics systems

## Notes for Next Sprint

Sprint 3 will establish the rendering pipeline:
- Create `game_rendering` orchestrator crate
- Set up render layers and compositing
- Integrate existing metaball renderer
- Prepare for widget renderer addition
- Camera system and viewport management

The physics foundation from this sprint will be essential for all future gameplay, so thorough testing and tuning is critical.
