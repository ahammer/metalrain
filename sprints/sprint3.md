# Sprint 3: Rendering Pipeline Architecture

## Sprint Goal

Establish the multi-layer rendering architecture that coordinates all visual subsystems. Build upon the decoupled metaball renderer from Sprint 2.1 to create a unified rendering pipeline with proper layer compositing and camera management.

## Current State Assessment

- ✅ Metaball renderer successfully decoupled from camera (Sprint 2.1)
- ✅ Coordinate mapping utilities available (`project_world_to_screen`, `screen_to_world`, etc.)
- ✅ MetaballViewMapper resource implemented for coordinate transformations
- ✅ Physics foundation established with ball entities (Sprint 2)
- ✅ Core architecture with modular crate structure (Sprint 1)
- ✅ Compositor test demo already scaffolded

## Deliverables

### 1. Rendering Orchestrator (`game_rendering` crate)

- [x] Create `game_rendering` crate structure with proper Cargo.toml
- [x] Define render layer enum with ordering and blend modes
- [x] Implement render target management system
- [x] Create layer compositing pipeline using Bevy's render graph
- [x] Set up proper render scheduling and synchronization

### 2. Render Target System

- [x] Create offscreen render targets for each layer
- [x] Implement target resolution management (handle window resizing)
- [x] Set up proper texture formats and sampling
- [x] Create resource management for GPU textures
- [x] Implement target clearing and preparation per frame

### 3. Layer Integration

- [x] **Background Layer (0)**: Simple gradient or solid color renderer
- [x] **Game World Layer (1)**: Basic sprite/mesh rendering setup
- [x] **Metaball Layer (2)**: Integrate existing metaball_renderer with offscreen target
- [x] **Effects Layer (3)**: Particle system foundation (even if empty initially)
- [ ] **UI Layer (4)**: Text and shape rendering setup

### 4. Camera System Enhancement

- [ ] Build upon existing coordinate mapping from Sprint 2.1
- [ ] Implement camera shake system with decay
- [ ] Add zoom controls with proper bounds
- [ ] Create viewport management for fixed aspect ratio
- [ ] Implement letterboxing/pillarboxing for different screens

### 5. Compositor Implementation

- [x] Create compositor shader in WGSL
- [x] Implement blend mode support per layer
- [x] Add debug visualization for layer boundaries
- [x] Create final presentation pass to window
- [ ] Implement proper color space handling

### 6. Demo: Compositor Test Enhancement

- [x] Update existing `compositor_test` demo to showcase the pipeline
- [x] Integrate physics demo elements from Sprint 2
- [x] Add visual elements to each layer for testing
- [x] Implement layer toggle system (keys 1-5)
- [ ] Add performance overlay showing render times
- [ ] Create interactive camera controls for testing
- [x] Add blend mode switching (keys Q/W/E for different modes)
- [x] Include metaball rendering on appropriate layer

## Technical Specifications

### File Structure Update

```
crates/
├── game_rendering/           # NEW
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── layers.rs        # Layer definitions and management
│       ├── targets.rs       # Render target resources
│       ├── compositor.rs    # Layer compositing system
│       ├── camera.rs        # Enhanced camera with shake/zoom
│       └── plugin.rs        # RenderingPlugin
├── metaball_renderer/        # EXISTING - integrate with new targets
└── game/                     # UPDATE - use new rendering pipeline

demos/
└── compositor_test/          # EXISTING - enhance for Sprint 3
    ├── Cargo.toml           # Update dependencies
    └── src/
        └── main.rs          # Implement full pipeline showcase
```

### Core Types

```rust
// layers.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum RenderLayer {
    Background = 0,
    GameWorld = 1,
    Metaballs = 2,
    Effects = 3,
    UI = 4,
}

// targets.rs
#[derive(Resource)]
pub struct RenderTargets {
    pub layers: HashMap<RenderLayer, Handle<Image>>,
    pub final_composite: Handle<Image>,
    pub resolution: UVec2,
}

// camera.rs
#[derive(Component)]
pub struct GameCamera {
    pub base_resolution: Vec2,
    pub viewport_scale: f32,
    pub shake_intensity: f32,
    pub shake_decay_rate: f32,
    pub shake_offset: Vec2,
}
```

### Compositor Test Demo Features

```rust
// Key bindings for compositor_test
// 1-5: Toggle individual layers
// Q/W/E: Switch blend modes (Normal/Additive/Multiply)
// Space: Trigger camera shake
// +/-: Zoom in/out
// F1: Toggle performance overlay
// F2: Toggle layer boundaries debug
// R: Reset all settings
```

## Acceptance Criteria

### Functional Requirements

- [ ] All 5 render layers functioning independently
-- [x] Metaball renderer outputs to correct layer target
- [ ] Camera shake and zoom work without affecting metaball coordinates
-- [x] Layer compositing produces clean final image
- [ ] No visual artifacts or layer bleeding
- [ ] `compositor_test` demo showcases all features

### Performance Requirements

- [ ] Maintain 60 FPS with all layers active
- [ ] Total frame time < 16ms
- [ ] Memory usage < 150MB (GPU)
- [ ] No frame drops during camera shake

### Integration Requirements

- [ ] Physics demo from Sprint 2 works unchanged
- [ ] Coordinate conversion utilities remain functional
- [ ] All existing demos compile and run
- [x] `compositor_test` integrates physics and metaballs

## Testing Strategy

### Unit Tests

- [ ] Layer ordering tests
- [ ] Render target creation/destruction
- [ ] Camera transformation math
- [ ] Coordinate mapping validation

### Integration Tests

- [ ] Multi-layer rendering test
- [ ] Window resize handling
- [ ] Performance benchmarks
- [ ] Memory leak detection

### Visual Tests (via compositor_test)

- [ ] Layer toggle verification
- [ ] Blend mode validation
- [ ] Camera shake smoothness
- [ ] Aspect ratio maintenance
- [ ] Cross-layer visual consistency

## Risk Mitigation Updates

| Risk | Impact | Mitigation | Status |
|------|--------|------------|---------|
| Coordinate system confusion | Medium | Use existing utilities from Sprint 2.1 | ✅ Addressed |
| Performance overhead | High | Profile each layer independently | 🔄 Monitor |
| Integration complexity | High | Test with existing demos frequently | 🔄 Ongoing |
| Shader compatibility | Medium | Test on multiple GPU vendors | 📋 Planned |

## Dependencies

### From Completed Sprints

- `metaball_renderer` with decoupled camera (Sprint 2.1)
- `game_physics` with ball entities (Sprint 2)
- `game_core` with base components (Sprint 1)
- Coordinate conversion utilities (Sprint 2.1)

### New Requirements

- WGSL shader compilation
- Bevy render graph API
- Image asset handling

## Definition of Done

- [x] `game_rendering` crate created and compiles
- [ ] All 5 layers render to separate targets
- [x] Compositor combines layers correctly
- [ ] `compositor_test` demo showcases all rendering features
- [ ] Performance targets met (60 FPS)
- [ ] Existing demos still functional
- [ ] Documentation updated with architecture diagrams
- [ ] No regression in metaball rendering quality

## Migration Notes

### For Existing Code

- Physics demo: Add `RenderingPlugin` to app
- Metaball demo: Can remain as-is for comparison
- Compositor test: Becomes the primary showcase
- Coordinate conversions: Use enhanced camera system

### For Future Sprints

- Sprint 4: Widget renderer will use GameWorld layer
- Sprint 8: Background effects on Background layer
- Sprint 9: UI/HUD on UI layer
- Sprint 10: Particles on Effects layer

## Notes for Next Sprint

Sprint 4 will implement the widget renderer for game world elements:

- Create visual representations for walls, targets, hazards
- Establish the game's visual style
- Build upon the GameWorld render layer
- Add glow and animation effects

The rendering pipeline established here will be critical for all future visual work, so getting the architecture right is essential. The `compositor_test` demo will serve as our reference implementation for the complete pipeline.
