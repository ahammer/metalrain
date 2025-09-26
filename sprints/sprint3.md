# Sprint 3: Rendering Pipeline Architecture

## Sprint Goal
Establish the multi-layer rendering architecture that coordinates all visual subsystems. Set up render targets, layer compositing, and camera management while integrating the existing metaball renderer into the new pipeline.

## Deliverables

### 1. Rendering Orchestrator (`game_rendering`)
- [ ] Create `game_rendering` crate structure
- [ ] Define render layer enum and ordering
- [ ] Implement render target management
- [ ] Set up layer compositing pipeline
- [ ] Create camera controller system

### 2. Render Layer System
- [ ] Background layer setup (Layer 0)
- [ ] Game world layer setup (Layer 1)
- [ ] Metaball layer integration (Layer 2)
- [ ] Effects layer preparation (Layer 3)
- [ ] UI overlay layer setup (Layer 4)

### 3. Camera Management
- [ ] Viewport configuration (fixed aspect ratio)
- [ ] Camera shake system (for impacts)
- [ ] Zoom controls (debug mode)
- [ ] Screen-to-world coordinate mapping
- [ ] Letterboxing for different screen sizes

### 4. Demo: Rendering Test
- [ ] All five render layers active simultaneously
- [ ] Layer toggle controls (show/hide each)
- [ ] Blend mode experiments
- [ ] Performance profiling display
- [ ] Visual layer separation demo

## Technical Specifications

### Render Layers
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderLayer {
    Background = 0,  // Environment, gradients
    GameWorld = 1,   // Walls, targets, hazards
    Metaballs = 2,   // Ball entities with GPU blending
    Effects = 3,     // Particles, explosions
    UI = 4,          // HUD, menus, overlays
}

impl RenderLayer {
    pub fn render_order(&self) -> i32 {
        *self as i32
    }
    
    pub fn blend_mode(&self) -> BlendMode {
        match self {
            Self::Metaballs => BlendMode::Additive,
            Self::Effects => BlendMode::Alpha,
            _ => BlendMode::Default,
        }
    }
}
```

### Render Pipeline Structure
```rust
pub struct RenderPipeline {
    pub targets: HashMap<RenderLayer, Handle<Image>>,
    pub final_target: Handle<Image>,
    pub compositor: CompositorPipeline,
}

pub struct CompositorPipeline {
    pub shader: Handle<Shader>,
    pub bind_groups: Vec<BindGroup>,
    pub pipeline: RenderPipeline,
}
```

### Camera Configuration
```rust
pub struct GameCamera {
    pub base_resolution: Vec2,      // 1280x720
    pub viewport_scale: f32,        // For zoom
    pub shake_intensity: f32,       // 0.0 - 1.0
    pub shake_decay: f32,           // Per second
    pub bounds: Option<Rect>,       // Arena limits
}

pub fn setup_camera(
    mut commands: Commands,
) {
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: 1280.0,
                    height: 720.0,
                },
                ..default()
            },
            ..default()
        },
        GameCamera::default(),
    ));
}
```

### Layer Compositing Shader
```wgsl
// compositor.wgsl
@group(0) @binding(0) var background_texture: texture_2d<f32>;
@group(0) @binding(1) var gameworld_texture: texture_2d<f32>;
@group(0) @binding(2) var metaball_texture: texture_2d<f32>;
@group(0) @binding(3) var effects_texture: texture_2d<f32>;
@group(0) @binding(4) var ui_texture: texture_2d<f32>;
@group(0) @binding(5) var sampler: sampler;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let background = textureSample(background_texture, sampler, uv);
    let gameworld = textureSample(gameworld_texture, sampler, uv);
    let metaballs = textureSample(metaball_texture, sampler, uv);
    let effects = textureSample(effects_texture, sampler, uv);
    let ui = textureSample(ui_texture, sampler, uv);
    
    // Layer compositing (back to front)
    var color = background;
    color = mix(color, gameworld, gameworld.a);
    color = color + metaballs * metaballs.a;  // Additive for glow
    color = mix(color, effects, effects.a);
    color = mix(color, ui, ui.a);
    
    return color;
}
```

### Integration with Metaball Renderer
```rust
// Modify existing metaball_renderer to render to texture
pub fn setup_metaball_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_targets: Res<RenderTargets>,
) {
    let target = render_targets.layers[&RenderLayer::Metaballs];
    
    // Configure metaball renderer to use this target
    commands.insert_resource(MetaballRenderTarget {
        texture: target,
    });
}
```

## Rendering Test Demo Features

### Visual Tests
- Gradient background animation
- Placeholder geometry for game world layer
- Active metaballs from Sprint 2
- Particle system on effects layer
- Debug UI overlay

### Performance Metrics
- FPS counter
- Frame time breakdown by layer
- GPU memory usage
- Draw call count
- Texture memory per layer

### Interactive Controls
- **1-5 Keys**: Toggle layers on/off
- **Tab**: Show layer boundaries
- **B**: Cycle blend modes
- **P**: Show performance overlay
- **F11**: Fullscreen toggle

## Performance Requirements

- Total render time: < 16ms (60 FPS)
- Per-layer render: < 3ms
- Compositing pass: < 2ms
- Memory usage: < 100MB GPU

## Success Criteria

- ✅ All five layers render correctly
- ✅ Metaball renderer integrated seamlessly
- ✅ No visual artifacts between layers
- ✅ 60 FPS with all layers active
- ✅ Camera shake works smoothly
- ✅ Proper aspect ratio maintenance

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance overhead | High | Profile early, optimize compositing |
| Layer bleeding | Medium | Careful blend mode selection |
| Memory usage | Medium | Reuse render targets where possible |
| Shader compatibility | High | Test on multiple GPUs |

## Dependencies

### From Previous Sprints
- Sprint 1: Core architecture
- Sprint 2: Ball entities for metaball rendering
- Existing `metaball_renderer` crate

### External Crates
- Already included in `bevy`

### Assets
- Compositor shader files
- Test textures for each layer
- Debug fonts

## Definition of Done

- [ ] Rendering test demo shows all layers
- [ ] Each layer can be toggled independently
- [ ] Metaball renderer integrated into pipeline
- [ ] Camera system fully functional
- [ ] Performance targets met
- [ ] No visual artifacts
- [ ] Compositor shader documented
- [ ] README.md explains rendering architecture

## Notes for Next Sprint

Sprint 4 will implement game world elements:
- Create `widget_renderer` crate
- Implement wall rendering with glow
- Add target rendering with animations
- Create hazard zone visualization
- Establish visual style for game elements

The rendering pipeline from this sprint provides the foundation for all visual elements, ensuring proper layering and performance as we add more complex visuals.
