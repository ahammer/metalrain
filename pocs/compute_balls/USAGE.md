# Metaball Renderer Usage

This proof-of-concept has been refactored into a reusable Bevy plugin. Add the plugin to your app to get a GPU compute based metaball renderer with an optional built‑in CPU simulation and fullscreen presentation material.

## Quick Start

```rust
use bevy::prelude::*;
use compute_balls::MetaballRendererPlugin; // adjust path if extracted to its own crate

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::default())
        .run();
}
```

This initializes:
* A compute pipeline dispatching the `compute_metaballs.wgsl` shader every frame.
* Two storage textures: a 16F field/normal output and an RGBA8 albedo output.
* A simple bouncy ball simulation feeding per‑ball (position, radius, color) data into a storage buffer.
* A fullscreen quad + material (`present_fullscreen.wgsl`) sampling the compute outputs.

## Feature Settings

`MetaballRendererSettings` allows enabling/disabling internal subsystems:

```rust
use compute_balls::{MetaballRendererPlugin, MetaballRendererSettings};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(MetaballRendererPlugin::with_settings(MetaballRendererSettings {
        with_simulation: false, // supply your own balls
        with_present: true,     // still use built-in fullscreen display
    }))
    .run();
```

If you disable `with_simulation`, you must populate the `BallBuffer` resource yourself.

## Providing Your Own Balls

Access the GPU-bound ball data via the `BallBuffer` resource (re-exported from `metaball.rs`). Update its `balls` vector each frame (or at your own cadence) and set `ParamsUniform.num_balls` accordingly:

```rust
use bevy::prelude::*;
use compute_balls::compute::types::{Ball, BallBuffer as GpuBallBuffer, ParamsUniform};

fn custom_update(mut buf: ResMut<GpuBallBuffer>, mut params: ResMut<ParamsUniform>) {
    buf.balls.clear();
    buf.balls.push(Ball {
        center: [200.0, 300.0],
        radius: 25.0,
        cluster_id: 0,
        color: [1.0, 0.2, 0.6, 1.0],
    });
    params.num_balls = buf.balls.len() as u32;
}
```

Register your system in `Update` when using `with_simulation: false`.

## Per-Ball Color Effects

Each `Ball` carries a `color: [f32;4]`. The compute shader writes both the density field (16F texture) and an albedo (RGBA8) to allow later composition / post effects. Modify colors per frame for glow / pulsation / cluster highlighting.

## Debug Inputs (Default Simulation)

When the internal simulation is enabled:
* `F` cycles debug render modes (shader dependent).
* `C` toggles center visualization (if supported by shader mode).
* `G` toggles gravity.
* `S` toggles syncing simulation to GPU (benchmarking aid).

## Coordinate Mapping

The CPU simulation uses a logical world of `(-HALF_EXTENT..HALF_EXTENT)` in both axes and maps into screen pixel coordinates. If you provide your own positions directly in pixel space, keep them inside the texture dimensions (`WIDTH x HEIGHT`) defined in `constants.rs`.

## Integration Notes

* Extract this `pocs/compute_balls` directory into its own crate for reuse. Expose a `lib.rs` that `pub use`s the plugin and relevant types.
* For dynamic resolutions, add a resize system that recreates the storage textures and updates `ParamsUniform.screen_size`.
* To remove the fullscreen quad and integrate into a larger composition, disable `with_present` and sample the `MetaballOutputTexture` + `MetaballAlbedoTexture` from your own material/shader.
* Storage buffer sizing is fixed by `MAX_BALLS` (see `constants.rs`). Ensure your custom ball count never exceeds this or refactor to a runtime-sized storage buffer.

## Extending

Potential enhancements when promoting to a crate:
* Config struct (resolution, iso, workgroup size) + builder pattern.
* Runtime resize support via events.
* Optional normal reconstruction + lighting pass.
* GPU-driven spawning (compute writes next frame's positions).
* Parallel CPU simulation feature gate.

## Minimum Bevy Version

Tested with the Bevy revision included as a submodule in this repository (ensure you track the same commit for compatibility with render internals).

---
Feel free to open issues / PRs when this becomes a standalone crate.
