# UI Scaffold Subsprint: Reusable Debug UI for Demos

## Sprint Goal

Create a lightweight, composable template-based UI layer using **Bevy-HUI** that provides interactive debug controls for demos without introducing complex state management or conflicting with existing scaffold patterns.

**Primary Deliverable**: A new `basic_ui` crate that provides HUI-based UI infrastructure, integrated through scaffold, and demonstrated in the **UI Test demo** (`demos/ui_test`).

## Architecture Overview

```
┌─────────────────┐
│   basic_ui      │ ← New crate: HUI templates & utilities
│  (HUI-based)    │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│   scaffold      │ ← Integrates basic_ui via feature flag
│ (debug_ui flag) │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│   demos/*       │ ← Demos use scaffold with debug_ui feature
│  (ui_test, etc) │
└─────────────────┘
```

## Philosophy

The UI Scaffold follows Metalrain's **Zero-Code Philosophy**:

- **Template-based**: Uses Bevy-HUI's HTML-style templates (not immediate-mode)
- **Declarative**: UI layouts defined in pseudo-HTML syntax
- **Integrated**: Extends existing Scaffold HUD rather than replacing it
- **Composable**: Each demo adds only what it needs via template includes
- **Minimal**: Text overlays preferred; interactive widgets only where necessary
- **Non-intrusive**: Works alongside existing keyboard shortcuts (F1, 1-5, arrow keys, etc.)
- **Hot-reload friendly**: Templates can be edited and reloaded during development

## Problem Statement

Currently, demos use:

1. **Text overlays** via Scaffold's performance HUD (good for stats display)
2. **Keyboard shortcuts** for toggling features (good for binary controls)
3. **Arrow keys** for parameter adjustment (limited precision)

However, some demo scenarios need:

- **Mode switching** that's more discoverable than keyboard shortcuts
- **Fine-grained parameter tuning** beyond arrow key increments
- **Multi-option selection** (radio buttons for visualization modes)
- **Real-time value inspection** with interactive adjustment
- **Contextual controls** that appear/hide based on demo phase

## Deliverables

### 1. New `basic_ui` Crate (Core Infrastructure)

- [ ] Create `crates/basic_ui` with Bevy-HUI dependency
- [ ] Define reusable HTML component templates:
  - [ ] Header bar template (`templates/header.html`)
  - [ ] Footer bar template (`templates/footer.html`)
  - [ ] Sidebar templates (`templates/sidebar_left.html`, `templates/sidebar_right.html`)
  - [ ] Panel container templates with common styling
  - [ ] Button/control widget templates
- [ ] Create `BasicUiPlugin` for template registration
- [ ] Export template loading utilities
- [ ] Document template customization patterns

### 2. Scaffold Integration (Feature-Gated)

- [ ] Add `bevy_hui` as optional dependency in scaffold (via `basic_ui`)
- [ ] Create `UiScaffoldPlugin` that wraps `BasicUiPlugin`
- [ ] Integrate with existing `ScaffoldHudState` toggle (F1 behavior)
- [ ] Define `ScaffoldUiContext` resource for demo UI state
- [ ] Add F2 as standard "interactive panel" toggle
- [ ] Implement F3 help panel template

### 3. Keyboard Binding Registry (Scaffold)

- [ ] Central `ScaffoldKeyBindings` resource documenting reserved keys
- [ ] Conflict detection for demo-added bindings
- [ ] Help panel template showing all bindings (F3 toggle)
- [ ] Export binding registration utilities for demos

### 4. UI Test Demo (Validation & Reference)

- [ ] Create `demos/ui_test` crate
- [ ] Implement custom templates extending `basic_ui`:
  - [ ] Header bar (demonstrates template customization)
  - [ ] Footer bar with status display
  - [ ] Left sidebar (widget demonstrations, toggleable with Tab)
  - [ ] Right sidebar (state inspector, toggleable with ~)
  - [ ] Center panel (content area)
- [ ] Demonstrate template property injection
- [ ] Include interactive widget examples (buttons, controls)
- [ ] Integrate with demo_launcher
- [ ] Add `run_ui_test` export and DEMO_NAME constant
- [ ] Create comprehensive README with usage patterns

### 5. Documentation & Examples

- [ ] Document `basic_ui` template structure in crate README
- [ ] Provide template customization guide
- [ ] Document scaffold integration pattern for new demos
- [ ] Include example of registering custom HUI functions
- [ ] Note future integration patterns for existing demos (Physics, Metaballs, Compositor)

## Technical Specifications

### 1. basic_ui Crate Structure

```toml
# crates/basic_ui/Cargo.toml
[package]
name = "basic_ui"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0"
description = "Reusable HUI-based UI templates for Metalrain demos"

[dependencies]
bevy = { workspace = true }
bevy_hui = "0.4"  # Compatible with Bevy 0.16
```

```rust
// crates/basic_ui/src/lib.rs
use bevy::prelude::*;
use bevy_hui::prelude::*;

pub struct BasicUiPlugin;

impl Plugin for BasicUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HuiPlugin)
            .add_systems(Startup, register_templates);
    }
}

fn register_templates(
    asset_server: Res<AssetServer>,
    mut html_components: ResMut<HtmlComponents>,
) {
    // Register reusable component templates
    html_components.register("ui_header", asset_server.load("ui/templates/header.html"));
    html_components.register("ui_footer", asset_server.load("ui/templates/footer.html"));
    html_components.register("ui_sidebar_left", asset_server.load("ui/templates/sidebar_left.html"));
    html_components.register("ui_sidebar_right", asset_server.load("ui/templates/sidebar_right.html"));
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

### 3. HUI Template Examples

```html
<!-- crates/basic_ui/assets/ui/templates/header.html -->
<template>
    <property name="title">Demo Title</property>
    
    <node 
        position_type="absolute"
        top="0px"
        left="0px"
        right="0px"
        height="40px"
        background="#1a1a1a"
        border_bottom="2px"
        border_color="#444444"
        padding="10px"
        display="flex"
        align_items="center"
    >
        <text font_size="20" font_color="#ffffff">
            {title}
        </text>
    </node>
</template>
```

```html
<!-- crates/basic_ui/assets/ui/templates/sidebar_left.html -->
<template>
    <node 
        position_type="absolute"
        left="0px"
        top="40px"
        bottom="30px"
        width="250px"
        background="#252525"
        border_right="2px"
        border_color="#444444"
        padding="15px"
        display="flex"
        flex_direction="column"
    >
        <!-- Content injected by extending templates -->
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

## Integration Checklist

### For the UI Test Demo (Primary Deliverable)

- [ ] Create `demos/ui_test/` directory structure
- [ ] Add `demos/ui_test/Cargo.toml` with proper dependencies
- [ ] Create `demos/ui_test/src/lib.rs` with `run_ui_test()` and `DEMO_NAME`
- [ ] Create `demos/ui_test/src/main.rs` for standalone execution
- [ ] Implement header bar (persistent, top)
- [ ] Implement footer bar (persistent, bottom)
- [ ] Implement left sidebar (toggleable with Tab)
- [ ] Implement right sidebar (toggleable with ~)
- [ ] Add all widget demonstrations (buttons, checkboxes, sliders, radio buttons)
- [ ] Add state visualization in right sidebar
- [ ] Add demo to `demo_launcher` dependencies
- [ ] Add demo to `demo_launcher` DEMOS array
- [ ] Update workspace `Cargo.toml` to include `ui_test` member
- [ ] Create `demos/ui_test/README.md` with usage instructions
- [ ] Test standalone: `cargo run -p ui_test`
- [ ] Test via launcher: `cargo run -p demo_launcher ui_test`
- [ ] Verify WASM compatibility with `pwsh scripts/wasm-dev.ps1`

### For Future Demo Integrations (Optional Reference)

- [ ] Add `scaffold = { features = ["debug_ui"] }` to demo's `Cargo.toml`
- [ ] Add `UiScaffoldPlugin` to app (if using interactive panels)
- [ ] Create demo-specific UI state resource
- [ ] Add render system using `ImmediateContext` in `Update`
- [ ] Check `ScaffoldUiContext::panels_visible` before rendering
- [ ] Position panels to avoid obscuring gameplay
- [ ] Use `positioning` helpers for consistent layout
- [ ] Register demo-specific key bindings with `ScaffoldKeyBindings`
- [ ] Document controls in demo's README
- [ ] Test panel toggle (F2) and help display (F3)
- [ ] Verify WASM compatibility

## Reserved Keyboard Shortcuts

**Scaffold Core** (DO NOT override):

- `F1`: Toggle scaffold HUD (stats/diagnostics)
- `F2`: Toggle interactive UI panels (debug_ui feature)
- `F3`: Show help/key bindings
- `1-5`: Layer toggles (Background, GameWorld, Metaballs, Effects, UI)
- `[`/`]`: Exposure adjustment
- `-`/`=`: Camera zoom
- `Space`: Camera shake
- `R`: Reset camera/physics
- `P`: Pause physics
- `Arrow keys`: Gravity adjustment
- `B`: Background mode cycle
- `M`: Metaball mode cycle
- `Esc`: Exit

**UI Test Demo Specific**:

- `Tab`: Toggle left sidebar
- `~` (Backquote): Toggle right sidebar
- `F2`: Toggle all UI panels (inherited from scaffold)

**Available for Other Demos**:

- `F4-F12`: Demo-specific features
- `Q`/`E`: Demo-specific cycles
- `T`/`Y`/`U`/`I`/`O`: Parameter adjustments
- `A`/`S`/`D`/`F`/`G`: Actions
- `Z`/`X`/`C`/`V`: Additional controls
- `Mouse`: Spawn/select/interact

## Best Practices

### DO

✅ Start with text overlays (extend scaffold HUD)  
✅ Use HUI templates for declarative layouts  
✅ Position panels consistently using absolute positioning with named anchors  
✅ Toggle panel visibility via `Visibility` component (respecting `panels_visible`)  
✅ Mirror keyboard shortcuts in UI for discoverability  
✅ Use F2 toggle (don't override F1)  
✅ Register demo keys with `ScaffoldKeyBindings`  
✅ Store UI entity handles in demo state resource  
✅ Use property injection for dynamic content updates (future enhancement)  

### DON'T

❌ Override scaffold's F1/1-5/arrow key bindings  
❌ Spawn UI templates every frame (spawn once in Startup, toggle visibility)  
❌ Hardcode positions in pixels (use percentages, flex layouts, or relative units)  
❌ Store UI state in multiple places (one resource per demo)  
❌ Add UI for things that work fine as keyboard shortcuts  
❌ Introduce dependencies on higher-level game crates  
❌ Create complex interactive widgets in first iteration (keep it simple)  

## Performance Considerations

- HUI templates have minimal overhead (~0.05ms per frame for 5 panels)
- Spawning templates once in Startup avoids per-frame allocations
- Visibility toggling is nearly free (component flag flip)
- Property injection (future) enables reactive updates without full respawn
- Panel count per demo should stay under 5 for visual clarity
- Template loading happens asynchronously during asset loading phase

## Success Criteria

✅ UI scaffold integrates seamlessly with existing scaffold infrastructure  
✅ F1 continues to toggle scaffold HUD as expected  
✅ F2 toggles interactive panels independently  
✅ F3 displays comprehensive help with all bindings  
✅ No conflicts with existing keyboard shortcuts  
✅ Demos can add panels without modifying scaffold core  
✅ WASM compatibility maintained  
✅ Performance impact < 0.5ms per frame  
✅ **UI Test demo fully functional and integrated:**

- ✅ Header bar displays at top (persistent)
- ✅ Footer bar displays at bottom (persistent)
- ✅ Left sidebar toggles with Tab
- ✅ Right sidebar toggles with ~
- ✅ All widgets demonstrate correctly (buttons, checkboxes, sliders, radio buttons)
- ✅ State visualization updates in real-time
- ✅ Accessible via demo_launcher
- ✅ Works standalone (`cargo run -p ui_test`)
- ✅ WASM build successful  

## Definition of Done

### 1. basic_ui Crate (New Crate - Foundation)

- [ ] `crates/basic_ui/` crate created with proper structure
- [ ] `basic_ui/Cargo.toml` with bevy_hui 0.4 dependency
- [ ] `HuiPlugin` integrated in basic_ui plugin
- [ ] Five HUI templates created in `basic_ui/assets/ui/`:
  - [ ] `header.html` - Top bar (40px height, dark theme)
  - [ ] `footer.html` - Bottom status bar (30px height)
  - [ ] `left_sidebar.html` - Left panel (250px width)
  - [ ] `right_sidebar.html` - Right panel (250px width)
  - [ ] `center_panel.html` - Main content area (responsive)
- [ ] `help_panel.html` template for F3 help display
- [ ] `BasicUiPlugin` exports HUI functionality
- [ ] Templates use absolute positioning with flexbox where appropriate
- [ ] Property injection points documented in templates (for future use)

### 2. Scaffold Integration (Feature-Gated)

- [ ] `scaffold/Cargo.toml` updated with `basic_ui` optional dependency
- [ ] `debug_ui` feature flag properly gates basic_ui
- [ ] `UiScaffoldPlugin` integrates `BasicUiPlugin`
- [ ] `ScaffoldUiContext` resource tracks panel visibility and help panel entity
- [ ] F2 toggle system for `panels_visible` flag
- [ ] F3 toggle system spawns/despawns help panel HtmlNode
- [ ] `ScaffoldKeyBindings` registry with 23 predefined bindings
- [ ] Conflict detection logic for key binding registration
- [ ] `scaffold/src/ui/` module structure properly organized
- [ ] Scaffold README updated with HUI integration guide
- [ ] No conflicts with existing scaffold bindings (F1/1-5/arrows reserved)

### 3. UI Test Demo (Primary Deliverable)

- [ ] `demos/ui_test/` crate created with proper structure
- [ ] `DEMO_NAME` constant and `run_ui_test()` function exported
- [ ] Five HTML templates copied to `demos/ui_test/assets/ui/`:
  - [ ] `header.html` loaded and spawned
  - [ ] `footer.html` loaded and spawned
  - [ ] `left_sidebar.html` loaded and spawned
  - [ ] `right_sidebar.html` loaded and spawned
  - [ ] `center_panel.html` loaded and spawned
- [ ] `UiTestState` resource tracks entity handles for all panels
- [ ] F2 toggle system updates panel visibility (sidebars + center panel)
- [ ] Header and footer remain visible (persistent)
- [ ] Templates demonstrate HUI layout capabilities (no interactive widgets needed)
- [ ] Demo added to `demo_launcher` dependencies
- [ ] Demo added to `demo_launcher` DEMOS array
- [ ] Workspace `Cargo.toml` updated with `ui_test` member
- [ ] `demos/ui_test/README.md` created with usage instructions
- [ ] Demo tested standalone: `cargo run -p ui_test`
- [ ] Demo tested via launcher: `cargo run -p demo_launcher ui_test`
- [ ] WASM build tested and verified working

### 4. Documentation & Testing

- [ ] Scaffold README updated with HUI integration patterns
- [ ] Template customization guide documented
- [ ] Best practices section reflects HUI approach (not immediate-mode)
- [ ] Reserved keyboard shortcuts documented (F1/F2/F3/1-5/arrows)
- [ ] All keyboard shortcuts tested and working
- [ ] WASM compatibility verified with `pwsh scripts/wasm-dev.ps1`
- [ ] Build passes: `cargo build --all`
- [ ] No new clippy warnings introduced

## Implementation Phases

### Phase 1: basic_ui Crate Foundation (3 hours)

- Create `crates/basic_ui/` directory structure
- Add `basic_ui/Cargo.toml` with bevy_hui dependency
- Implement `BasicUiPlugin` integrating `HuiPlugin`
- Create five HTML templates in `assets/ui/templates/`:
  - `header.html` - Top bar with title injection point
  - `footer.html` - Bottom status bar with property injection
  - `left_sidebar.html` - Left panel with flexbox layout
  - `right_sidebar.html` - Right panel with state display
  - `center_panel.html` - Main responsive content area
- Create `help_panel.html` template for F3 display
- Document template structure and customization points

### Phase 2: Scaffold Integration (2 hours)

- Add `basic_ui` optional dependency to scaffold
- Create `debug_ui` feature flag
- Implement `UiScaffoldPlugin` with `BasicUiPlugin` integration
- Add `ScaffoldUiContext` resource (panels_visible, help_panel_entity)
- Implement F2 toggle system for `panels_visible`
- Implement F3 toggle system (spawn/despawn help HtmlNode)
- Add `ScaffoldKeyBindings` resource with 23 bindings
- Implement conflict detection for key registration

### Phase 3: UI Test Demo Creation (3 hours)

- Create `demos/ui_test/` directory structure
- Set up `Cargo.toml` with bevy_hui and scaffold dependencies
- Copy HTML templates from basic_ui to `demos/ui_test/assets/ui/`
- Implement `lib.rs`:
  - `UiTestState` resource with entity handles
  - `setup_demo_ui` system spawning five HtmlNodes
  - `toggle_panels` system for F2 handling
  - `update_panel_visibility` system toggling Visibility component
- Create `main.rs` standalone entry point
- Create `README.md` with usage instructions

### Phase 4: Demo Launcher Integration (1 hour)

- Add `ui_test` to workspace `Cargo.toml` members
- Add `ui_test` dependency to `demo_launcher/Cargo.toml`
- Import `run_ui_test` and `DEMO_NAME` in launcher
- Add demo entry to DEMOS array with description
- Test via launcher interface

### Phase 5: Testing & Documentation (1 hour)

- Test standalone: `cargo run -p ui_test`
- Test via launcher: `cargo run -p demo_launcher ui_test`
- Test WASM build with `pwsh scripts/wasm-dev.ps1`
- Update scaffold README with HUI integration guide
- Document template-based patterns (not immediate-mode)
- Verify all keyboard shortcuts work correctly
- Run full workspace build: `cargo build --all`
- Check for clippy warnings: `cargo clippy --all`

**Total Estimated Time**: 10 hours

## Future Enhancements (Out of Scope)

### Template System Extensions

- **Property Injection System**: Dynamic content updates via property bindings
- **Custom HUI Functions**: Register Rust functions callable from templates
- **Template Inheritance**: Base templates with extending child templates
- **Component Templates**: Reusable UI component library
- **Conditional Rendering**: Show/hide template sections based on state

### Interactive Widgets (HUI v0.5+)

- **Button Handlers**: Click event routing to Bevy systems
- **Text Input Fields**: Editable text with validation
- **Slider Widgets**: Interactive parameter adjustment
- **Checkbox/Radio**: Toggle and selection widgets
- **Dropdown Menus**: Dynamic option lists

### Advanced Features

- **Theme Support**: CSS-style theme files with color schemes
- **Layout Persistence**: Save panel positions to config file
- **Preset System**: Named UI configurations (save/load)
- **Graph Widgets**: Real-time performance charts
- **Command Palette**: Quick action search (Ctrl+P)
- **Multi-Window Support**: Detachable panels

### Demo Integrations (Future Sprint)

- **Physics Playground**: Spawn mode selector, physics parameter controls
- **Metaballs Test**: Visualization mode selector, compute settings
- **Compositor Test**: Layer visibility toggles, blend mode controls
- **Architecture Test**: ECS inspection, system ordering visualization

## UI Test Demo README Template

```markdown
# UI Test Demo

Comprehensive demonstration of the UI Scaffold system showcasing HUI template-based layout patterns.

## Purpose

This demo validates the UI Scaffold infrastructure using Bevy-HUI templates and serves as a reference implementation for:

- **Header/Footer patterns**: Persistent bars at top and bottom
- **Sidebar patterns**: Toggleable left/right panels  
- **Responsive layouts**: Center content adapts to sidebar visibility
- **Template-based UI**: Declarative HTML-style UI definitions
- **Visibility management**: ECS-based show/hide patterns

## Features

### Layout Components

- **Header Bar**: Persistent title bar at top (40px height)
- **Footer Bar**: Status display at bottom (30px height)
- **Left Sidebar**: Widget demonstrations placeholder (250px width, F2 to toggle)
- **Right Sidebar**: State inspector placeholder (250px width, F2 to toggle)
- **Center Panel**: Main content area explaining HUI features

### Template System

- **HTML Templates**: All UI defined in `.html` files under `assets/ui/`
- **Absolute Positioning**: Panels use CSS-like absolute positioning
- **Flexbox Layouts**: Internal panel layouts use flexbox (column/row)
- **Property Injection**: Placeholders for dynamic content (future enhancement)
- **Hot Reload**: Templates can be edited and reloaded during development

### Keyboard Controls

- `F1`: Toggle scaffold HUD (performance stats)
- `F2`: Toggle UI sidebars and center panel
- `F3`: Show help/key bindings
- `Esc`: Exit demo

## Running

### Standalone

```bash
cargo run -p ui_test
```

### Via Demo Launcher

```bash
cargo run -p demo_launcher ui_test
```

### WASM

```powershell
pwsh scripts/wasm-dev.ps1
# Then select "UI Layout Patterns" from launcher
```

## Architecture

This demo uses:

- **Scaffold Integration**: `UiScaffoldPlugin` for F2/F3 toggle functionality
- **Bevy-HUI**: Template-based declarative UI system
- **Resource-Based State**: `UiTestState` tracks entity handles for all panels
- **Visibility Component**: Standard Bevy `Visibility` for show/hide

## Integration Pattern

The UI Test demo demonstrates the recommended pattern for adding HUI UI to demos:

1. **Add Dependencies**: `bevy_hui = "0.4"` and `scaffold = { features = ["debug_ui"] }`
2. **Add HuiPlugin**: Include in app plugin list
3. **Create Templates**: Define UI in HTML files under `assets/ui/`
4. **Spawn Once**: Load `HtmlNode` components in Startup system
5. **Store Handles**: Track entity IDs in demo state resource
6. **Toggle Visibility**: Update `Visibility` component based on F2 state
7. **Register Bindings**: Add custom keys to `ScaffoldKeyBindings`

## Best Practices Demonstrated

✅ Template-based declarative UI (no per-frame rendering logic)
✅ Spawn once in Startup (no runtime allocations)
✅ Visibility toggling (efficient component flag flip)
✅ Entity handle tracking (proper ECS resource management)
✅ Keyboard-friendly (all controls accessible via F2)
✅ WASM-compatible (no platform-specific dependencies)

## Future Enhancements

- Property injection for dynamic content updates
- Interactive widget event handlers (buttons, sliders)
- Custom HUI function registration from Rust
- Template inheritance and component library
- Theme system with color customization

## Notes

This subsprint creates the **basic_ui crate** and **minimal viable UI infrastructure** using Bevy-HUI templates without introducing complexity that violates the Zero-Code Philosophy. The focus is on:

1. **Separating Concerns**: basic_ui crate provides reusable template foundation
2. **Template-Based**: Declarative HTML-style UI definitions (not immediate-mode)
3. **Integrating**: Scaffold provides F2/F3 toggle functionality via optional feature
4. **Maintaining**: WASM compatibility and minimal performance overhead
5. **Demonstrating**: ui_test demo shows layout patterns and visibility management

The result is a lightweight, optional layer with proper crate separation:

- **basic_ui**: Reusable HUI template infrastructure
- **scaffold**: Integrates basic_ui via `debug_ui` feature, adds F2/F3 toggles
- **demos**: Use scaffold with `debug_ui` feature to access UI templates

The **UI Test demo** serves as both validation and reference implementation.

## Quick Start Implementation Guide

### Step 1: Create basic_ui Crate

```bash
# Create directory structure
mkdir -p crates/basic_ui/src
mkdir -p crates/basic_ui/assets/ui/templates

# Create files:
# - crates/basic_ui/Cargo.toml (with bevy_hui = "0.4")
# - crates/basic_ui/src/lib.rs (BasicUiPlugin)
# - crates/basic_ui/assets/ui/templates/*.html (five templates + help_panel)
```

### Step 2: Scaffold Integration

```toml
# In crates/scaffold/Cargo.toml
[dependencies]
basic_ui = { path = "../basic_ui", optional = true }

[features]
debug_ui = ["basic_ui"]
```

```rust
// In crates/scaffold/src/ui/plugin.rs
use basic_ui::BasicUiPlugin;

#[cfg(feature = "debug_ui")]
impl Plugin for UiScaffoldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BasicUiPlugin)
            .init_resource::<ScaffoldUiContext>()
            // ... F2/F3 toggle systems
    }
}
```

### Step 3: Create UI Test Demo

```bash
# Create directory structure
mkdir -p demos/ui_test/src
mkdir -p demos/ui_test/assets/ui

# Copy templates from basic_ui to demos/ui_test/assets/ui/
# Create files:
# - demos/ui_test/Cargo.toml
# - demos/ui_test/src/lib.rs (with HtmlNode spawning)
# - demos/ui_test/src/main.rs
# - demos/ui_test/README.md
```

### Step 4: Integrate with Launcher

```toml
# In demos/demo_launcher/Cargo.toml
ui_test = { path = "../ui_test" }
```

```rust
// In demos/demo_launcher/src/main.rs
use ui_test::{run_ui_test, DEMO_NAME as UI_TEST_DEMO};

// Add to DEMOS array
DemoEntry {
    name: UI_TEST_DEMO,
    run: run_ui_test,
    description: "UI layout patterns with Bevy-HUI",
},
```

### Step 5: Update Workspace & Test

```toml
# In workspace Cargo.toml [workspace.members]
"crates/basic_ui",
"demos/ui_test",
```

```bash
# Build all
cargo build --all

# Standalone
cargo run -p ui_test

# Via launcher
cargo run -p demo_launcher ui_test

# WASM
pwsh scripts/wasm-dev.ps1
```

---

*UI Scaffold: Just enough interaction, nothing more.*
