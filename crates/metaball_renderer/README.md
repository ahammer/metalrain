# metaball_renderer

GPU compute-based metaball rendering system producing high-quality blob visuals with field and albedo textures.

## Description

This crate implements a compute shader-based metaball renderer that generates smooth, organic-looking blob visuals. It uses a two-pass GPU pipeline: first computing the metaball field and colors, then calculating 3D normals for lighting. The output is provided as offscreen textures (field + albedo) for integration with external compositing systems.

The renderer uses world-space coordinates for game logic while automatically handling the mapping to texture space for GPU processing. It supports dynamic metaball addition/removal, per-ball colors, clustering behavior, and configurable world bounds.

## Purpose

**For Users:**

- Smooth, visually appealing blob rendering
- Dynamic metaball behavior with colors and clustering
- High performance through GPU compute shaders
- Organic visual style suitable for various game aesthetics

**For Downstream Developers:**

- Clean separation between game logic (world space) and rendering (texture space)
- Automatic coordinate mapping and radius scaling
- Offscreen rendering for flexible compositing
- Optional built-in presentation quad for simple use cases
- Coordinate utilities for mouse picking and projection
- Configurable world bounds and resolution
- Integration with multi-layer rendering pipelines

## Key API Components

### Plugin

- **`MetaballRendererPlugin`** - Main plugin with configurable settings
  - Built from `MetaballRenderSettings` for customization

### Resources

- **`MetaballRenderSettings`** - Configuration for metaball rendering
  - `world_bounds: Rect` - World-space region that maps to texture
  - `texture_width: u32` - Field/albedo texture width
  - `texture_height: u32` - Field/albedo texture height
  - `present_via_quad: bool` - Whether to spawn presentation quad
  - `presentation_layer: Option<u8>` - Render layer for presentation quad

- **`MetaballCoordinateMapper`** - Handles coordinate transformations
  - `world_to_metaball(pos: Vec3) -> Vec2` - World to texture coordinates
  - `metaball_to_uv(tex: Vec2) -> Vec2` - Texture pixels to UV [0,1]
  - `world_radius_to_tex(radius: f32) -> f32` - Scale world radius to pixels

- **`RuntimeSettings`** - Runtime-mutable rendering options
  - `clustering_enabled: bool` - Toggle clustering behavior

- **`MetaballDiagnosticsConfig`** - Performance monitoring settings
  - Logging and timing information

### Components

- **`MetaBall`** - Core metaball component (attach to Transform entity)
  - `radius_world: f32` - Radius in world units

- **`MetaBallColor`** - Optional color override for individual metaballs
  - Defaults to white if not present

- **`MetaBallCluster`** - Optional clustering behavior marker
  - Groups metaballs for visual clustering effects

### Functions

- **`metaball_textures(world: &World) -> Option<(Handle<Image>, Handle<Image>)>`**
  - Returns (field, albedo) texture handles for custom compositing

- **`project_world_to_screen(...)`** - Convert world position to screen coordinates
- **`screen_to_world(...)`** - Convert screen coordinates to world position
- **`screen_to_metaball_uv(...)`** - Convert screen coordinates to metaball texture UV

### Features

- **`present`** - (Default) Enables built-in presentation quad rendering
- **`shader_hot_reload`** - (Default) Enables shader hot-reloading during development

## Usage Example

### Basic Setup

```rust
use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings, MetaBall};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::default())
        .add_systems(Startup, spawn_metaballs)
        .run();
}

fn spawn_metaballs(mut commands: Commands) {
    // Spawn metaballs using Transform for world position
    for i in 0..20 {
        let angle = i as f32 * std::f32::consts::TAU / 20.0;
        let radius = 150.0;
        let x = angle.cos() * radius;
        let y = angle.sin() * radius;
        
        commands.spawn((
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            GlobalTransform::default(),
            MetaBall { radius_world: 20.0 },
        ));
    }
}
```

### Custom Configuration

```rust
use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings};

fn main() {
    let settings = MetaballRenderSettings::default()
        // Set world bounds that match your game area
        .with_world_bounds(Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(800.0, 600.0),
        ))
        // Enable built-in presentation quad
        .with_presentation(true)
        // Assign to compositor layer 2 (for multi-layer pipeline)
        .with_presentation_layer(Some(2))
        // Custom texture resolution
        .with_resolution(1024, 768);
    
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::with(settings))
        .run();
}
```

### Colored Metaballs

```rust
use bevy::prelude::*;
use metaball_renderer::{MetaBall, MetaBallColor};

fn spawn_colored_metaballs(mut commands: Commands) {
    // Red metaball
    commands.spawn((
        Transform::from_xyz(-100.0, 0.0, 0.0),
        GlobalTransform::default(),
        MetaBall { radius_world: 25.0 },
        MetaBallColor(Color::srgb(1.0, 0.2, 0.2)),
    ));
    
    // Blue metaball
    commands.spawn((
        Transform::from_xyz(100.0, 0.0, 0.0),
        GlobalTransform::default(),
        MetaBall { radius_world: 25.0 },
        MetaBallColor(Color::srgb(0.2, 0.2, 1.0)),
    ));
}
```

### Dynamic Metaballs

```rust
use bevy::prelude::*;
use metaball_renderer::MetaBall;

fn spawn_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            let (camera, camera_transform) = camera.single();
            
            // Convert screen to world coordinates
            if let Ok(world_pos) = camera.viewport_to_world_2d(
                camera_transform,
                cursor_pos,
            ) {
                commands.spawn((
                    Transform::from_translation(world_pos.extend(0.0)),
                    GlobalTransform::default(),
                    MetaBall { radius_world: 15.0 },
                ));
            }
        }
    }
}
```

### Custom Compositing

```rust
use bevy::prelude::*;
use metaball_renderer::metaball_textures;

fn access_metaball_textures(world: &World) {
    if let Some((field_handle, albedo_handle)) = metaball_textures(world) {
        // Use these handles in custom materials or render passes
        // field_handle contains the metaball field (distance values)
        // albedo_handle contains colored metaball output with normals
    }
}
```

### Mouse Picking

```rust
use bevy::prelude::*;
use metaball_renderer::screen_to_world;

fn check_metaball_click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    metaballs: Query<(&Transform, &MetaBall)>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let window = windows.single();
        if let Some(cursor_pos) = window.cursor_position() {
            let (camera, camera_transform) = camera.single();
            
            if let Some(world_pos) = screen_to_world(
                cursor_pos,
                camera,
                camera_transform,
            ) {
                // Check if click is inside any metaball
                for (transform, metaball) in &metaballs {
                    let distance = world_pos.distance(transform.translation.truncate());
                    if distance < metaball.radius_world {
                        info!("Clicked metaball at {:?}", transform.translation);
                    }
                }
            }
        }
    }
}
```

## Coordinate System Architecture

The renderer operates in multiple coordinate spaces:

1. **World Space**: Game logic coordinates (arbitrary units and bounds)
2. **Texture Pixels**: GPU texture space [0..W, 0..H]
3. **UV Space**: Normalized texture coordinates [0..1]
4. **Screen Space**: Final display coordinates

The `MetaballCoordinateMapper` automatically handles transformations between these spaces based on the configured `world_bounds`.

### Migration from Pre-2.1

**Old approach (manual texture coordinates):**

```rust
commands.spawn(MetaBall {
    center: world_to_tex(pos),
    radius: r,
});
```

**New approach (automatic via Transform):**

```rust
commands.spawn((
    Transform::from_translation(pos.extend(0.0)),
    GlobalTransform::default(),
    MetaBall { radius_world: r },
));
```

The packing system automatically maps `Transform` → texture coordinates each frame.

## Rendering Pipeline

### Compute Pass 1: Field Generation

- Reads packed metaball data (position, radius, color)
- Computes metaball field values for each texture pixel
- Outputs field texture + initial albedo

### Compute Pass 2: Normal Calculation

- Samples field texture to compute 3D normals
- Applies lighting based on normals
- Outputs final lit albedo texture

### Optional Presentation

When the `present` feature is enabled and `present_via_quad` is true:

- Spawns a fullscreen quad with the metaball material
- Can be assigned to specific compositor layer
- Suitable for simple use cases or debugging

## Performance Characteristics

- GPU compute-based: O(1) with respect to metaball count (within workgroup limits)
- Packing system runs when metaballs are added/removed or transforms change
- Compute passes run every frame
- Texture resolution directly impacts performance
- Consider lower resolution for high metaball counts or mobile targets

## Configuration Best Practices

### World Bounds

Set `world_bounds` to match your playable game area:

```rust
// For a game with visible area of 800×600 world units
.with_world_bounds(Rect::from_center_size(Vec2::ZERO, Vec2::new(800.0, 600.0)))
```

### Resolution

Higher resolution = sharper metaballs but lower performance:

```rust
// High quality (desktop)
.with_resolution(1920, 1080)

// Medium quality (mobile)
.with_resolution(1280, 720)

// Performance mode
.with_resolution(854, 480)
```

### Presentation Layer

For multi-layer rendering pipelines, explicitly set the layer:

```rust
.with_presentation(true)
.with_presentation_layer(Some(2)) // Layer 2 = Metaballs in game_rendering
```

## Dependencies

- `bevy` - Core engine and rendering
- `bytemuck` - Zero-copy type casting for GPU buffers
- `static_assertions` - Compile-time size verification
- `game_assets` - Shader asset management

## Dev Dependencies

- `criterion` - Benchmarking coordinate transformations

## Testing

The crate includes tests for:

- Coordinate mapping correctness
- World bounds validation
- Radius scaling
- UV coordinate clamping

Run tests with:

```bash
cargo test -p metaball_renderer
```

### Benchmark Coordinate Performance

```bash
cargo bench -p metaball_renderer
```

## Diagnostics

Enable diagnostics for performance monitoring:

```rust
use metaball_renderer::{MetaballDiagnosticsPlugin, MetaballDiagnosticsConfig};

app.add_plugins(MetaballDiagnosticsPlugin {
    config: MetaballDiagnosticsConfig {
        log_packing_time: true,
        log_render_time: true,
    },
});
```

## Known Limitations

- Maximum metaballs limited by GPU buffer size and workgroup limits
- Field quality degrades at very high metaball counts
- No spatial culling (all metaballs processed even if off-screen)
- Fixed resolution (no automatic LOD scaling)

## Future Enhancements

- Dynamic resolution/LOD based on metaball count
- Spatial culling for off-screen metaballs
- Optimized clustering via spatial partitioning
- Additional visual effects (glow, trails)
- Soft shadows and ambient occlusion
- Export field data for gameplay queries

## License

Dual-licensed under MIT or Apache-2.0.
