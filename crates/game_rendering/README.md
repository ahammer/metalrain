# game_rendering

Multi-layer rendering pipeline with compositor and camera management for the metalrain game.

## Description

This crate implements a sophisticated multi-layer rendering system built on Sprint 3's architecture. It manages per-layer render targets, a compositor that blends layers together, and a flexible camera system with effects like shake and zoom. The system separates game content into distinct render layers (Background, GameWorld, Metaballs, UI) that are composited together for the final frame.

The architecture enables independent rendering of different visual layers, dynamic layer toggling, per-layer blend modes, and post-processing effects.

## Purpose

**For Users:**

- High-quality visual composition with layered rendering
- Camera effects like shake and zoom for game feel
- Dynamic visual effects through layer blending
- Smooth rendering performance through optimized render targets

**For Downstream Developers:**

- Clean abstraction for multi-layer rendering
- Per-layer render targets for advanced effects
- Flexible camera system with command-based effects
- Integration with Bevy's render graph
- Runtime-configurable compositor settings
- Support for toggling layers and blend modes

## Key API Components

### Plugin

- **`GameRenderingPlugin`** - Main plugin that sets up the rendering pipeline
  - Configures render targets for each layer
  - Sets up compositor pass
  - Registers camera systems
  - Handles window resize events

### Resources

- **`RenderTargets`** - Container for all layer render targets
  - Internal management of layer-specific render texture allocations

- **`RenderTargetHandles`** - Publicly accessible handles to render textures
  - Access layer textures for custom rendering or effects

- **`CompositorSettings`** - Runtime compositor configuration
  - Global opacity and blend settings
  - Per-layer enable/disable
  - Blend mode configuration

- **`LayerBlendState`** - Per-layer blend mode configuration
  - Additive, Multiply, Normal, Screen blend modes

- **`LayerToggleState`** - Per-layer visibility toggles
  - Runtime layer enable/disable

- **`GameCameraSettings`** - Camera behavior configuration
  - Shake intensity, duration, decay
  - Zoom levels and transition speeds
  - Smoothing and interpolation settings

- **`CompositorPresentation`** - Controls how compositor output is displayed
  - Render to texture vs. direct to screen

- **`RenderSurfaceSettings`** - Resolution and pixel ratio settings

### Components

- **`GameCamera`** - Main game camera with state tracking
  - Current shake offset and intensity
  - Zoom level and interpolation state
  - Smooth position and rotation tracking

- **`RenderLayer`** - Enum identifying layer assignment
  - `Background` - Background visuals (layer 0)
  - `GameWorld` - Primary game entities (layer 1)
  - `Metaballs` - Metaball rendering (layer 2)
  - `UI` - UI overlay (layer 3)

### Events

- **`CameraShakeCommand`** - Trigger camera shake effect
  - `intensity: f32` - Shake strength
  - `duration: f32` - Shake duration in seconds

- **`CameraZoomCommand`** - Trigger camera zoom
  - `target_zoom: f32` - Target zoom level
  - `transition_time: f32` - Time to reach target

### Types

- **`BlendMode`** - Layer blend modes
  - `Normal` - Standard alpha blending
  - `Additive` - Additive blending
  - `Multiply` - Multiplicative blending
  - `Screen` - Screen blending

- **`LayerConfig`** - Configuration for individual layers
  - Blend mode, opacity, enabled state

## Usage Example

### Basic Setup

```rust
use bevy::prelude::*;
use game_rendering::GameRenderingPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameRenderingPlugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    // GameCamera is automatically created by the plugin
    // You can customize it by querying and modifying
}
```

### Assigning Entities to Layers

```rust
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

fn spawn_layered_entities(mut commands: Commands) {
    // Background layer (0)
    commands.spawn((
        Sprite::from_color(Color::srgb(0.1, 0.1, 0.2), Vec2::splat(100.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        RenderLayers::layer(0),
    ));
    
    // GameWorld layer (1) - main gameplay entities
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.5, 0.0), Vec2::splat(50.0)),
        Transform::from_xyz(100.0, 100.0, 0.0),
        RenderLayers::layer(1),
    ));
    
    // Metaballs layer (2)
    // Typically managed by metaball_renderer
    
    // UI layer (3)
    commands.spawn((
        Text::new("Score: 0"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        RenderLayers::layer(3),
        Transform::from_xyz(-300.0, 250.0, 10.0),
    ));
}
```

### Camera Shake Effect

```rust
use bevy::prelude::*;
use game_rendering::CameraShakeCommand;

fn trigger_explosion_shake(
    mut shake_events: EventWriter<CameraShakeCommand>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        shake_events.send(CameraShakeCommand {
            intensity: 15.0,
            duration: 0.5,
        });
    }
}
```

### Camera Zoom

```rust
use bevy::prelude::*;
use game_rendering::CameraZoomCommand;

fn zoom_camera(
    mut zoom_events: EventWriter<CameraZoomCommand>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Equal) {
        // Zoom in
        zoom_events.send(CameraZoomCommand {
            target_zoom: 1.5,
            transition_time: 0.3,
        });
    }
    
    if input.just_pressed(KeyCode::Minus) {
        // Zoom out
        zoom_events.send(CameraZoomCommand {
            target_zoom: 0.75,
            transition_time: 0.3,
        });
    }
}
```

### Runtime Compositor Configuration

```rust
use bevy::prelude::*;
use game_rendering::{CompositorSettings, LayerBlendState, BlendMode};

fn configure_compositor(
    mut settings: ResMut<CompositorSettings>,
    mut blend_state: ResMut<LayerBlendState>,
    input: Res<ButtonInput<KeyCode>>,
) {
    // Toggle metaballs layer
    if input.just_pressed(KeyCode::KeyM) {
        settings.metaballs_enabled = !settings.metaballs_enabled;
    }
    
    // Change blend mode for metaballs
    if input.just_pressed(KeyCode::KeyB) {
        blend_state.metaballs = match blend_state.metaballs {
            BlendMode::Normal => BlendMode::Additive,
            BlendMode::Additive => BlendMode::Multiply,
            BlendMode::Multiply => BlendMode::Screen,
            BlendMode::Screen => BlendMode::Normal,
        };
    }
}
```

### Layer Toggling

```rust
use bevy::prelude::*;
use game_rendering::LayerToggleState;

fn toggle_layers(
    mut toggle_state: ResMut<LayerToggleState>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Digit1) {
        toggle_state.background = !toggle_state.background;
    }
    if input.just_pressed(KeyCode::Digit2) {
        toggle_state.game_world = !toggle_state.game_world;
    }
    if input.just_pressed(KeyCode::Digit3) {
        toggle_state.metaballs = !toggle_state.metaballs;
    }
    if input.just_pressed(KeyCode::Digit4) {
        toggle_state.ui = !toggle_state.ui;
    }
}
```

### Accessing Render Target Handles

```rust
use bevy::prelude::*;
use game_rendering::RenderTargetHandles;

fn access_render_targets(handles: Res<RenderTargetHandles>) {
    // Access layer textures for custom post-processing or effects
    if let Some(game_world_texture) = handles.game_world.as_ref() {
        // Use in custom render pass or material
    }
}
```

## Rendering Architecture

### Layer System

The rendering pipeline uses four distinct layers:

1. **Background (Layer 0)**: Background visuals, typically solid colors or gradients
2. **GameWorld (Layer 1)**: Primary gameplay entities (walls, targets, paddles, etc.)
3. **Metaballs (Layer 2)**: Metaball rendering output from `metaball_renderer`
4. **UI (Layer 3)**: User interface overlay elements

Each layer renders to its own texture, then the compositor combines them based on blend modes and opacity settings.

### Compositor

The compositor shader (`compositor.wgsl`) blends all layers together:

- Samples each layer texture
- Applies per-layer blend modes (Normal, Additive, Multiply, Screen)
- Respects layer opacity and enable/disable flags
- Outputs final composited frame

### Camera System

The camera system provides:

- **Smooth following**: Interpolated position/rotation tracking
- **Shake effects**: Procedural camera shake with decay
- **Zoom**: Smooth zoom transitions with configurable speed
- **Per-layer cameras**: Each render layer has its own camera derived from the main GameCamera

## Configuration

### Default Settings

```rust
GameCameraSettings {
    shake_decay: 5.0,
    max_shake_offset: 20.0,
    zoom_speed: 2.0,
    min_zoom: 0.5,
    max_zoom: 3.0,
    smooth_factor: 0.1,
}

CompositorSettings {
    background_enabled: true,
    game_world_enabled: true,
    metaballs_enabled: true,
    ui_enabled: true,
    global_opacity: 1.0,
}

LayerBlendState {
    background: BlendMode::Normal,
    game_world: BlendMode::Normal,
    metaballs: BlendMode::Additive,
    ui: BlendMode::Normal,
}
```

## Dependencies

- `bevy` - Core rendering and ECS
- `metaball_renderer` - Metaball layer integration
- `game_core` - Core game types
- `serde` - (Optional) Configuration serialization

## Performance Considerations

- Each layer incurs one render pass
- Render target resolution matches window size by default
- Consider lower resolution for expensive layers (metaballs)
- Compositor runs as a single fullscreen pass
- Camera interpolation runs every frame but is lightweight

## Testing

Test coverage should include:

- Camera shake decay and intensity
- Zoom interpolation
- Layer visibility toggling
- Compositor blend mode switching
- Window resize handling

Run tests with:

```bash
cargo test -p game_rendering
```

## Future Enhancements

- Dynamic resolution per layer for performance scaling
- Additional blend modes (Overlay, ColorDodge, etc.)
- Layer-specific post-processing effects
- Camera path following and cutscenes
- HDR rendering support
- Multi-camera support for split-screen
