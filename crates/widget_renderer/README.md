# widget_renderer

Visual rendering system for game world elements including walls, targets, hazards, paddles, and spawn points.

## Description

This crate provides sprite-based visual representations for all gameplay entities defined in `game_core`. It maintains a clean separation between game logic/physics and visual presentation by automatically spawning and updating visual components when game entities are added or modified. The renderer supports simple animations like hit flashes, destruction effects, and pulsing highlights.

All visuals are assigned to `RenderLayers::layer(1)` for integration with the multi-layer rendering pipeline in `game_rendering`.

## Purpose

**For Users:**

- Provides clear visual feedback for all game entities
- Animated visual effects for hits, destruction, and active states
- Color-coded visual language for different entity types
- Health visualization through opacity changes

**For Downstream Developers:**

- Automatic visual spawning - just add game components, visuals appear
- Loosely coupled from physics - no direct dependencies on physics state
- Simple sprite-based rendering suitable for 2D gameplay
- Easy to extend with additional visual effects
- Clean integration with layered rendering pipeline

## Key API Components

### Plugin

- **`WidgetRendererPlugin`** - Main plugin that registers all visual systems

### Systems

The plugin automatically registers systems to handle visual lifecycle:

#### Spawning Systems

- `spawn_wall_visuals` - Creates colored rectangle sprites for walls
- `spawn_target_visuals` - Creates circular sprites for targets
- `spawn_hazard_visuals` - Creates semi-transparent rectangles for hazards
- `spawn_paddle_visuals` - Creates cyan rectangles for paddles
- `spawn_spawnpoint_visuals` - Creates dual-ring visuals for spawn points

#### Animation Systems

- `update_target_animations` - Handles hit flash and destruction animations
- `update_hazard_pulse` - Sinusoidal alpha pulsing for hazards
- `update_active_spawnpoint_pulse` - Scale/alpha pulse for active spawn points
- `update_selected_highlight` - Visual tint for selected entities

#### Cleanup Systems

- `cleanup_destroyed_targets` - Removes entities after destruction animation completes

## Usage Example

### Basic Setup

```rust
use bevy::prelude::*;
use game_core::{GameCorePlugin, WallBundle, TargetBundle, PaddleBundle};
use widget_renderer::WidgetRendererPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GameCorePlugin)
        .add_plugins(WidgetRendererPlugin)
        .add_systems(Startup, spawn_game_entities)
        .run();
}

fn spawn_game_entities(mut commands: Commands) {
    // Spawn entities with game_core bundles
    // Visuals are automatically created by widget_renderer
    
    // Wall - will get a white rectangle visual
    commands.spawn(WallBundle::new(
        Vec2::new(-400.0, -300.0),
        Vec2::new(-400.0, 300.0),
        10.0,
        Color::WHITE,
    ));
    
    // Target - will get a colored circle visual with health-based opacity
    commands.spawn(TargetBundle::new(
        Vec2::new(100.0, 100.0),
        Target::new(3, 25.0, Color::srgb(0.2, 0.8, 0.3)),
    ));
    
    // Paddle - will get a cyan rectangle visual
    commands.spawn(PaddleBundle::new(
        Vec2::new(0.0, -250.0),
        Paddle::default(),
    ));
}
```

### Manual Entity Spawning

```rust
use bevy::prelude::*;
use game_core::*;

fn spawn_custom_target(mut commands: Commands) {
    // Spawn just the component - visual system will detect and add sprite
    commands.spawn((
        Target::new(5, 30.0, Color::srgb(1.0, 0.5, 0.0)),
        Transform::from_xyz(200.0, 150.0, 0.0),
        GlobalTransform::IDENTITY,
    ));
}
```

### Triggering Animations

```rust
use game_core::{Target, TargetState};

// Hit animation - target flashes and scales up briefly
fn trigger_target_hit(mut targets: Query<&mut Target>) {
    for mut target in &mut targets {
        if target.health > 0 {
            target.health -= 1;
            target.state = TargetState::Hit(0.0);
        }
    }
}

// Destruction animation - target scales up and fades out
fn trigger_target_destruction(mut targets: Query<&mut Target>) {
    for mut target in &mut targets {
        if target.health == 0 {
            target.state = TargetState::Destroying(0.0);
        }
    }
}
```

## Visual Specifications

### Walls

- **Shape**: Rectangle oriented along start-end vector
- **Color**: From `Wall.color` component
- **Size**: Length × thickness
- **Layer**: RenderLayers::layer(1)
- **Z-order**: 0.0

### Targets

- **Shape**: Circle
- **Color**: From `Target.color` component
- **Size**: diameter = 2 × radius
- **Opacity**: Based on health ratio (50-100% alpha)
- **Layer**: RenderLayers::layer(1)
- **Z-order**: 1.0
- **Animations**:
  - Hit: 1.2× scale pulse, white flash (1.0s duration)
  - Destroying: 1.4× scale expansion, fade to transparent (0.5s duration)

### Hazards

- **Shape**: Rectangle
- **Color**: Red with low alpha (0.8, 0.1, 0.1, 0.35)
- **Size**: From `Hazard.bounds`
- **Effect**: Sinusoidal alpha pulse (0.1-0.5 range, 2Hz)
- **Layer**: RenderLayers::layer(1)
- **Z-order**: -0.1 (behind other entities)

### Paddles

- **Shape**: Rectangle
- **Color**: Cyan (0.1, 0.85, 0.95)
- **Size**: 2 × half_extents
- **Layer**: RenderLayers::layer(1)
- **Z-order**: 0.2

### Spawn Points

- **Shape**: Two concentric circles (inner + ring)
- **Colors**: Yellow (0.9, 0.9, 0.25) and light yellow (1.0, 1.0, 0.5)
- **Sizes**: Inner = 1.6 × radius, Ring = 2.4 × radius
- **Effect**: Scale pulse when active (1.0-1.15× range, 3.5Hz)
- **Opacity**: 90% when active, 40% when inactive
- **Layer**: RenderLayers::layer(1)
- **Z-order**: 0.05-0.06

### Selected Entities

- **Effect**: Red tint added to base color (+0.3 red channel)
- **Trigger**: `Selected` component added to entity

## Integration with Rendering Pipeline

All widget visuals use `RenderLayers::layer(1)` to integrate with the `game_rendering` compositor system. This allows widgets to be:

- Rendered to a dedicated layer texture
- Composited with other layers (background, metaballs, UI)
- Toggled on/off at the layer level
- Processed with layer-specific post-effects

## Dependencies

- `bevy` - Core rendering and ECS
- `bevy_rapier2d` - Physics types (minimal usage)
- `game_core` - Core component definitions
- `game_physics` - Physics configuration types

## Performance Notes

- Visual spawning is automatic but runs only on `Added<T>` queries (one-time cost)
- Animation systems run every frame but only query entities with relevant components
- No GPU instancing currently - consider for >100 similar entities
- Sprite batching handled automatically by Bevy's 2D renderer

## Future Extensions

- Glow/outline effects using multi-sprite composition
- Particle systems for target destruction
- Trail effects for fast-moving entities
- Additional hazard visual types
- GPU instancing for high entity counts
- Customizable color schemes and themes
