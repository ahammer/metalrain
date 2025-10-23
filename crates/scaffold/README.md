# Scaffold Integration Plugin

The `scaffold` crate packages the default Metalrain subsystems (assets, physics, rendering) into a single Bevy plugin for demos and integration tests. It provides a "blank canvas" with a working layered compositor, metaball renderer, Rapier physics, and background renderer with one line of setup.

**Note:** The scaffold intentionally does NOT include input handling, HUD, or keybindings. These UI concerns belong in individual demos or higher-level integrations.

## Quick Start

```rust
use bevy::prelude::*;
use scaffold::ScaffoldIntegrationPlugin;

fn main() {
    App::new()
        .add_plugins(ScaffoldIntegrationPlugin::default())
        // add demo-specific systems/resources
        .run();
}
```

To set a demo name for metadata tracking:

```rust
App::new().add_plugins(ScaffoldIntegrationPlugin::with_demo_name("compositor_test"));
```

## Defaults

| Concern | Default |
| --- | --- |
| Base render resolution | `1280 x 720` |
| Metaball texture size | `512 x 512` |
| World bounds | square `[-256, 256]` in X/Y |
| Wall thickness | `10` |
| Gravity | `(0, 0)` |
| Background | `BackgroundConfig::default()` |
| Debug rendering | Rapier debug lines (native only) |

## Customisation

Insert a `ScaffoldConfig` resource *before* adding the plugin to override defaults:

```rust
use scaffold::resources::ScaffoldConfig;

App::new()
    .insert_resource(ScaffoldConfig::default().with_world_half_extent(320.0))
    .add_plugins(ScaffoldIntegrationPlugin::default());
```

## Extending

- Demos should implement their own input handling systems as needed.
- Spawn additional visuals or walls as needed; the scaffold keeps its arena square to avoid imposing demo-specific shapes.
- The plugin avoids introducing new domain components; cross-crate behaviour remains in `game_core`, `game_physics`, and `game_rendering`.
