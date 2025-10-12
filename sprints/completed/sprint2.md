# Sprint 2: Physics Foundation & Ball Behavior

## Sprint Goal

Implement core physics systems using Rapier2D, establish ball movement mechanics with proper bouncing and clustering behavior. Create a physics playground demo to validate and tune physics parameters.

## Deliverables

### 1. Physics Crate (`game_physics`)

- [x] Create `game_physics` crate structure
- [x] Integrate Rapier2D physics engine (gravity applied via custom system pending direct config alignment)
- [x] Implement physics-to-Ball component sync (velocity -> `Ball.velocity`)
- [x] Add clustering force system (spatial hash optimized; fallback naive)
- [x] Create collision event handling (ball-ball logging groundwork)

### 2. Ball Movement Systems

- [x] Velocity-based movement with Rapier `RigidBody::Dynamic`
- [x] Configurable restitution (per-ball `Restitution` component)
- [x] Friction and damping parameters
- [x] Gravity influence (custom external gravity force; arena configurability deferred)
- [x] Speed clamping to maintain readability

### 3. Clustering Behavior

- [x] Distance-based attraction between balls
- [x] Configurable clustering strength and radius (`PhysicsConfig`)
- [x] Visual feedback through metaball renderer (speed-based color gradient)
- [x] Performance optimization for many balls (spatial hash grid)

### 4. Demo: Physics Playground

- [x] Interactive ball spawning (cursor-based LMB + directional RMB, reset & pause controls)
- [x] Real-time parameter adjustment (keyboard + on-screen text overlay; egui deferred until compatible version)
- [x] Visual debugging for forces and velocities (gizmos active)
- [x] Stress test with 50+ balls (T key spawns to 60; logs FPS)
- [x] Wall collision testing environment (static boundary colliders)

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

- `bevy = "0.16.1"` (workspace)
- `bevy_rapier2d = "0.30"` (current; gravity resource API divergence noted)
- `bevy_egui = "0.30"` (integrated earlier; temporarily disabled in demo while resolving dual Bevy version issue)

### Assets

- Test arena layouts (walls)
- Debug font for UI

## Definition of Done (Reconciled)

| Criterion | Status | Notes |
|-----------|--------|-------|
| Physics playground demo compiles & runs | ✅ | Interactive controls active |
| 50+ balls spawn without crashes | ✅ | Stress test harness validates stability |
| Clustering behavior looks natural | ✅ | Optimized force remains smooth |
| All physics parameters tunable live | ✅ | Keyboard adjustments + overlay (egui pending version alignment) |
| No physics explosions / instability | ✅ | Stable under stress |
| Debug visualizations (forces, velocities) | ✅ | Gizmos for velocity + clustering radius |
| Code follows Sprint 1 patterns | ✅ | Consistent modular plugins |
| README documents physics systems | ✅ | `crates/game_physics/README.md` added |

All originally deferred items were implemented (UI via simplified keyboard overlay instead of egui panel due to version alignment constraints) to fully meet and slightly expand the initial Sprint 2 scope.

Sprint 2 is now COMPLETE including enhancements (UI, debug gizmos, collision events, performance optimization, visual feedback, stress testing).

## Sprint Outcome Summary

Delivered a functioning `game_physics` crate with:

- `PhysicsConfig` resource (gravity, restitution, friction, clustering params, speed limits).
- Rapier integration (custom gravity application system due to config API/version mismatch).
- Clustering force system (O(n^2)).
- Velocity clamping and synchronization back to `Ball` component.
- Demo with boundary walls and spawning hook.

## Follow-Up Backlog (New / Extended)

1. Further clustering optimization (parallelism / SIMD) for 200+ balls.
2. Rich collision events (effects, sound hooks, impulse-based color flash).
3. Persist & hot-reload physics config profiles.
4. Optional separation / anti-clump repulsion component.
5. Automated benchmark export (CSV frame metrics over N seconds).

## Notes for Next Sprint

Sprint 3 will establish the rendering pipeline:

- Create `game_rendering` orchestrator crate
- Set up render layers and compositing
- Integrate existing metaball renderer
- Prepare for widget renderer addition
- Camera system and viewport management

The physics foundation from this sprint will be essential for all future gameplay, so thorough testing and tuning is critical.
