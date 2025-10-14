# UI Demo POC: Bevy-HUI Interface Exploration

> **🎯 Sprint Direction Change**: This document has been updated to focus on a standalone POC demo using Bevy-HUI directly, rather than integrating UI into core crates. The goal is to explore HUI's capabilities before committing to an architecture decision.

## Sprint Goal

Create a **proof-of-concept demo** using **Bevy-HUI** directly to explore UI patterns and demonstrate interactive controls with a mock compositor interface. This is a standalone demo that does NOT integrate with core crates—it's purely exploratory.

**Primary Deliverable**: A new `demos/ui_demo` that showcases Bevy-HUI capabilities with buttons, controls, and a mock compositor interface inspired by `compositor_test`.

## Architecture Overview

```
┌─────────────────────────┐
│   demos/ui_demo         │ ← Standalone POC demo
│   (Bevy-HUI direct)     │
│                         │
│  • No scaffold          │
│  • No core crate deps   │
│  • Pure exploration     │
└─────────────────────────┘
```

**Key Principle**: This demo is intentionally isolated. It does NOT integrate with scaffold, game_rendering, or any other crates. It's a sandbox for exploring Bevy-HUI's capabilities.

## Philosophy

This POC explores Bevy-HUI as a potential UI solution:

- **Template-based**: Uses Bevy-HUI's HTML-style templates (not immediate-mode)
- **Declarative**: UI layouts defined in pseudo-HTML syntax
- **Exploratory**: Test all supported widgets (buttons, checkboxes, sliders, etc.)
- **Mock Interface**: Simulate compositor controls without real functionality
- **Standalone**: No dependencies on existing crate architecture
- **Learning Tool**: Understand HUI's capabilities and limitations before deciding on integration strategy

## Problem Statement

Before integrating UI into the existing architecture, we need to:

1. **Understand Bevy-HUI**: What widgets are supported? How do they work?
2. **Test Interaction**: Can we handle button clicks, slider changes, checkboxes?
3. **Evaluate Performance**: What's the overhead? Is it WASM-compatible?
4. **Design Patterns**: What layout patterns work well for our use cases?

This POC answers these questions with a **mock compositor interface** that demonstrates:

- **Layer visibility toggles** (mirroring compositor_test's 1-5 keys)
- **Effect controls** (burst force, wall pulse parameters)
- **Rendering mode selection** (metaball visualization modes)
- **Real-time parameter adjustment** (sliders for timing, strength, distance)
- **Status display** (FPS, entity count, active effects)

## Deliverables

### 1. UI Demo Crate (`demos/ui_demo`)

- [ ] Create `demos/ui_demo` with Bevy-HUI dependency
- [ ] Implement mock compositor state resource
- [ ] Create HUI templates for all UI panels:
  - [ ] Control panel (left sidebar with layer toggles, effect controls)
  - [ ] Status panel (top bar with FPS, entity count, active effects)
  - [ ] Parameter panel (right sidebar with sliders for timing/strength)
  - [ ] Info panel (center overlay with help text)
- [ ] Implement visual simulation (colored balls bouncing in viewport)
- [ ] Wire up all interactive controls (buttons, checkboxes, sliders)
- [ ] Add keyboard shortcuts for quick testing (1-5 for layers, Space for effects)

### 2. Widget Demonstrations

- [ ] **Buttons**: Trigger burst force, reset simulation
- [ ] **Checkboxes**: Layer visibility toggles (Background, GameWorld, Metaballs, Effects, UI)
- [ ] **Radio Buttons**: Metaball visualization mode (Normal, Distance Field, Normals, Raw Compute)
- [ ] **Sliders**: Effect parameters (burst interval, strength, wall pulse timing)
- [ ] **Text Display**: Real-time status updates (FPS counter, ball count, active effects)
- [ ] **Dropdown** (if supported): Preset configurations

### 3. Mock Compositor Interface

Inspired by `compositor_test`, simulate these controls:

- [ ] **Layer Visibility**: 5 checkboxes matching compositor layers
- [ ] **Burst Force**: Button + parameter sliders (interval, duration, radius, strength)
- [ ] **Wall Pulse**: Button + parameter sliders (interval, duration, distance, strength)
- [ ] **Visualization Mode**: Radio buttons for different rendering modes
- [ ] **Simulation Controls**: Play/Pause, Reset, Spawn More Balls
- [ ] **Stats Display**: Frame time, ball count, physics tick rate

### 4. Documentation & Findings

- [ ] Create comprehensive README explaining POC purpose
- [ ] Document which HUI features work well
- [ ] Note any limitations or issues discovered
- [ ] Include screenshots of UI layout
- [ ] Provide recommendations for future integration strategy

## Technical Specifications

### 1. UI Demo Structure

```toml
# demos/ui_demo/Cargo.toml
[package]
name = "ui_demo"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0"
publish = false
description = "POC demo exploring Bevy-HUI with mock compositor interface"

[dependencies]
bevy = { workspace = true }
bevy_hui = "0.4"  # Compatible with Bevy 0.16
rand = "0.8"
```

```rust
// demos/ui_demo/src/lib.rs
use bevy::prelude::*;
use bevy_hui::prelude::*;

pub const DEMO_NAME: &str = "UI Demo (Bevy-HUI POC)";

/// Mock compositor state simulating compositor_test functionality
#[derive(Resource, Debug)]
pub struct MockCompositorState {
    // Layer visibility
    pub layer_background: bool,
    pub layer_game_world: bool,
    pub layer_metaballs: bool,
    pub layer_effects: bool,
    pub layer_ui: bool,
    
    // Effect parameters
    pub burst_interval: f32,
    pub burst_duration: f32,
    pub burst_radius: f32,
    pub burst_strength: f32,
    
    pub wall_pulse_interval: f32,
    pub wall_pulse_duration: f32,
    pub wall_pulse_distance: f32,
    pub wall_pulse_strength: f32,
    
    // Visualization mode
    pub viz_mode: VizMode,
    
    // Simulation state
    pub paused: bool,
    pub ball_count: usize,
    pub fps: f32,
    pub active_burst: bool,
    pub active_wall_pulse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VizMode {
    Normal,
    DistanceField,
    Normals,
    RawCompute,
}

impl Default for MockCompositorState {
    fn default() -> Self {
        Self {
            layer_background: true,
            layer_game_world: true,
            layer_metaballs: true,
            layer_effects: true,
            layer_ui: true,
            
            burst_interval: 3.0,
            burst_duration: 0.6,
            burst_radius: 110.0,
            burst_strength: 1400.0,
            
            wall_pulse_interval: 10.0,
            wall_pulse_duration: 0.8,
            wall_pulse_distance: 120.0,
            wall_pulse_strength: 2200.0,
            
            viz_mode: VizMode::Normal,
            paused: false,
            ball_count: 400,
            fps: 60.0,
            active_burst: false,
            active_wall_pulse: false,
        }
    }
}

pub fn run_ui_demo() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            HuiPlugin,
        ))
        .init_resource::<MockCompositorState>()
        .add_systems(Startup, (setup_camera, setup_ui, spawn_visual_simulation))
        .add_systems(Update, (
            handle_keyboard_shortcuts,
            handle_ui_interactions,
            update_ui_displays,
            update_visual_simulation,
            update_fps_counter,
        ))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Spawn all HUI templates
    commands.spawn((
        HtmlNode(asset_server.load("ui/control_panel.html")),
        Name::new("ControlPanel"),
    ));
    
    commands.spawn((
        HtmlNode(asset_server.load("ui/status_bar.html")),
        Name::new("StatusBar"),
    ));
    
    commands.spawn((
        HtmlNode(asset_server.load("ui/parameter_panel.html")),
        Name::new("ParameterPanel"),
    ));
}

fn spawn_visual_simulation(mut commands: Commands) {
    // Spawn colored circles to represent balls
    // This is just visual - no physics, just bouncing sprites
}

fn handle_keyboard_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<MockCompositorState>,
) {
    // 1-5 for layer toggles
    if keys.just_pressed(KeyCode::Digit1) {
        state.layer_background = !state.layer_background;
    }
    // ... etc
}

fn handle_ui_interactions(
    // Query for HUI interaction events
    // Update MockCompositorState based on button clicks, slider changes, etc.
) {
    // This will be fleshed out based on HUI's event system
}

fn update_ui_displays(
    state: Res<MockCompositorState>,
    // Query for HUI text/display elements
) {
    // Update FPS counter, ball count, active effect indicators
}

fn update_visual_simulation(
    state: Res<MockCompositorState>,
    // Query for visual simulation entities
) {
    // Animate the colored circles based on simulated effects
}

fn update_fps_counter(
    time: Res<Time>,
    mut state: ResMut<MockCompositorState>,
) {
    state.fps = 1.0 / time.delta_secs();
}
```

### 2. Scaffold Integration (Feature-Gated)

```toml
# crates/scaffold/Cargo.toml
[dependencies]
bevy = { workspace = true }
bevy_rapier2d = { workspace = true }
basic_ui = { path = "../basic_ui", optional = true }
# ... other dependencies

[features]
debug_ui = ["basic_ui"]
```

```rust
// crates/scaffold/src/ui/plugin.rs
use bevy::prelude::*;
use basic_ui::BasicUiPlugin;

/// Optional plugin providing HUI-based UI infrastructure for demos.
/// Enable with the "debug_ui" feature.
#[cfg(feature = "debug_ui")]
pub struct UiScaffoldPlugin;

#[cfg(feature = "debug_ui")]
impl Plugin for UiScaffoldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BasicUiPlugin)
            .init_resource::<ScaffoldUiContext>()
            .init_resource::<ScaffoldKeyBindings>()
            .init_resource::<HelpPanelVisible>()
            .add_systems(Update, (toggle_ui_panels, toggle_help_panel));
    }
}

/// Context for demo-specific UI panels
#[derive(Resource, Debug, Default)]
pub struct ScaffoldUiContext {
    /// Whether interactive panels are visible (toggled with F2)
    pub panels_visible: bool,
    
    /// Entity for the help panel UI (F3)
    pub help_panel_entity: Option<Entity>,
}

/// Toggle interactive panels with F2 (F1 is scaffold HUD)
fn toggle_ui_panels(
    keys: Res<ButtonInput<KeyCode>>,
    mut ctx: ResMut<ScaffoldUiContext>,
) {
    if keys.just_pressed(KeyCode::F2) {
        ctx.panels_visible = !ctx.panels_visible;
        info!("Interactive UI panels: {}", ctx.panels_visible);
    }
}

/// Toggle help panel with F3
fn toggle_help_panel(
    keys: Res<ButtonInput<KeyCode>>,
    mut help_visible: ResMut<HelpPanelVisible>,
    mut ctx: ResMut<ScaffoldUiContext>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if keys.just_pressed(KeyCode::F3) {
        help_visible.0 = !help_visible.0;
        
        if help_visible.0 {
            // Spawn help panel from basic_ui template
            let entity = commands.spawn(HtmlNode(asset_server.load("ui/help_panel.html"))).id();
            ctx.help_panel_entity = Some(entity);
        } else if let Some(entity) = ctx.help_panel_entity.take() {
            commands.entity(entity).despawn();
        }
    }
}
```

### 2. HUI Template Examples

```html
<!-- demos/ui_demo/assets/ui/control_panel.html -->
<template>
    <node 
        position_type="absolute"
        left="10px"
        top="60px"
        width="280px"
        background="#1a1a1add"
        border="2px solid #444"
        padding="15px"
        display="flex"
        flex_direction="column"
        gap="12px"
    >
        <text font_size="18" color="#fff" font_weight="bold">
            Layer Visibility
        </text>
        
        <checkbox id="layer_background" checked="true">
            Background
        </checkbox>
        
        <checkbox id="layer_game_world" checked="true">
            Game World
        </checkbox>
        
        <checkbox id="layer_metaballs" checked="true">
            Metaballs
        </checkbox>
        
        <checkbox id="layer_effects" checked="true">
            Effects
        </checkbox>
        
        <checkbox id="layer_ui" checked="true">
            UI
        </checkbox>
        
        <divider height="2px" background="#444" margin="8px 0" />
        
        <text font_size="18" color="#fff" font_weight="bold">
            Effect Controls
        </text>
        
        <button id="trigger_burst">
            Trigger Burst Force
        </button>
        
        <button id="trigger_wall_pulse">
            Trigger Wall Pulse
        </button>
        
        <button id="reset_simulation">
            Reset Simulation
        </button>
    </node>
</template>
```

```html
<!-- demos/ui_demo/assets/ui/status_bar.html -->
<template>
    <node 
        position_type="absolute"
        top="10px"
        left="10px"
        right="10px"
        height="40px"
        background="#1a1a1add"
        border="2px solid #444"
        padding="8px 15px"
        display="flex"
        align_items="center"
        justify_content="space-between"
    >
        <text font_size="16" color="#fff">
            UI Demo - Bevy-HUI POC
        </text>
        
        <text id="fps_counter" font_size="14" color="#0f0">
            FPS: 60
        </text>
        
        <text id="ball_counter" font_size="14" color="#aaa">
            Balls: 400
        </text>
        
        <text id="active_effects" font_size="14" color="#f80">
            <!-- Dynamically updated with active effects -->
        </text>
    </node>
</template>
```

```html
<!-- demos/ui_demo/assets/ui/parameter_panel.html -->
<template>
    <node 
        position_type="absolute"
        right="10px"
        top="60px"
        width="300px"
        background="#1a1a1add"
        border="2px solid #444"
        padding="15px"
        display="flex"
        flex_direction="column"
        gap="15px"
    >
        <text font_size="18" color="#fff" font_weight="bold">
            Burst Force Parameters
        </text>
        
        <label>
            Interval: <span id="burst_interval_value">3.0s</span>
        </label>
        <slider id="burst_interval" min="1.0" max="10.0" value="3.0" step="0.1" />
        
        <label>
            Duration: <span id="burst_duration_value">0.6s</span>
        </label>
        <slider id="burst_duration" min="0.1" max="2.0" value="0.6" step="0.1" />
        
        <label>
            Radius: <span id="burst_radius_value">110</span>
        </label>
        <slider id="burst_radius" min="50" max="300" value="110" step="10" />
        
        <label>
            Strength: <span id="burst_strength_value">1400</span>
        </label>
        <slider id="burst_strength" min="500" max="5000" value="1400" step="100" />
        
        <divider height="2px" background="#444" margin="8px 0" />
        
        <text font_size="18" color="#fff" font_weight="bold">
            Visualization Mode
        </text>
        
        <radio-group id="viz_mode">
            <radio value="normal" checked="true">Normal</radio>
            <radio value="distance">Distance Field</radio>
            <radio value="normals">Normals</radio>
            <radio value="raw">Raw Compute</radio>
        </radio-group>
    </node>
</template>
```

### 4. UI Test Demo Implementation

```rust
// demos/ui_test/Cargo.toml
[package]
name = "ui_test"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0"
publish = false

[dependencies]
bevy = { workspace = true }
bevy_hui = "0.4"
scaffold = { path = "../../crates/scaffold", features = ["debug_ui"] }
game_assets = { path = "../../crates/game_assets" }
```

```rust
// demos/ui_test/src/lib.rs
use bevy::prelude::*;
use bevy_hui::{HuiPlugin, HtmlNode};
use scaffold::ui::UiScaffoldPlugin;

pub const DEMO_NAME: &str = "UI Layout Patterns";

#[derive(Resource, Default)]
pub struct UiTestState {
    pub panels_visible: bool,
    // UI entity handles
    pub header: Option<Entity>,
    pub footer: Option<Entity>,
    pub left_sidebar: Option<Entity>,
    pub right_sidebar: Option<Entity>,
    pub center_panel: Option<Entity>,
}

pub fn run_ui_test() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            HuiPlugin,
            UiScaffoldPlugin,
        ))
        .init_resource::<UiTestState>()
        .add_systems(Startup, (setup_demo_camera, setup_demo_ui))
        .add_systems(Update, (toggle_panels, update_panel_visibility))
        .run();
}

fn setup_demo_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

/// Spawn HUI templates for all UI panels
fn setup_demo_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<UiTestState>,
) {
    // Load all templates from basic_ui assets
    state.header = Some(
        commands.spawn(HtmlNode(asset_server.load("ui/header.html"))).id()
    );
    
    state.footer = Some(
        commands.spawn(HtmlNode(asset_server.load("ui/footer.html"))).id()
    );
    
    state.left_sidebar = Some(
        commands.spawn(HtmlNode(asset_server.load("ui/left_sidebar.html"))).id()
    );
    
    state.right_sidebar = Some(
        commands.spawn(HtmlNode(asset_server.load("ui/right_sidebar.html"))).id()
    );
    
    state.center_panel = Some(
        commands.spawn(HtmlNode(asset_server.load("ui/center_panel.html"))).id()
    );
    
    info!("UI Test demo initialized with HUI templates");
}

/// Toggle panels with F2
fn toggle_panels(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<UiTestState>,
) {
    if keys.just_pressed(KeyCode::F2) {
        state.panels_visible = !state.panels_visible;
        info!("UI panels: {}", state.panels_visible);
    }
}

/// Show/hide UI panels based on state
fn update_panel_visibility(
    state: Res<UiTestState>,
    mut query: Query<&mut Visibility>,
) {
    let visibility = if state.panels_visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    
    // Toggle sidebars and center panel only (header/footer stay visible)
    for entity in [
        state.left_sidebar,
        state.right_sidebar,
        state.center_panel,
    ].iter().filter_map(|e| *e) {
        if let Ok(mut vis) = query.get_mut(entity) {
            *vis = visibility;
        }
    }
}
```

```rust
// demos/ui_test/src/main.rs (standalone entry point)
fn main() {
    ui_test::run_ui_test();
}
```

### Demo Integration Examples (Reference Only - Not Part of This Sprint)

These examples show how existing demos *could* integrate the UI scaffold feature once it's complete. They are provided as reference patterns but are **NOT** required deliverables for this sprint.

#### Physics Playground Example Pattern

```rust
// Example only - shows sidebar pattern for physics controls
// Would add spawn mode selector and physics parameter sliders
```

#### Metaballs Test Example Pattern

```rust
// Example only - shows radio button pattern for visualization modes
// Would add visualization mode selector (distance field, normals, etc.)
```

#### Compositor Test Example Pattern

```rust
// Example only - shows checkbox group pattern for layer visibility
// Would add layer visibility toggles mirroring keyboard shortcuts
```

### 5. Keyboard Binding Registry

```rust
// crates/scaffold/src/ui/bindings.rs

/// Central registry of scaffold keyboard bindings to prevent conflicts
#[derive(Resource, Debug, Clone)]
pub struct ScaffoldKeyBindings {
    pub bindings: Vec<KeyBinding>,
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub description: String,
    pub category: BindingCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingCategory {
    Scaffold,
    Camera,
    Layers,
    Debug,
    Demo,
}

impl Default for ScaffoldKeyBindings {
    fn default() -> Self {
        Self {
            bindings: vec![
                KeyBinding {
                    key: KeyCode::F1,
                    description: "Toggle Scaffold HUD".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::F2,
                    description: "Toggle Interactive UI Panels".to_string(),
                    category: BindingCategory::Debug,
                },
                KeyBinding {
                    key: KeyCode::F3,
                    description: "Show Help/Key Bindings".to_string(),
                    category: BindingCategory::Debug,
                },
                KeyBinding {
                    key: KeyCode::Digit1,
                    description: "Toggle Background Layer".to_string(),
                    category: BindingCategory::Layers,
                },
                KeyBinding {
                    key: KeyCode::Digit2,
                    description: "Toggle Game World Layer".to_string(),
                    category: BindingCategory::Layers,
                },
                KeyBinding {
                    key: KeyCode::Digit3,
                    description: "Toggle Metaballs Layer".to_string(),
                    category: BindingCategory::Layers,
                },
                KeyBinding {
                    key: KeyCode::Digit4,
                    description: "Toggle Effects Layer".to_string(),
                    category: BindingCategory::Layers,
                },
                KeyBinding {
                    key: KeyCode::Digit5,
                    description: "Toggle UI Layer".to_string(),
                    category: BindingCategory::Layers,
                },
                KeyBinding {
                    key: KeyCode::BracketLeft,
                    description: "Decrease Exposure".to_string(),
                    category: BindingCategory::Camera,
                },
                KeyBinding {
                    key: KeyCode::BracketRight,
                    description: "Increase Exposure".to_string(),
                    category: BindingCategory::Camera,
                },
                KeyBinding {
                    key: KeyCode::Minus,
                    description: "Zoom Out".to_string(),
                    category: BindingCategory::Camera,
                },
                KeyBinding {
                    key: KeyCode::Equal,
                    description: "Zoom In".to_string(),
                    category: BindingCategory::Camera,
                },
                KeyBinding {
                    key: KeyCode::Space,
                    description: "Camera Shake".to_string(),
                    category: BindingCategory::Camera,
                },
                KeyBinding {
                    key: KeyCode::KeyR,
                    description: "Reset Camera/Physics".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::KeyP,
                    description: "Pause Physics".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::ArrowUp,
                    description: "Increase Gravity Y".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::ArrowDown,
                    description: "Decrease Gravity Y".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::ArrowLeft,
                    description: "Decrease Gravity X".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::ArrowRight,
                    description: "Increase Gravity X".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::KeyB,
                    description: "Cycle Background Mode".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::KeyM,
                    description: "Cycle Metaball Mode".to_string(),
                    category: BindingCategory::Scaffold,
                },
                KeyBinding {
                    key: KeyCode::Escape,
                    description: "Exit Application".to_string(),
                    category: BindingCategory::Scaffold,
                },
            ],
        }
    }
}

impl ScaffoldKeyBindings {
    /// Check if a key is already bound by scaffold
    pub fn is_reserved(&self, key: KeyCode) -> bool {
        self.bindings.iter().any(|b| b.key == key && b.category != BindingCategory::Demo)
    }
    
    /// Add a demo-specific binding (with conflict detection)
    pub fn add_demo_binding(&mut self, key: KeyCode, description: String) -> Result<(), String> {
        if self.is_reserved(key) {
            let existing = self.bindings.iter()
                .find(|b| b.key == key)
                .unwrap();
            return Err(format!(
                "Key {:?} already bound to: {}",
                key, existing.description
            ));
        }
        
        self.bindings.push(KeyBinding {
            key,
            description,
            category: BindingCategory::Demo,
        });
        
        Ok(())
    }
}

```

### 6. Help Panel Template

```html
<!-- crates/basic_ui/assets/ui/templates/help_panel.html -->
<template>
    <node 
        position_type="absolute"
        left="50%"
        top="50%"
        transform="translate(-50%, -50%)"
        width="500px"
        height="600px"
        background="#1a1a1a"
        border="2px"
        border_color="#444444"
        padding="20px"
        display="flex"
        flex_direction="column"
    >
        <text font_size="24" font_color="#ffffff">
            Keyboard Bindings (F3 to close)
        </text>
        
        <node height="2px" background="#444444" margin_top="10px" margin_bottom="10px"></node>
        
        <!-- Scaffold bindings section -->
        <text font_size="18" font_color="#aaaaaa">Scaffold</text>
        <text font_size="14" font_color="#cccccc">  F1: Toggle HUD</text>
        <text font_size="14" font_color="#cccccc">  F2: Toggle UI Panels</text>
        <text font_size="14" font_color="#cccccc">  F3: Show Help</text>
        
        <node height="1px" background="#333333" margin_top="8px" margin_bottom="8px"></node>
        
        <!-- Camera bindings section -->
        <text font_size="18" font_color="#aaaaaa">Camera</text>
        <text font_size="14" font_color="#cccccc">  -/=: Zoom</text>
        <text font_size="14" font_color="#cccccc">  Space: Shake</text>
        <text font_size="14" font_color="#cccccc">  R: Reset</text>
        
        <!-- Additional sections... -->
    </node>
</template>
```

## Implementation Checklist

### Phase 1: Project Setup

- [ ] Create `demos/ui_demo/` directory structure
- [ ] Add `demos/ui_demo/Cargo.toml` with bevy_hui dependency
- [ ] Create `demos/ui_demo/src/lib.rs` with `run_ui_demo()` and `DEMO_NAME`
- [ ] Create `demos/ui_demo/src/main.rs` for standalone execution
- [ ] Define `MockCompositorState` resource with all parameters
- [ ] Add `demos/ui_demo/assets/ui/` directory for templates

### Phase 2: HUI Template Creation

- [ ] Create `control_panel.html` (left sidebar with checkboxes + buttons)
- [ ] Create `status_bar.html` (top bar with FPS/stats display)
- [ ] Create `parameter_panel.html` (right sidebar with sliders + radio buttons)
- [ ] Create `info_panel.html` (optional center overlay with help text)
- [ ] Test template loading and rendering

### Phase 3: Visual Simulation

- [ ] Implement `spawn_visual_simulation` system (spawn colored circles)
- [ ] Implement `update_visual_simulation` system (simple bouncing physics)
- [ ] Add visual effects for burst force (radial push from center)
- [ ] Add visual effects for wall pulse (inward push from edges)
- [ ] Layer visibility handling (show/hide based on checkboxes)

### Phase 4: UI Interaction Wiring

- [ ] Wire up layer visibility checkboxes (1-5 keys + UI)
- [ ] Wire up burst force button and parameters
- [ ] Wire up wall pulse button and parameters
- [ ] Wire up visualization mode radio buttons
- [ ] Wire up simulation controls (pause, reset, spawn more)
- [ ] Update status bar displays (FPS counter, ball count, active effects)

### Phase 5: Testing & Documentation

- [ ] Test standalone: `cargo run -p ui_demo`
- [ ] Test all interactive controls (buttons, checkboxes, sliders, radios)
- [ ] Verify keyboard shortcuts work (1-5, Space, P, R)
- [ ] Test WASM build with `pwsh scripts/wasm-dev.ps1`
- [ ] Create comprehensive README with findings
- [ ] Document which HUI features worked well
- [ ] Note any limitations or issues
- [ ] Include screenshots of UI layout
- [ ] Provide recommendations for potential integration

## Keyboard Shortcuts (UI Demo Specific)

Since this is a standalone POC, we can define custom shortcuts without worrying about scaffold conflicts:

**Layer Toggles**:

- `1`: Toggle Background layer visibility
- `2`: Toggle GameWorld layer visibility
- `3`: Toggle Metaballs layer visibility
- `4`: Toggle Effects layer visibility
- `5`: Toggle UI layer visibility

**Effect Triggers**:

- `Space`: Trigger burst force manually
- `W`: Trigger wall pulse manually

**Simulation Controls**:

- `P`: Pause/Resume simulation
- `R`: Reset simulation (respawn balls)
- `+`/`=`: Spawn more balls
- `-`: Remove balls

**Visualization**:

- `V`: Cycle visualization mode
- `F1`: Toggle UI panels visibility
- `Esc`: Exit demo

## Exploration Goals

### Questions to Answer

This POC should help us understand:

✅ **Widget Support**: Which HUI widgets are available and functional? (buttons, checkboxes, sliders, radio buttons, dropdowns)  
✅ **Event Handling**: How do we wire up interactions? Is it ergonomic? Are there callbacks or polling?  
✅ **Template Syntax**: Is the HTML-like syntax expressive enough? Can we nest templates?  
✅ **Performance**: What's the frame time impact? Is it acceptable for game UIs?  
✅ **Hot Reload**: Can we edit templates and see changes without recompiling?  
✅ **WASM Compatibility**: Does it work in browser? Any special considerations?  
✅ **Layout Flexibility**: Can we achieve the layouts we need (sidebars, overlays, responsive)?  
✅ **State Binding**: How do we update UI displays based on game state?  

### Success Criteria

This POC is successful if:

✅ All widgets render correctly and are interactive  
✅ We can wire up state changes from UI to game logic  
✅ We can update UI displays based on changing state  
✅ Performance overhead is reasonable (<1ms per frame)  
✅ Templates are easy to author and modify  
✅ WASM build works without issues  
✅ We have a clear recommendation on whether to adopt HUI  

## Expected Learning Outcomes

By completing this POC, we should know:

📊 **Performance Profile**: Exact frame time overhead of HUI system  
🎨 **Layout Capabilities**: Whether we can achieve desired UI layouts  
🔧 **Integration Complexity**: How much code is needed to wire up interactions  
🐛 **Limitations & Issues**: Any showstoppers or deal-breakers  
📱 **WASM Viability**: Whether it's production-ready for web deployment  
🎯 **Recommendation**: Clear go/no-go decision on adopting HUI for demos  

## Definition of Done

### 1. Demo Structure & Setup

- [ ] `demos/ui_demo/` crate created with proper structure
- [ ] `demos/ui_demo/Cargo.toml` with bevy_hui 0.4 dependency
- [ ] `DEMO_NAME` constant and `run_ui_demo()` function exported
- [ ] `src/main.rs` standalone entry point works
- [ ] `MockCompositorState` resource defined with all parameters
- [ ] Visual simulation module created (bouncing balls)

### 2. HUI Templates Created

- [ ] `control_panel.html` - Left sidebar with layer checkboxes + effect buttons
- [ ] `status_bar.html` - Top bar with FPS, ball count, active effects
- [ ] `parameter_panel.html` - Right sidebar with sliders + radio buttons for viz mode
- [ ] All templates use proper HUI syntax and render correctly
- [ ] Templates use absolute positioning for consistent layout

### 3. Visual Simulation Functional

- [ ] 400 colored balls spawn and bounce around viewport
- [ ] Simple physics (no Rapier needed - just bouncing)
- [ ] Burst force effect visualized (radial outward push)
- [ ] Wall pulse effect visualized (inward push from edges)
- [ ] Layer visibility respected (balls hide when GameWorld layer off)
- [ ] Pause/resume works correctly
- [ ] Reset respawns balls

### 4. UI Interactions Wired Up

- [ ] Layer visibility checkboxes control which layers render
- [ ] Burst force button triggers visual effect
- [ ] Wall pulse button triggers visual effect
- [ ] Burst parameter sliders update state correctly
- [ ] Wall pulse parameter sliders update state correctly
- [ ] Visualization mode radio buttons work
- [ ] FPS counter updates in real-time
- [ ] Ball count display updates when balls spawn/despawn
- [ ] Active effects indicator shows current effects

### 5. Keyboard Shortcuts Functional

- [ ] Keys 1-5 toggle layer visibility (mirror checkbox state)
- [ ] Space triggers burst force
- [ ] W triggers wall pulse
- [ ] P pauses/resumes simulation
- [ ] R resets simulation
- [ ] +/- adds/removes balls
- [ ] V cycles visualization mode
- [ ] F1 toggles UI visibility
- [ ] Esc exits demo

### 6. Documentation & Findings

- [ ] Comprehensive README created explaining POC purpose
- [ ] Widget support matrix documented (which widgets work, how well)
- [ ] Event handling approach documented (polling vs callbacks)
- [ ] Template syntax observations noted
- [ ] Performance measurements recorded (frame times with/without UI)
- [ ] Hot reload experience documented
- [ ] WASM compatibility verified
- [ ] Limitations and issues clearly listed
- [ ] Screenshots included showing UI layout
- [ ] Clear recommendation provided (adopt HUI or explore alternatives)

### 7. Testing & Validation

- [ ] Demo tested standalone: `cargo run -p ui_demo`
- [ ] All interactive controls tested and working
- [ ] Visual simulation behaves correctly
- [ ] WASM build successful: `pwsh scripts/wasm-dev.ps1`
- [ ] No clippy warnings introduced
- [ ] Performance acceptable (<1ms UI overhead)

## Implementation Phases

### Phase 1: Project Setup & Basic Structure (1.5 hours)

- Create `demos/ui_demo/` directory structure
- Set up `Cargo.toml` with bevy_hui 0.4 dependency
- Create `lib.rs` with `MockCompositorState` resource
- Create `main.rs` entry point
- Add camera setup system
- Verify basic app runs

### Phase 2: HUI Template Creation (2 hours)

- Create `assets/ui/` directory structure
- Write `control_panel.html` with checkboxes and buttons
- Write `status_bar.html` with text displays
- Write `parameter_panel.html` with sliders and radio buttons
- Test template loading and rendering
- Iterate on layout and styling

### Phase 3: Visual Simulation (2 hours)

- Implement ball spawning system (400 colored circles)
- Implement simple bouncing physics (no Rapier)
- Add burst force visual effect (radial push)
- Add wall pulse visual effect (inward push from edges)
- Test simulation runs smoothly

### Phase 4: UI Interaction Wiring (3 hours)

- Research HUI's event handling system (callbacks vs polling)
- Wire up layer visibility checkboxes
- Wire up effect trigger buttons
- Wire up parameter sliders (with value display updates)
- Wire up visualization mode radio buttons
- Wire up keyboard shortcuts (1-5, Space, W, P, R, V, F1, Esc)
- Test all interactions work correctly

### Phase 5: State Display Updates (1 hour)

- Implement FPS counter update system
- Implement ball count display updates
- Implement active effects indicator
- Add visual feedback when effects are active
- Polish UI responsiveness

### Phase 6: Testing, Documentation & Findings (1.5 hours)

- Test all features thoroughly
- Test WASM build and browser compatibility
- Measure performance (frame times with/without UI)
- Write comprehensive README with findings
- Document widget support matrix
- Note any issues or limitations discovered
- Provide clear recommendations for adoption
- Include screenshots of UI

**Total Estimated Time**: 11 hours

## Next Steps (After POC)

Based on the POC findings, we can decide:

### If HUI is suitable

1. **Integration Strategy**: Determine how to integrate with scaffold/existing demos
2. **Crate Organization**: Decide if we need a shared `ui_common` crate or demo-specific UI
3. **Widget Library**: Build reusable component templates
4. **Event System**: Establish patterns for UI→Game and Game→UI communication
5. **Performance Optimization**: Profile and optimize any bottlenecks
6. **Documentation**: Write comprehensive guides for adding UI to demos

### If HUI has limitations

1. **Alternative Evaluation**: Explore `bevy_egui` or `kayak_ui`
2. **Custom Solution**: Consider building minimal immediate-mode UI layer
3. **Hybrid Approach**: Use HUI for static layouts, immediate-mode for dynamic elements
4. **Defer Decision**: Continue with keyboard-only controls until better solution emerges

## UI Test Demo README Template

```markdown
## README Template (To be created in demos/ui_demo/)

```markdown
# UI Demo - Bevy-HUI Proof of Concept

## Purpose

This is a **standalone POC** exploring Bevy-HUI as a potential UI solution for interactive demo controls. It does NOT integrate with scaffold or any core crates—it's purely exploratory.

The demo simulates a compositor interface inspired by `compositor_test`, showcasing:

- Layer visibility controls
- Effect parameter adjustment
- Visualization mode selection
- Real-time status displays
- Visual simulation with interactive effects

## Features

### Mock Compositor Interface

- **5 Layer Toggles**: Background, GameWorld, Metaballs, Effects, UI (checkboxes + 1-5 keys)
- **Burst Force**: Button + 4 sliders (interval, duration, radius, strength)
- **Wall Pulse**: Button + 4 sliders (interval, duration, distance, strength)
- **Visualization Modes**: Radio buttons for Normal, Distance Field, Normals, Raw Compute
- **Status Display**: FPS counter, ball count, active effects indicator

### Visual Simulation

- 400 colored balls bouncing around viewport (no physics engine—just simple math)
- Burst force effect: Radial outward push from center
- Wall pulse effect: Inward push from edges
- Layer visibility: Balls hide/show based on GameWorld layer checkbox

### Widgets Demonstrated

- ✅ **Buttons**: Trigger effects, reset simulation
- ✅ **Checkboxes**: Layer visibility toggles
- ✅ **Sliders**: Parameter adjustment with real-time value display
- ✅ **Radio Buttons**: Visualization mode selection
- ✅ **Text Display**: Dynamic FPS counter, ball count, status messages

## Keyboard Controls

- `1-5`: Toggle layer visibility
- `Space`: Trigger burst force
- `W`: Trigger wall pulse
- `P`: Pause/resume simulation
- `R`: Reset simulation
- `+/-`: Add/remove balls
- `V`: Cycle visualization mode
- `F1`: Toggle all UI visibility
- `Esc`: Exit

## Running

### Standalone

```bash
cargo run -p ui_demo
```

### WASM (if compatible)

```powershell
# First build for web
wasm-pack build --target web demos/ui_demo

# Serve locally
python -m http.server 8000
# Then open http://localhost:8000
```

## Findings & Recommendations

### Widget Support

| Widget | Status | Notes |
|--------|--------|-------|
| Buttons | ✅ | [Note findings here] |
| Checkboxes | ✅ | [Note findings here] |
| Sliders | ✅ | [Note findings here] |
| Radio Buttons | ✅ | [Note findings here] |
| Text Display | ✅ | [Note findings here] |
| Dropdowns | ❓ | [Test if available] |

### Event Handling

[Document how HUI handles interactions - callbacks, polling, events?]

### Performance

- **UI Overhead**: X.XXms per frame
- **With/Without UI**: Compare frame times
- **WASM Performance**: [Note any differences]

### Template System

**Pros:**

- [List positives]

**Cons:**

- [List limitations]

### Overall Recommendation

**[✅ Adopt / ❌ Reject / ⚠️ Needs More Work]**

[Provide clear reasoning for recommendation]

## Architecture Notes

- **No Scaffold**: This demo intentionally avoids scaffold to keep it simple
- **No Physics Engine**: Uses basic math for bouncing (good enough for POC)
- **Mock State**: `MockCompositorState` resource tracks all UI state
- **Template-Based**: All UI defined in HTML files under `assets/ui/`
- **Standalone**: Can be evaluated independently of project architecture

## Next Steps

If HUI proves suitable:

1. Define integration strategy with scaffold
2. Create shared UI component library
3. Add UI to existing demos (physics_playground, compositor_test, metaballs_test)

If HUI has limitations:

1. Evaluate alternatives (bevy_egui, kayak_ui)
2. Consider hybrid approach
3. Document specific blockers

---

*This is a proof of concept—not production code.*

```
