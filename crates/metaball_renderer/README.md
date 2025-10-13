# Metaball Renderer

High-performance, spatially-accelerated metaball rendering for Bevy, optimized for dynamic ball creation and destruction.

## Features

- **Spatial Acceleration**: Uniform grid partitioning reduces per-pixel complexity from O(N) to O(k)
- **Fixed-Capacity GPU Buffers**: Zero reallocations after initialization, eliminating pipeline stalls
- **Dynamic Ball Management**: Efficient free-list-based slot reuse for rapid creation/destruction
- **Clustering Support**: Optional dominant-cluster rendering for visual coherence
- **Gradient-Packed Output**: Single-pass computation of field value, gradient, and signed distance

## Performance

For a typical scenario (512×512 texture, 200 balls):

- **Before**: 52.4M ball evaluations per frame
- **After**: 786K ball evaluations per frame (~67× reduction)
- **Memory**: Zero GPU allocations after startup
- **Scalability**: Frame time remains stable as total ball count increases (depends on local density)

## Usage

```rust
use bevy::prelude::*;
use metaball_renderer::{
    MetaBall, MetaBallColor, MetaballRendererPlugin, MetaballRenderSettings,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin)
        .insert_resource(MetaballRenderSettings {
            texture_size: UVec2::new(512, 512),
            enable_clustering: true,
        })
        .add_systems(Startup, spawn_metaballs)
        .run();
}

fn spawn_metaballs(mut commands: Commands) {
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        MetaBall { radius_world: 20.0 },
        MetaBallColor(LinearRgba::new(1.0, 0.5, 0.2, 1.0)),
    ));
}
```

## Architecture

### Spatial Grid

The renderer divides screen space into a uniform grid (64×64 pixel cells by default). Each frame:

1. **CPU**: Build spatial index mapping grid cells to influencing balls
2. **GPU**: Each pixel queries only balls in its cell (typically 5-20 instead of 100-1000)

### Fixed-Capacity Buffers

- Ball data lives in a pre-allocated GPU buffer (4096 balls max)
- CPU maintains a free-list for slot reuse
- No GPU memory reallocations during gameplay

### Shader Pipeline

1. **Compute Pass**: Calculate metaball field, gradient, and color per pixel
2. **Output**: RGBA16Float texture with packed gradient data
3. **Composition**: Blend with game world in main render pass

## Configuration

```rust
MetaballRenderSettings {
    /// Output texture dimensions (affects grid size)
    texture_size: UVec2::new(1024, 1024),
    
    /// Enable cluster-based rendering
    enable_clustering: true,
}
```

## Coordinate Systems

- **World Space**: Game logic coordinates (Bevy Transform)
- **Metaball Space**: Normalized texture coordinates [0, texture_size]
- **Grid Space**: Cell indices [0, grid_dimensions]

Use `MetaballCoordinateMapper` for conversions.

## Limitations

- Maximum 4096 balls (configurable via `MAX_BALLS` constant)
- Grid cell size fixed at 64×64 pixels
- WebGPU-only (uses compute shaders with storage buffers)

## Advanced Topics

See [OPTIMIZATION.md](./OPTIMIZATION.md) for:

- Detailed performance analysis
- Implementation architecture
- Benchmarking methodology
- Future optimization opportunities

## Testing

```bash
# Unit tests
cargo test -p metaball_renderer

# Visual validation
cargo run -p metaballs_test

# Benchmarks (TODO)
cargo bench -p metaball_renderer
```

## Dependencies

- `bevy`: Game engine framework
- `game_assets`: Centralized shader asset management
- `bytemuck`: Safe zero-copy type casting for GPU data

## License

See workspace LICENSE file.
