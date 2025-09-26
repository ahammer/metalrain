# game_core

Foundational ECS layer for the modular game architecture established in Sprint 1.

Provides:
- Core components: `Ball`, `Wall`, `Target`, `Hazard`
- Shared resources: `GameState`, `ArenaConfig`
- Events: `BallSpawned`, `TargetDestroyed`, `GameWon`, `GameLost`
- Bundles: `BallBundle`

## Usage
```rust
app.add_plugins(GameCorePlugin);
```

## Future Extensions
Physics (collision shapes, velocities), serialization, save/load, and color/material abstractions.
