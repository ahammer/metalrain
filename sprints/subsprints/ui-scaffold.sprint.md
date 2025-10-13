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

### 3. Demo Integration Patterns

- [ ] **Physics Playground**: Spawn mode toolbar + physics sliders
- [ ] **Metaballs Test**: Visualization mode selector (distance/normals/albedo)
- [ ] **Compositor Test**: Layer visibility checkboxes
- [ ] **Background Renderer**: Mode cycling with preview
- [ ] Document integration pattern in scaffold README

### 4. Keyboard Binding Registry

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

### Demo Integration: Physics Playground Example

```rust
// In physics_playground/Cargo.toml
[dependencies]
scaffold = { path = "../../crates/scaffold", features = ["debug_ui"] }

// In physics_playground/src/ui.rs
use bevy::prelude::*;
use bevy_immediate::prelude::*;
use scaffold::ui::{ScaffoldUiContext, positioning};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnMode {
    Balls,
    Walls,
    Obstacles,
}

impl SpawnMode {
    pub fn name(&self) -> &'static str {
        match self {
            SpawnMode::Balls => "Balls",
            SpawnMode::Walls => "Walls",
            SpawnMode::Obstacles => "Obstacles",
        }
    }
}

#[derive(Resource, Debug)]
pub struct PhysicsPlaygroundUi {
    pub spawn_mode: SpawnMode,
}

impl Default for PhysicsPlaygroundUi {
    fn default() -> Self {
        Self {
            spawn_mode: SpawnMode::Balls,
        }
    }
}

pub fn render_physics_controls(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    mut ui_state: ResMut<PhysicsPlaygroundUi>,
    mut physics: ResMut<game_physics::PhysicsConfig>,
    windows: Query<&Window>,
) {
    // Only render when panels are visible (F2 toggle)
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    
    // Position below scaffold HUD
    let pos = positioning::below_hud(window_size);

    ui::Window::new("physics_controls")
        .position(pos)
        .size([320.0, 250.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Physics Playground Controls");
            ui.separator();
            
            // Spawn mode selector (complement keyboard Tab shortcut)
            if ui.button(None, format!("Spawn Mode: {}", ui_state.spawn_mode.name())) {
                ui_state.spawn_mode = match ui_state.spawn_mode {
                    SpawnMode::Balls => SpawnMode::Walls,
                    SpawnMode::Walls => SpawnMode::Obstacles,
                    SpawnMode::Obstacles => SpawnMode::Balls,
                };
            }
            
            ui.separator();
            ui.label(None, "Physics Parameters");
            
            // Fine-grained gravity adjustment (complement arrow keys)
            ui.slider(None, "Gravity Y", &mut physics.gravity.y, -1000.0, 1000.0);
            
            // Clustering force (no keyboard shortcut alternative)
            ui.slider(None, "Clustering", &mut physics.clustering_strength, 0.0, 500.0);
            
            ui.separator();
            
            if ui.button(None, "Reset Physics (R)") {
                physics.gravity = Vec2::ZERO;
                physics.clustering_strength = 200.0;
            }
        });
}
```

### Demo Integration: Metaballs Test Example

```rust
// In metaballs_test/src/ui.rs
use bevy::prelude::*;
use bevy_immediate::prelude::*;
use scaffold::ui::{ScaffoldUiContext, positioning};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationMode {
    FinalComposite,
    DistanceField,
    Normals,
    Albedo,
}

impl VisualizationMode {
    pub fn name(&self) -> &'static str {
        match self {
            VisualizationMode::FinalComposite => "Final Composite",
            VisualizationMode::DistanceField => "Distance Field",
            VisualizationMode::Normals => "Normals",
            VisualizationMode::Albedo => "Albedo",
        }
    }
}

#[derive(Resource, Debug)]
pub struct MetaballVisualizationUi {
    pub mode: VisualizationMode,
}

pub fn render_metaball_debug_ui(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    mut viz: ResMut<MetaballVisualizationUi>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    let pos = positioning::calculate_position(
        window_size,
        positioning::Anchor::TopRight,
        Vec2::new(280.0, 180.0),
        10.0,
    );

    ui::Window::new("metaball_viz")
        .position(pos)
        .size([280.0, 180.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Visualization Mode");
            ui.separator();
            
            // Radio button group for mode selection
            if ui.radio_button(None, "Final Composite", viz.mode == VisualizationMode::FinalComposite) {
                viz.mode = VisualizationMode::FinalComposite;
            }
            if ui.radio_button(None, "Distance Field", viz.mode == VisualizationMode::DistanceField) {
                viz.mode = VisualizationMode::DistanceField;
            }
            if ui.radio_button(None, "Normals", viz.mode == VisualizationMode::Normals) {
                viz.mode = VisualizationMode::Normals;
            }
            if ui.radio_button(None, "Albedo", viz.mode == VisualizationMode::Albedo) {
                viz.mode = VisualizationMode::Albedo;
            }
            
            ui.separator();
            ui.label(None, "Keyboard: M to cycle modes");
        });
}
```

### Demo Integration: Compositor Test Example

```rust
// In compositor_test/src/ui.rs
use bevy::prelude::*;
use bevy_immediate::prelude::*;
use scaffold::ui::{ScaffoldUiContext, positioning};
use game_rendering::RenderLayer;

#[derive(Resource, Debug)]
pub struct LayerVisibilityUi {
    pub background: bool,
    pub game_world: bool,
    pub metaballs: bool,
    pub effects: bool,
    pub ui: bool,
}

impl Default for LayerVisibilityUi {
    fn default() -> Self {
        Self {
            background: true,
            game_world: true,
            metaballs: true,
            effects: true,
            ui: true,
        }
    }
}

pub fn render_compositor_layer_ui(
    mut ctx: NonSendMut<ImmediateContext>,
    scaffold_ctx: Res<ScaffoldUiContext>,
    mut visibility: ResMut<LayerVisibilityUi>,
    windows: Query<&Window>,
) {
    if !scaffold_ctx.panels_visible {
        return;
    }

    let window = windows.single();
    let window_size = Vec2::new(window.width(), window.height());
    let pos = positioning::calculate_position(
        window_size,
        positioning::Anchor::BottomRight,
        Vec2::new(250.0, 220.0),
        10.0,
    );

    ui::Window::new("compositor_layers")
        .position(pos)
        .size([250.0, 220.0])
        .show(&mut ctx, |ui| {
            ui.label(None, "Compositor Layers");
            ui.separator();
            
            // Checkboxes mirror keyboard shortcuts (1-5)
            ui.checkbox(None, "[1] Background", &mut visibility.background);
            ui.checkbox(None, "[2] Game World", &mut visibility.game_world);
            ui.checkbox(None, "[3] Metaballs", &mut visibility.metaballs);
            ui.checkbox(None, "[4] Effects", &mut visibility.effects);
            ui.checkbox(None, "[5] UI", &mut visibility.ui);
            
            ui.separator();
            
            if ui.button(None, "Show All") {
                visibility.background = true;
                visibility.game_world = true;
                visibility.metaballs = true;
                visibility.effects = true;
                visibility.ui = true;
            }
            
            if ui.button(None, "Hide All") {
                visibility.background = false;
                visibility.game_world = false;
                visibility.metaballs = false;
                visibility.effects = false;
                visibility.ui = false;
            }
        });
}
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

When adding debug UI to a demo:

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

**Available for Demos**:

- `F4-F12`: Demo-specific features
- `Tab`: Mode switching
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
✅ All three demo integrations working (physics/metaballs/compositor)  

## Definition of Done

- [ ] `UiScaffoldPlugin` feature-gated in scaffold crate
- [ ] `ScaffoldUiContext` resource with F2 toggle
- [ ] `positioning` helpers for window-relative layout
- [ ] `ScaffoldKeyBindings` registry with conflict detection
- [ ] F3 help panel showing all bindings by category
- [ ] Physics Playground UI integration complete
- [ ] Metaballs Test UI integration complete
- [ ] Compositor Test UI integration complete
- [ ] All demos tested on native and WASM
- [ ] Scaffold README updated with UI integration guide
- [ ] No conflicts with existing scaffold bindings

## Implementation Phases

### Phase 1: Foundation (2 hours)

- Add `bevy_immediate` dependency (feature-gated)
- Create `UiScaffoldPlugin` with F2 toggle
- Add `ScaffoldUiContext` resource
- Create `positioning` module with helpers

### Phase 2: Bindings System (1 hour)

- Implement `ScaffoldKeyBindings` resource
- Add conflict detection
- Create F3 help panel system
- Document all existing scaffold keys

### Phase 3: Demo Integration (3 hours)

- Physics Playground: spawn mode + physics sliders
- Metaballs Test: visualization mode radio buttons
- Compositor Test: layer visibility checkboxes
- Test all demos on native + WASM

### Phase 4: Documentation (1 hour)

- Update scaffold README with integration guide
- Document best practices and patterns
- Create integration checklist
- Add examples for common patterns

**Total Estimated Time**: 7 hours

## Future Enhancements (Out of Scope)

- **Preset System**: Save/load UI parameter configurations
- **Theme Support**: Customizable panel colors/fonts
- **Layout Persistence**: Remember panel positions between runs
- **Drag-and-Drop**: Reposition panels at runtime
- **Graph Widgets**: Real-time performance visualization
- **Command Palette**: Quick action search (Ctrl+P)

## Notes

This subsprint creates the **minimal viable UI infrastructure** for demos without introducing complexity that violates the Zero-Code Philosophy. The focus is on:

1. **Extending** (not replacing) existing scaffold patterns
2. **Composing** UI from simple, stateless widgets
3. **Integrating** with keyboard shortcuts for discoverability
4. **Maintaining** WASM compatibility and performance

The result is a lightweight, optional layer that demos can adopt as needed without coupling to complex UI framework state machines.

---

*UI Scaffold: Just enough interaction, nothing more.*
