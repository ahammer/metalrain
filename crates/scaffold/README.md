# Scaffold Integration Plugin

The `scaffold` crate packages the default Metalrain subsystems (assets, physics, rendering, diagnostics, input, HUD) into a single Bevy plugin for demos and integration tests. It lets an app start with a working layered compositor, metaball renderer, Rapier physics, and background renderer with one line of setup.

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

To brand the HUD with your demo name:

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
| Gravity | `(0, 0)` (fully adjustable at runtime) |
| Background | `BackgroundConfig::default()` |
| Diagnostics | Frame time HUD + Rapier debug lines |

## Runtime Controls

The scaffold installs consistent bindings across demos:

- `1-5`: toggle compositor layers
- `[`/`]`: adjust exposure
- `-`/`=`: zoom camera
- `Space`: camera shake impulse
- `R`: reset camera, exposure, gravity
- `P`: pause/resume physics
- `Arrow keys`: change gravity vector
- `B`: cycle background mode
- `M`: cycle metaball modes (clustered / no clustering / hidden)
- `F1`: toggle HUD visibility
- `F2`: toggle layer-boundary debug overlay
- `Esc`: exit application

## Customisation

Insert a `ScaffoldConfig` resource *before* adding the plugin to override defaults:

```rust
use scaffold::resources::ScaffoldConfig;

App::new()
    .insert_resource(ScaffoldConfig::default().with_world_half_extent(320.0))
    .add_plugins(ScaffoldIntegrationPlugin::default());
```

Additional resources exposed by the plugin (`ScaffoldHudState`, `ScaffoldMetaballMode`, `ScaffoldPerformanceStats`) allow demos to react to standard HUD state or extend diagnostics.

## Extending

- Use the exported `ScaffoldSystemSet::Input`/`::Hud` markers to order custom systems relative to scaffold input or HUD updates.
- Spawn additional visuals or walls as needed; the scaffold keeps its arena square to avoid imposing demo-specific shapes.
- The plugin avoids introducing new domain components; cross-crate behaviour remains in `game_core`, `game_physics`, and `game_rendering`.
