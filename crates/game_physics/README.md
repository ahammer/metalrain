# game_physics

Foundational physics layer (Sprint 2) for the project. Integrates Rapier2D with core `Ball` ECS components, providing:

- Tunable `PhysicsConfig` resource
- Clustering / attraction force (naive O(n^2) implementation – optimize later)
- Velocity clamping for visual readability
- Sync of Rapier velocities back into `Ball`
- Ready for UI tuning via egui (implemented in the `physics_playground` demo)

## Systems Overview

Order (conceptual):
1. `apply_clustering_forces` – accumulate ExternalForce toward nearby balls
2. Rapier integration step (handled by Rapier plugin)
3. `sync_physics_to_balls` – write `Velocity.linvel` into `Ball.velocity`
4. `clamp_velocities` – enforce min/max speed constraints

## Configuration
`PhysicsConfig` fields and defaults:
```
pixels_per_meter: 50.0
gravity: (0.0, -500.0)
ball_restitution: 0.95
ball_friction: 0.1
clustering_strength: 100.0
clustering_radius: 150.0
max_ball_speed: 500.0
min_ball_speed: 100.0
```

## Demo: physics_playground
Run the interactive playground to spawn balls, adjust parameters, and observe clustering.

### Run
```
cargo run -p physics_playground
```

### Controls
- Left Click: spawn a ball at cursor with random velocity
- Sliders (egui panel): adjust gravity, restitution, friction, clustering parameters, speed limits
- Velocity gizmos: yellow lines show current linear velocity vectors

## Planned Enhancements
- Spatial partitioning / grid to reduce clustering complexity
- Pause / resume & time scaling
- Separation / anti-clump force for dense clusters
- Collision event hooks into gameplay
- Additional debug visualizations (force magnitudes, collision normals)

## Testing
Basic sanity test for `PhysicsConfig` included. Further property-based tests or performance benchmarks can be added when the API stabilizes.
