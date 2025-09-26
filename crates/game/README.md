# game

Integration crate assembling core gameplay building blocks with rendering (metaball renderer) and higher-level orchestration.

Currently:
- Adds `GameCorePlugin`
- Spawns a demo ball
- Emits a `GameWon` event after a short timer (placeholder logic)

## Usage
```rust
App::new()
  .add_plugins((DefaultPlugins, GamePlugin))
  .run();
```

## Roadmap
- Tie into physics crate (future `game_physics`)
- Real win/lose conditions
- Level / target spawning systems
