# background_renderer

A configurable background rendering system for the metalrain game, providing multiple visual modes including solid colors, gradients, and animated effects.

## Description

This crate implements a Material2D-based background rendering system that supports runtime-configurable visual modes. It uses a custom WGSL shader (`background.wgsl`) to render fullscreen backgrounds with various effects including solid colors, linear gradients, radial gradients, and animated patterns.

## Purpose

**For Users:**

- Provides visually appealing, customizable backgrounds for game scenes
- Supports multiple rendering modes that can enhance game atmosphere
- Offers runtime configuration without requiring code changes

**For Downstream Developers:**

- Simple plugin integration with Bevy's ECS architecture
- Exposes a `BackgroundConfig` resource for runtime modification
- Clean separation of concerns - handles only background rendering
- Integrates seamlessly with the game's asset management system

## Key API Components

### Plugin

- **`BackgroundRendererPlugin`** - Main plugin that registers types, systems, and the Material2D pipeline

### Resources

- **`BackgroundConfig`** - Runtime-configurable settings for background appearance
  - `mode: BackgroundMode` - Visual rendering mode
  - `primary_color: LinearRgba` - Primary color for all modes
  - `secondary_color: LinearRgba` - Secondary color for gradients
  - `angle: f32` - Gradient direction (linear mode)
  - `animation_speed: f32` - Animation speed multiplier
  - `radial_center: Vec2` - Center point for radial gradients (normalized coords)
  - `radial_radius: f32` - Radius for radial gradients

### Types

- **`BackgroundMode`** - Enum defining available rendering modes:
  - `Solid` - Single solid color
  - `LinearGradient` - Linear gradient between two colors
  - `RadialGradient` - Radial gradient from center point
  - `Animated` - Time-based animated effect

### Material

- **`BackgroundMaterial`** - Custom Material2D implementation that drives the shader

## Usage Example

```rust
use bevy::prelude::*;
use background_renderer::{BackgroundRendererPlugin, BackgroundConfig, BackgroundMode};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BackgroundRendererPlugin)
        .add_systems(Startup, configure_background)
        .run();
}

fn configure_background(mut config: ResMut<BackgroundConfig>) {
    // Set a linear gradient from dark blue to black
    config.mode = BackgroundMode::LinearGradient;
    config.primary_color = LinearRgba::rgb(0.05, 0.05, 0.15);
    config.secondary_color = LinearRgba::rgb(0.0, 0.0, 0.0);
    config.angle = std::f32::consts::FRAC_PI_4; // 45 degrees
}

// Runtime mode switching example
fn cycle_background_mode(
    input: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<BackgroundConfig>,
) {
    if input.just_pressed(KeyCode::KeyB) {
        config.mode = config.mode.next();
    }
}
```

## Dependencies

- `bevy` - Core engine functionality
- `game_assets` - Asset loading and shader management

## Architecture Notes

The background renderer uses Bevy's Material2D system to render a fullscreen quad. The `setup_background` system creates the initial quad, while `update_background` syncs the `BackgroundConfig` resource changes to the material uniforms each frame. The actual rendering logic is implemented in `assets/shaders/background.wgsl`.
