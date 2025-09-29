# Sub-Sprint: Background Renderer Implementation

## Goal

Implement a clean background rendering system that integrates with the existing layer-based compositor, providing visual depth through gradient backgrounds with optional animation.

## Current State Analysis

- ✅ Layer system exists (`game_rendering` crate with Background layer)
- ✅ Compositor infrastructure ready
- ✅ Metaball background cycling exists (`src/rendering/metaballs/systems.rs`)
- ⚠️ Background module stub exists (`src/rendering/background/mod.rs`) but is empty
- ❌ No actual background rendering implementation

## Simplified Scope (This Sprint)

### Core Implementation

1. **Single unified background system** (not a separate crate initially)
   - Extend existing `src/rendering/background/` module
   - Leverage existing rendering infrastructure

2. **Three practical modes**:
   - `Solid`: Single color fill
   - `LinearGradient`: Two-color gradient (vertical/horizontal/diagonal via angle)
   - `Animated`: Time-based color cycling with smooth interpolation

3. **Integration approach**:
   - Material2d-based implementation
   - Full-screen quad renderer
   - Proper RenderLayers integration

## Implementation Plan

### Phase 1: Core Structure (2 hours)

```rust
// src/rendering/background/mod.rs
pub mod material;
pub mod systems;
pub mod plugin;

pub use material::{BackgroundMaterial, BackgroundMode};
pub use plugin::BackgroundPlugin;

// Resource for runtime configuration
#[derive(Resource, Debug, Clone)]
pub struct BackgroundConfig {
    pub mode: BackgroundMode,
    pub primary_color: LinearRgba,
    pub secondary_color: LinearRgba,
    pub angle: f32,           // For gradient direction (radians)
    pub animation_speed: f32, // For animated mode
}
```

### Phase 2: Material & Shader (3 hours)

```rust
// src/rendering/background/material.rs
#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
#[repr(C)]
pub struct BackgroundMaterial {
    #[uniform(0)]
    pub mode: u32,
    #[uniform(0)]
    pub color_a: Vec4,
    #[uniform(0)]
    pub color_b: Vec4,
    #[uniform(0)]
    pub params: Vec4, // x: angle, y: time, z: animation_speed, w: reserved
}
```

```wgsl
// assets/shaders/background.wgsl
struct BackgroundUniforms {
    mode: u32,
    _pad: vec3<u32>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    params: vec4<f32>, // angle, time, speed, reserved
}

@group(1) @binding(0)
var<uniform> uniforms: BackgroundUniforms;

@fragment
fn fragment(
    @location(0) uv: vec2<f32>,
) -> @location(0) vec4<f32> {
    if uniforms.mode == 0u { // Solid
        return uniforms.color_a;
    } else if uniforms.mode == 1u { // Linear Gradient
        let angle = uniforms.params.x;
        let rotated = rotate_uv(uv - 0.5, angle) + 0.5;
        let t = rotated.y;
        return mix(uniforms.color_a, uniforms.color_b, t);
    } else { // Animated
        let time = uniforms.params.y;
        let speed = uniforms.params.z;
        let t = (sin(time * speed) + 1.0) * 0.5;
        return mix(uniforms.color_a, uniforms.color_b, t);
    }
}
```

### Phase 3: System Integration (2 hours)

```rust
// src/rendering/background/systems.rs
pub fn setup_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    config: Res<BackgroundConfig>,
) {
    // Spawn full-screen quad
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(2.0, 2.0))),
        MeshMaterial2d(materials.add(BackgroundMaterial::from_config(&config))),
        Transform::from_scale(Vec3::splat(1000.0)), // Ensure full coverage
        RenderLayers::layer(0), // Background layer
        BackgroundEntity,
    ));
}

pub fn update_background(
    time: Res<Time>,
    config: Res<BackgroundConfig>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    query: Query<&MeshMaterial2d<BackgroundMaterial>, With<BackgroundEntity>>,
) {
    if !config.is_changed() && config.mode != BackgroundMode::Animated {
        return;
    }
    
    // Update material uniforms
    if let Ok(handle) = query.get_single() {
        if let Some(mat) = materials.get_mut(handle) {
            mat.update_from_config(&config, time.elapsed_secs());
        }
    }
}
```

### Phase 4: Demo Integration (1 hour)

```rust
// Add to compositor_test or physics_playground
app.add_plugins(BackgroundPlugin)
   .insert_resource(BackgroundConfig {
       mode: BackgroundMode::LinearGradient,
       primary_color: LinearRgba::rgb(0.1, 0.05, 0.15),   // Deep purple
       secondary_color: LinearRgba::rgb(0.02, 0.02, 0.05), // Near black
       angle: std::f32::consts::PI * 0.25, // 45 degree angle
       animation_speed: 1.0,
   });

// Optional: Key binding to cycle modes
fn cycle_background_mode(
    mut config: ResMut<BackgroundConfig>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        config.mode = config.mode.next();
        info!("Background mode: {:?}", config.mode);
    }
}
```

## Testing Checklist

- [ ] Background renders behind all other content
- [ ] Mode switching works without flicker
- [ ] Animation runs smoothly at 60 FPS
- [ ] Window resize maintains correct coverage
- [ ] No z-fighting with game content
- [ ] Memory usage stable over time
- [ ] Integration with existing metaball background toggle

## Success Metrics

- Background adds <0.1ms to frame time
- Zero allocation per frame after setup
- Clean integration with existing layer system
- No visual artifacts or tearing

## Deferred for Later

- Radial/circular gradients
- Multi-stop gradients
- Noise/procedural textures
- Parallax layers
- Image backgrounds
- Day/night cycles

## Next Steps After Completion

1. Add configuration UI in Sprint 9
2. Integrate with level themes in Sprint 6
3. Add particle overlay effects in Sprint 10

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| Conflicts with existing rendering | Use established RenderLayers system |
| Performance impact | Keep shader simple, single draw call |
| Integration complexity | Build on existing infrastructure, don't recreate |

## Estimated Time: 8 hours total

- Phase 1: 2 hours (structure)
- Phase 2: 3 hours (shader + material)
- Phase 3: 2 hours (systems)
- Phase 4: 1 hour (demo integration)
