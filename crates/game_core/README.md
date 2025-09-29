# game_core

Foundational ECS layer for the modular game architecture established in Sprint 1.

Provides:

- Core components: `Ball`, `Wall`, `Target`, `Hazard`, `Paddle`, `SpawnPoint`
- Shared resources: `GameState`, `ArenaConfig`, spawn resources (`BallSpawnPolicy`, `ActiveSpawnRotation`, `SpawnMetrics`)
- Events: `BallSpawned`, `TargetDestroyed`, `GameWon`, `GameLost`, `SpawnBallEvent`
- Bundles: `BallBundle`
- Plugins: `PaddlePlugin`, `SpawningPlugin`

## Usage

```rust
app.add_plugins((GameCorePlugin, PaddlePlugin, SpawningPlugin));
```

To emit a spawn event manually (e.g. input system):

```rust
fn manual_spawn(mut w: EventWriter<SpawnBallEvent>, rotation: Res<ActiveSpawnRotation>) {
    if let Some(spawn_e) = rotation.current_entity() {
        w.write(SpawnBallEvent { spawn_entity: spawn_e, override_position: None });
    }
}
```

## Future Extensions

Upcoming: richer gameplay state, scoring, AI paddle control, spawn weighting & policies, serialization, save/load, and color/material abstractions. When the `GamePhysicsPlugin` is present, `Paddle` entities automatically receive kinematic Rapier bodies and colliders for proper collision response.
