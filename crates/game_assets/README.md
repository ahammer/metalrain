# game_assets

Centralized asset management for the metalrain project, providing standardized loading and access to fonts, shaders, and other game resources.

## Description

This crate provides a unified asset management system that abstracts asset loading paths and handles. It ensures consistent asset access across all crates and demo applications, eliminating hardcoded paths and providing a foundation for future features like embedded assets and hot-reloading.

The crate handles workspace structure complexity by providing configuration helpers that automatically resolve asset paths based on execution context (demo crate, game crate, or workspace root).

## Purpose

**For Users:**

- Ensures consistent asset availability across the game
- Provides foundation for features like asset hot-reloading
- Enables potential embedded asset builds for web deployment

**For Downstream Developers:**

- Eliminates hardcoded asset paths in dependent crates
- Provides typed, validated asset handles through resources
- Simplifies demo and test setup with standardized configuration
- Single source of truth for all asset loading
- Supports multiple execution contexts (demos, tests, workspace root)

## Key API Components

### Plugin

- **`GameAssetsPlugin`** - Main plugin that loads and manages all game assets
  - `use_embedded: bool` - Future feature flag for embedded asset mode

### Resources

- **`GameAssets`** - Top-level resource containing all asset categories
  - `fonts: FontAssets` - Font handles
  - `shaders: ShaderAssets` - Shader handles
  
- **`FontAssets`** - Font resource handles
  - `ui_regular: Handle<Font>` - Regular UI font (FiraSans-Regular)
  - `ui_bold: Handle<Font>` - Bold UI font (FiraSans-Bold)

- **`ShaderAssets`** - Shader resource handles
  - `compositor: Handle<Shader>` - Compositor shader
  - `compute_metaballs: Handle<Shader>` - Metaball compute shader
  - `compute_3d_normals: Handle<Shader>` - 3D normals compute shader
  - `present_fullscreen: Handle<Shader>` - Fullscreen presentation shader
  - `background: Handle<Shader>` - Background rendering shader

### Configuration Helpers

- **`AssetRootMode`** - Enum defining execution contexts:
  - `DemoCrate` - Running from `demos/<demo_name>` directory
  - `GameCrate` - Running from `crates/<crate_name>` directory
  - `WorkspaceRoot` - Running from workspace root

- **`configure_standard_assets(app, mode)`** - Configures AssetPlugin and GameAssetsPlugin
- **`configure_demo(app)`** - Convenience wrapper for demo crates
- **`configure_game_crate(app)`** - Convenience wrapper for game crates
- **`configure_workspace_root(app)`** - Convenience wrapper for workspace-level execution

## Usage Example

### In a Demo Crate

```rust
use bevy::prelude::*;
use game_assets::{configure_demo, GameAssets};

fn main() {
    let mut app = App::new();
    configure_demo(&mut app); // Sets up DefaultPlugins + GameAssetsPlugin
    app.add_systems(Startup, use_assets);
    app.run();
}

fn use_assets(assets: Res<GameAssets>) {
    // Access loaded assets through typed handles
    let regular_font = assets.fonts.ui_regular.clone();
    let compositor_shader = assets.shaders.compositor.clone();
    // Use these handles in your systems...
}
```

### In a Game Crate

```rust
use bevy::prelude::*;
use game_assets::{configure_game_crate, GameAssets};

fn main() {
    let mut app = App::new();
    configure_game_crate(&mut app);
    app.add_systems(Startup, setup_rendering);
    app.run();
}

fn setup_rendering(assets: Res<GameAssets>) {
    let metaball_shader = assets.shaders.compute_metaballs.clone();
    // Use in render pipeline setup...
}
```

### Manual Configuration

```rust
use bevy::prelude::*;
use game_assets::{AssetRootMode, configure_standard_assets, GameAssets};

fn main() {
    let mut app = App::new();
    
    // Custom configuration
    configure_standard_assets(&mut app, AssetRootMode::WorkspaceRoot);
    
    app.add_systems(Startup, access_assets);
    app.run();
}

fn access_assets(assets: Res<GameAssets>) {
    info!("Assets loaded: {:?}", assets);
}
```

## Asset Structure

Assets are expected to be in the `assets/` directory relative to the configured root:

```
assets/
├── fonts/
│   ├── FiraSans-Regular.ttf
│   └── FiraSans-Bold.ttf
└── shaders/
    ├── background.wgsl
    ├── compositor.wgsl
    ├── compute_3d_normals.wgsl
    ├── compute_metaballs.wgsl
    └── present_fullscreen.wgsl
```

## Features

- **`embedded`** - (Future) Enables embedding assets directly in the binary for web builds

## Dependencies

- `bevy` - Core engine and asset system
- `serde` - (Optional) For future configuration serialization

## Architecture Notes

The `GameAssets` resource implements `ExtractResource` to allow automatic extraction into the render world, enabling render graph pipeline creation to access shader handles directly. All asset loading occurs during the `Startup` stage through the `load_assets` system.
