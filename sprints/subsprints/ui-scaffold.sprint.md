# UI Scaffold Subsprint: Reusable Debug UI for Demos

## Sprint Goal

Create a lightweight, composable immediate-mode UI layer within the scaffold that provides interactive debug controls for demos without introducing complex state management or conflicting with existing scaffold patterns.

## Philosophy

The UI Scaffold follows Metalrain's **Zero-Code Philosophy**:

- **Lightweight**: Immediate-mode rendering, no complex panel management
- **Integrated**: Extends existing Scaffold HUD rather than replacing it
- **Composable**: Each demo adds only what it needs via simple systems
- **Minimal**: Text overlays preferred; interactive widgets only where necessary
- **Non-intrusive**: Works alongside existing keyboard shortcuts (F1, 1-5, arrow keys, etc.)

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

### 1. Lightweight UI Integration

- [ ] Add `bevy_immediate` as optional scaffold feature
- [ ] Create `UiScaffoldPlugin` for immediate-mode rendering
- [ ] Integrate with existing `ScaffoldHudState` toggle (F1 behavior)
- [ ] Define `ScaffoldUiContext` resource for demo UI state
- [ ] Add F2 as standard "interactive panel" toggle

### 2. Core Widget Helpers

- [ ] Window positioning helpers (relative to window edges)
- [ ] Button widget (emits events on click)
- [ ] Checkbox widget (direct state binding)
- [ ] Radio button group (enum selection)
- [ ] Slider widget (f32 parameter adjustment)
- [ ] Label/separator formatting utilities
- [ ] Header/footer layout components
- [ ] Sidebar layout components (left/right)
- [ ] Panel grouping and spacing utilities

### 3. UI Test Demo (NEW)

- [ ] Create `ui_test` demo crate with scaffold integration
- [ ] Implement header bar (top, persistent)
- [ ] Implement footer bar (bottom, persistent)
- [ ] Implement left sidebar (toggleable with Tab)
- [ ] Implement right sidebar (toggleable with ~)
- [ ] Demonstrate all widget types (buttons, checkboxes, sliders, radio buttons)
- [ ] Include widget state visualization
- [ ] Add color picker and theme selector demonstrations
- [ ] Integrate with demo_launcher
- [ ] Add `run_ui_test` export and DEMO_NAME constant

### 4. Demo Integration Examples (Existing Demos)

- [ ] **Physics Playground**: Spawn mode toolbar + physics sliders (example only)
- [ ] **Metaballs Test**: Visualization mode selector (example only)
- [ ] **Compositor Test**: Layer visibility checkboxes (example only)

### 5. Keyboard Binding Registry

- [ ] Central `ScaffoldKeyBindings` resource documenting reserved keys
- [ ] Conflict detection for demo-added bindings
- [ ] Runtime binding display (help panel via F3)

## Technical Specifications

### UI Scaffold Plugin (Feature-Gated)

```rust
// In scaffold/Cargo.toml
[dependencies]
bevy_immediate = { version = "0.3.0", optional = true }

[features]
debug_ui = ["bevy_immediate"]
```

```rust
// In scaffold/src/ui.rs
use bevy::prelude::*;

#[cfg(feature = "debug_ui")]
use bevy_immediate::prelude::*;

/// Optional plugin providing immediate-mode UI infrastructure for demos.
/// Enable with the "debug_ui" feature.
#[cfg(feature = "debug_ui")]
pub struct UiScaffoldPlugin;

#[cfg(feature = "debug_ui")]
impl Plugin for UiScaffoldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_immediate::ImmediateModePlugin)
            .init_resource::<ScaffoldUiContext>()
            .add_systems(Update, toggle_ui_panels);
    }
}

/// Context for demo-specific UI panels
#[derive(Resource, Debug, Default)]
pub struct ScaffoldUiContext {
    /// Whether interactive panels are visible (toggled with F2)
    pub panels_visible: bool,
    
    /// Demo-specific panel state (demos populate this)
    pub demo_state: Option<Box<dyn std::any::Any + Send + Sync>>,
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
```

### Widget Positioning Utilities

```rust
/// Helpers for positioning UI panels relative to window edges
pub mod positioning {
    use bevy::prelude::*;

    pub enum Anchor {
        TopLeft,
        TopRight,
        BottomLeft,
        BottomRight,
        Center,
    }

    pub fn calculate_position(
        window_size: Vec2,
        anchor: Anchor,
        panel_size: Vec2,
        margin: f32,
    ) -> [f32; 2] {
        match anchor {
            Anchor::TopLeft => [margin, margin],
            Anchor::TopRight => [window_size.x - panel_size.x - margin, margin],
            Anchor::BottomLeft => [margin, window_size.y - panel_size.y - margin],
            Anchor::BottomRight => [
                window_size.x - panel_size.x - margin,
                window_size.y - panel_size.y - margin,
            ],
            Anchor::Center => [
                (window_size.x - panel_size.x) * 0.5,
                (window_size.y - panel_size.y) * 0.5,
            ],
        }
    }

    /// Position panel below scaffold HUD (top-left, offset by 80px)
    pub fn below_hud(window_size: Vec2) -> [f32; 2] {
        [10.0, 80.0]
    }
}
```

### UI Test Demo: Comprehensive UI Pattern Demonstration

```rust
// In demos/ui_test/Cargo.toml
[package]
name = "ui_test"
version = "0.1.0"
edition = "2021"
license = "GPL-3.0"
publish = false

[dependencies]
bevy = { workspace = true }
scaffold = { path = "../../crates/scaffold", features = ["debug_ui"] }
game_assets = { path = "../../crates/game_assets" }

// In demos/ui_test/src/lib.rs
use bevy::prelude::*;
use bevy_immediate::prelude::*;
use scaffold::ui::{ScaffoldUiContext, positioning};
use scaffold::ScaffoldIntegrationPlugin;

pub const DEMO_NAME: &str = "ui_test";

#[derive(Resource, Debug)]
pub struct UiTestState {
    pub left_sidebar_visible: bool,
    pub right_sidebar_visible: bool,
    pub checkbox_demo: bool,
    pub slider_value: f32,
    pub radio_selection: WidgetDemo,
    pub button_click_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetDemo {
    Buttons,
    Checkboxes,
    Sliders,
    RadioButtons,
}

impl Default for UiTestState {
    fn default() -> Self {
        Self {
            left_sidebar_visible: true,
            right_sidebar_visible: true,
            checkbox_demo: false,
            slider_value: 50.0,
            radio_selection: WidgetDemo::Buttons,
            button_click_count: 0,
        }
    }
}

pub fn run_ui_test() {
    App::new()
        .add_plugins(ScaffoldIntegrationPlugin::with_demo_name(DEMO_NAME))
        .init_resource::<UiTestState>()
        .add_systems(
            Update,
            (
                toggle_sidebars,
                render_header,
                render_footer,
                render_left_sidebar,
                render_right_sidebar,
                render_center_panel,
            ),
        )
        .run();
}

/// Toggle sidebars with Tab (left) and ~ (right)
fn toggle_sidebars(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<UiTestState>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        state.left_sidebar_visible = !state.left_sidebar_visible;
        info!("Left sidebar: {}", state.left_sidebar_visible);
    }
    if keys.just_pressed(KeyCode::Backquote) {
        state.right_sidebar_visible = !state.right_sidebar_visible;
        info!("Right sidebar: {}", state.right_sidebar_visible);
    }
}

/// Persistent header bar at top
fn render_header(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    ui::Window::new("header")
        .position([0.0, 0.0])
        .size([window_size.x, 40.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "UI Test Demo - Header Bar");
            ui.same_line();
            ui.label(None, "[Tab] Toggle Left | [~] Toggle Right | [F2] Toggle UI");
        });
}

/// Persistent footer bar at bottom
fn render_footer(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    state: Res<UiTestState>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    ui::Window::new("footer")
        .position([0.0, window_size.y - 30.0])
        .size([window_size.x, 30.0])
        .show(&mut ctx, |ui| {
            ui.label(None, format!(
                "Status: Left={} | Right={} | Checkbox={} | Slider={:.1} | Clicks={}",
                state.left_sidebar_visible,
                state.right_sidebar_visible,
                state.checkbox_demo,
                state.slider_value,
                state.button_click_count,
            ));
        });
}

/// Left sidebar - Widget demonstrations
fn render_left_sidebar(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    mut state: ResMut<UiTestState>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible || !state.left_sidebar_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    ui::Window::new("left_sidebar")
        .position([0.0, 40.0])
        .size([250.0, window_size.y - 70.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Widget Demonstrations");
            ui.separator();
            
            // Button demo
            if ui.button(None, format!("Click Me! ({})", state.button_click_count)) {
                state.button_click_count += 1;
            }
            
            ui.separator();
            
            // Checkbox demo
            ui.checkbox(None, "Checkbox Demo", &mut state.checkbox_demo);
            
            ui.separator();
            ui.label(None, "Slider Demo");
            ui.slider(None, "Value", &mut state.slider_value, 0.0, 100.0);
            
            ui.separator();
            ui.label(None, "Radio Button Demo");
            
            if ui.radio_button(None, "Buttons", state.radio_selection == WidgetDemo::Buttons) {
                state.radio_selection = WidgetDemo::Buttons;
            }
            if ui.radio_button(None, "Checkboxes", state.radio_selection == WidgetDemo::Checkboxes) {
                state.radio_selection = WidgetDemo::Checkboxes;
            }
            if ui.radio_button(None, "Sliders", state.radio_selection == WidgetDemo::Sliders) {
                state.radio_selection = WidgetDemo::Sliders;
            }
            if ui.radio_button(None, "Radio Buttons", state.radio_selection == WidgetDemo::RadioButtons) {
                state.radio_selection = WidgetDemo::RadioButtons;
            }
        });
}

/// Right sidebar - State visualization
fn render_right_sidebar(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    state: Res<UiTestState>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible || !state.right_sidebar_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    ui::Window::new("right_sidebar")
        .position([window_size.x - 250.0, 40.0])
        .size([250.0, window_size.y - 70.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "State Inspector");
            ui.separator();
            
            ui.label(None, format!("Left Sidebar: {}", state.left_sidebar_visible));
            ui.label(None, format!("Right Sidebar: {}", state.right_sidebar_visible));
            ui.label(None, format!("Checkbox: {}", state.checkbox_demo));
            ui.label(None, format!("Slider: {:.2}", state.slider_value));
            ui.label(None, format!("Radio: {:?}", state.radio_selection));
            ui.label(None, format!("Button Clicks: {}", state.button_click_count));
            
            ui.separator();
            ui.label(None, "Layout Information");
            ui.label(None, format!("Window: {:.0}x{:.0}", window_size.x, window_size.y));
            ui.label(None, format!("Header Height: 40px"));
            ui.label(None, format!("Footer Height: 30px"));
            ui.label(None, format!("Sidebar Width: 250px"));
        });
}

/// Center panel - Active widget demonstration area
fn render_center_panel(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    state: Res<UiTestState>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    let left_offset = if state.left_sidebar_visible { 260.0 } else { 10.0 };
    let right_offset = if state.right_sidebar_visible { 260.0 } else { 10.0 };
    let panel_width = window_size.x - left_offset - right_offset;
    
    ui::Window::new("center_panel")
        .position([left_offset, 50.0])
        .size([panel_width, window_size.y - 90.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Widget Demonstration Area");
            ui.separator();
            
            match state.radio_selection {
                WidgetDemo::Buttons => {
                    ui.label(None, "Button Widgets");
                    ui.label(None, "Buttons emit events on click.");
                    ui.label(None, "Use for: Actions, mode switching, confirmations");
                }
                WidgetDemo::Checkboxes => {
                    ui.label(None, "Checkbox Widgets");
                    ui.label(None, "Checkboxes provide direct state binding.");
                    ui.label(None, "Use for: Boolean toggles, feature flags");
                }
                WidgetDemo::Sliders => {
                    ui.label(None, "Slider Widgets");
                    ui.label(None, "Sliders provide fine-grained parameter adjustment.");
                    ui.label(None, "Use for: Numeric parameters, percentages");
                }
                WidgetDemo::RadioButtons => {
                    ui.label(None, "Radio Button Widgets");
                    ui.label(None, "Radio buttons provide enum selection.");
                    ui.label(None, "Use for: Mutually exclusive options");
                }
            }
        });
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

### Keyboard Binding Registry

```rust
// In scaffold/src/bindings.rs

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

/// System to display help panel (F3)
pub fn render_help_panel(
    mut ctx: NonSendMut<ImmediateContext>,
    bindings: Res<ScaffoldKeyBindings>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
) {
    static mut HELP_VISIBLE: bool = false;
    
    if keys.just_pressed(KeyCode::F3) {
        unsafe { HELP_VISIBLE = !HELP_VISIBLE; }
    }
    
    if unsafe { !HELP_VISIBLE } {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    let pos = [
        (window_size.x - 500.0) * 0.5,
        (window_size.y - 600.0) * 0.5,
    ];

    ui::Window::new("help_panel")
        .position(pos)
        .size([500.0, 600.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Keyboard Bindings (F3 to close)");
            ui.separator();
            
            for category in [
                BindingCategory::Scaffold,
                BindingCategory::Camera,
                BindingCategory::Layers,
                BindingCategory::Debug,
                BindingCategory::Demo,
            ] {
                let category_bindings: Vec<_> = bindings.bindings.iter()
                    .filter(|b| b.category == category)
                    .collect();
                
                if category_bindings.is_empty() {
                    continue;
                }
                
                ui.label(None, format!("{:?}", category));
                
                for binding in category_bindings {
                    ui.label(None, format!("  {:?}: {}", binding.key, binding.description));
                }
                
                ui.separator();
            }
        });
}
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
✅ Use immediate-mode UI only for actual interaction  
✅ Position panels consistently (use `positioning` helpers)  
✅ Check `scaffold_ctx.panels_visible` before rendering  
✅ Mirror keyboard shortcuts in UI for discoverability  
✅ Use F2 toggle (don't override F1)  
✅ Register demo keys with `ScaffoldKeyBindings`  

### DON'T

❌ Override scaffold's F1/1-5/arrow key bindings  
❌ Create complex panel state management (it's immediate-mode!)  
❌ Hardcode positions in pixels (calculate from window size)  
❌ Store UI state in multiple places (one resource per demo)  
❌ Add UI for things that work fine as keyboard shortcuts  
❌ Introduce dependencies on higher-level game crates  

## Performance Considerations

- Immediate-mode UI has negligible overhead (~0.1ms per frame)
- Only render panels when `panels_visible` is true (F2 toggled on)
- Avoid complex calculations inside UI rendering (pre-compute in systems)
- Use resource changes for reactivity, not per-frame polling
- Panel count per demo should stay under 5 for visual clarity

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

### Core UI Scaffold Infrastructure

- [ ] `UiScaffoldPlugin` feature-gated in scaffold crate
- [ ] `ScaffoldUiContext` resource with F2 toggle
- [ ] `positioning` helpers for window-relative layout (including header/footer/sidebar utilities)
- [ ] `ScaffoldKeyBindings` registry with conflict detection
- [ ] F3 help panel showing all bindings by category
- [ ] Scaffold README updated with UI integration guide
- [ ] No conflicts with existing scaffold bindings

### UI Test Demo (Primary Deliverable)

- [ ] `demos/ui_test/` crate created with proper structure
- [ ] `DEMO_NAME` constant and `run_ui_test()` function exported
- [ ] Header bar component implemented and functional
- [ ] Footer bar component implemented and functional
- [ ] Left sidebar component implemented (toggleable with Tab)
- [ ] Right sidebar component implemented (toggleable with ~)
- [ ] Center panel component implemented with responsive layout
- [ ] Button widget demonstrations working
- [ ] Checkbox widget demonstrations working
- [ ] Slider widget demonstrations working
- [ ] Radio button widget demonstrations working
- [ ] State visualization in right sidebar updating correctly
- [ ] Demo added to `demo_launcher` dependencies
- [ ] Demo added to `demo_launcher` DEMOS array
- [ ] Workspace `Cargo.toml` updated with `ui_test` member
- [ ] `demos/ui_test/README.md` created with usage instructions
- [ ] Demo tested standalone: `cargo run -p ui_test`
- [ ] Demo tested via launcher: `cargo run -p demo_launcher ui_test`
- [ ] WASM build tested and verified working

## Implementation Phases

### Phase 1: Foundation (2 hours)

- Add `bevy_immediate` dependency (feature-gated)
- Create `UiScaffoldPlugin` with F2 toggle
- Add `ScaffoldUiContext` resource
- Create `positioning` module with helpers (including header/footer/sidebar positioning)

### Phase 2: Bindings System (1 hour)

- Implement `ScaffoldKeyBindings` resource
- Add conflict detection
- Create F3 help panel system
- Document all existing scaffold keys

### Phase 3: UI Test Demo Creation (4 hours)

- Create `demos/ui_test/` directory structure
- Set up `Cargo.toml` with dependencies
- Implement `lib.rs` with core demo logic:
  - `UiTestState` resource
  - Header rendering system
  - Footer rendering system
  - Left sidebar system (toggleable with Tab)
  - Right sidebar system (toggleable with ~)
  - Center panel system
  - Widget demonstration systems
- Create `main.rs` for standalone execution
- Create `README.md` with usage instructions

### Phase 4: Demo Launcher Integration (1 hour)

- Add `ui_test` to workspace `Cargo.toml`
- Add `ui_test` dependency to `demo_launcher`
- Import `run_ui_test` and `DEMO_NAME` in launcher
- Add demo entry to DEMOS array
- Test via launcher interface

### Phase 5: Testing & Documentation (2 hours)

- Test standalone: `cargo run -p ui_test`
- Test via launcher: `cargo run -p demo_launcher ui_test`
- Test WASM build with `pwsh scripts/wasm-dev.ps1`
- Update scaffold README with UI integration guide
- Document best practices and patterns
- Verify all keyboard shortcuts work correctly

**Total Estimated Time**: 10 hours

## Future Enhancements (Out of Scope)

- **Preset System**: Save/load UI parameter configurations
- **Theme Support**: Customizable panel colors/fonts
- **Layout Persistence**: Remember panel positions between runs
- **Drag-and-Drop**: Reposition panels at runtime
- **Graph Widgets**: Real-time performance visualization
- **Command Palette**: Quick action search (Ctrl+P)
- **Color Picker Widget**: Interactive HSV/RGB color selection
- **Text Input Widget**: Editable text fields for numeric/string parameters
- **Existing Demo Integrations**: Physics Playground, Metaballs Test, Compositor Test (defer to future sprint)
- **Multi-Window Support**: Detachable panels

## UI Test Demo README Template

```markdown
# UI Test Demo

Comprehensive demonstration of the UI Scaffold system showcasing layout patterns and widget types.

## Purpose

This demo validates the UI Scaffold infrastructure and serves as a reference implementation for:

- **Header/Footer patterns**: Persistent bars at top and bottom
- **Sidebar patterns**: Toggleable left/right panels
- **Responsive layouts**: Center content adapts to sidebar visibility
- **Widget demonstrations**: All core UI widget types
- **State management**: Real-time state visualization

## Features

### Layout Components

- **Header Bar**: Persistent information bar at top
- **Footer Bar**: Status display at bottom
- **Left Sidebar**: Widget demonstrations (toggle with Tab)
- **Right Sidebar**: State inspector (toggle with ~)
- **Center Panel**: Context-sensitive demonstration area

### Widget Demonstrations

- **Buttons**: Click counter demonstration
- **Checkboxes**: Boolean toggle demonstration
- **Sliders**: Continuous value adjustment (0-100)
- **Radio Buttons**: Mutually exclusive selection among widget types

### Keyboard Controls

- `F1`: Toggle scaffold HUD (performance stats)
- `F2`: Toggle all UI panels
- `F3`: Show help/key bindings
- `Tab`: Toggle left sidebar
- `~` (Backquote): Toggle right sidebar
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
# Then select ui_test from launcher
```

## Architecture

This demo uses:

- **Scaffold Integration**: `ScaffoldIntegrationPlugin` for baseline functionality
- **Immediate-Mode UI**: `bevy_immediate` for stateless widget rendering
- **Resource-Based State**: Single `UiTestState` resource for all demo state
- **System-Based Rendering**: Each layout component has its own render system

## Integration Pattern

The UI Test demo demonstrates the recommended pattern for adding debug UI to demos:

1. **Feature-gate**: Add `scaffold = { features = ["debug_ui"] }` to Cargo.toml
2. **State Resource**: Create single resource for demo UI state
3. **Render Systems**: One system per layout component
4. **Visibility Check**: Respect `ScaffoldUiContext::panels_visible` (F2 toggle)
5. **Positioning**: Use `positioning` helpers for consistent layout
6. **Keyboard Bindings**: Register custom bindings with `ScaffoldKeyBindings`

## Best Practices Demonstrated

✅ Immediate-mode rendering (no complex state management)
✅ Responsive layout (adapts to sidebar visibility)
✅ Consistent positioning (uses scaffold positioning helpers)
✅ Performance-conscious (only renders when panels visible)
✅ Keyboard-friendly (all controls accessible via keyboard)
✅ WASM-compatible (no platform-specific dependencies)

## Future Enhancements

- Color picker widget demonstration
- Text input widget demonstration
- Graph/plot widget demonstration
- Drag-and-drop panel repositioning
- Theme selector demonstration

```

## Notes

This subsprint creates the **minimal viable UI infrastructure** for demos without introducing complexity that violates the Zero-Code Philosophy. The focus is on:

1. **Extending** (not replacing) existing scaffold patterns
2. **Composing** UI from simple, stateless widgets
3. **Integrating** with keyboard shortcuts for discoverability
4. **Maintaining** WASM compatibility and performance
5. **Demonstrating** layout patterns through the UI Test demo

The result is a lightweight, optional layer that demos can adopt as needed without coupling to complex UI framework state machines. The **UI Test demo** serves as both validation and reference implementation.

---

*UI Scaffold: Just enough interaction, nothing more.*
