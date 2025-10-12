# Sprint 8: Background & Environmental Effects

## Sprint Goal
Create atmospheric background rendering with dynamic gradients, parallax layers, ambient particles, and arena boundary visualization to enhance the game's visual depth and atmosphere.

## Deliverables

### 1. Background Renderer Crate (`background_renderer`)
- [ ] Create `background_renderer` crate structure
- [ ] Implement gradient background system
- [ ] Add parallax layer support
- [ ] Create ambient particle systems
- [ ] Build edge glow effects

### 2. Dynamic Gradient Backgrounds
- [ ] Animated color transitions
- [ ] Multiple gradient presets
- [ ] Time-based color shifts
- [ ] Reactive to game state
- [ ] Smooth blending between presets

### 3. Parallax Layer System
- [ ] Multiple depth layers
- [ ] Camera-responsive movement
- [ ] Floating geometric shapes
- [ ] Depth fog effect
- [ ] Performance-optimized rendering

### 4. Ambient Particle Effects
- [ ] Floating dust/stars
- [ ] Directional drift
- [ ] Depth-based sizing
- [ ] Color variation
- [ ] Density configuration

### 5. Arena Boundary Visualization
- [ ] Subtle edge glow
- [ ] Corner accents
- [ ] Pulsing effects
- [ ] Warning zones
- [ ] Viewport letterboxing

## Technical Specifications

### Background Configuration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    pub gradient: GradientConfig,
    pub parallax: ParallaxConfig,
    pub particles: ParticleConfig,
    pub edge_effects: EdgeConfig,
}

#[derive(Debug, Clone)]
pub struct GradientConfig {
    pub start_color: Color,
    pub end_color: Color,
    pub angle: f32,              // Gradient angle in radians
    pub animation_speed: f32,     // Color shift speed
    pub pulse_intensity: f32,    // 0.0 to 1.0
}

pub struct ParallaxConfig {
    pub layer_count: usize,
    pub base_speed: f32,
    pub depth_multiplier: f32,
    pub opacity_falloff: f32,
}
```

### Gradient Shader
```wgsl
// gradient_background.wgsl
struct BackgroundUniforms {
    start_color: vec4<f32>,
    end_color: vec4<f32>,
    time: f32,
    angle: f32,
    pulse: f32,
}

@group(0) @binding(0) var<uniform> uniforms: BackgroundUniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    // Rotate UV by angle
    let cos_a = cos(uniforms.angle);
    let sin_a = sin(uniforms.angle);
    let rotated = vec2<f32>(
        uv.x * cos_a - uv.y * sin_a,
        uv.x * sin_a + uv.y * cos_a
    );
    
    // Calculate gradient
    let t = rotated.y;
    
    // Add time-based animation
    let animated_t = t + sin(uniforms.time * 0.5) * 0.1;
    
    // Pulse effect
    let pulse = sin(uniforms.time * 2.0) * uniforms.pulse;
    
    // Blend colors
    var color = mix(uniforms.start_color, uniforms.end_color, animated_t);
    color = color + vec4<f32>(pulse, pulse, pulse, 0.0);
    
    return color;
}
```

### Parallax Layer System
```rust
#[derive(Component)]
pub struct ParallaxLayer {
    pub depth: f32,          // 0.0 (far) to 1.0 (near)
    pub movement_scale: f32,  // How much it moves with camera
    pub opacity: f32,
    pub shapes: Vec<ParallaxShape>,
}

#[derive(Clone)]
pub struct ParallaxShape {
    pub position: Vec2,
    pub size: f32,
    pub rotation: f32,
    pub shape_type: ShapeType,
}

pub enum ShapeType {
    Circle,
    Triangle,
    Hexagon,
    Cross,
}

pub fn update_parallax_layers(
    camera: Query<&Transform, With<Camera>>,
    mut layers: Query<(&ParallaxLayer, &mut Transform), Without<Camera>>,
) {
    let camera_transform = camera.single();
    let camera_pos = camera_transform.translation.xy();
    
    for (layer, mut transform) in layers.iter_mut() {
        // Move layer based on camera position and depth
        let offset = camera_pos * layer.movement_scale * (1.0 - layer.depth);
        transform.translation.x = -offset.x;
        transform.translation.y = -offset.y;
        
        // Set z-order based on depth
        transform.translation.z = -100.0 + layer.depth * 10.0;
    }
}
```

### Ambient Particle System
```rust
#[derive(Component)]
pub struct AmbientParticle {
    pub velocity: Vec2,
    pub size: f32,
    pub opacity: f32,
    pub depth: f32,
    pub lifetime: f32,
    pub color: Color,
}

pub fn spawn_ambient_particles(
    mut commands: Commands,
    config: Res<ParticleConfig>,
    time: Res<Time>,
    mut spawn_timer: Local<Timer>,
) {
    spawn_timer.tick(time.delta());
    
    if spawn_timer.just_finished() {
        let count = (config.density * 10.0) as usize;
        
        for _ in 0..count {
            let depth = rand::random::<f32>();
            let size = lerp(0.5, 3.0, 1.0 - depth);
            let opacity = lerp(0.1, 0.5, 1.0 - depth);
            
            commands.spawn(ParticleBundle {
                particle: AmbientParticle {
                    velocity: Vec2::new(
                        rand::random::<f32>() * 20.0 - 10.0,
                        rand::random::<f32>() * -30.0 - 10.0,
                    ),
                    size,
                    opacity,
                    depth,
                    lifetime: rand::random::<f32>() * 10.0 + 20.0,
                    color: config.particle_color,
                },
                sprite: SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(size)),
                        color: config.particle_color.with_alpha(opacity),
                        ..default()
                    },
                    ..default()
                },
            });
        }
        
        spawn_timer.reset();
    }
}

pub fn update_ambient_particles(
    mut particles: Query<(&mut Transform, &mut AmbientParticle, &mut Sprite)>,
    time: Res<Time>,
    arena_bounds: Res<ArenaBounds>,
) {
    for (mut transform, mut particle, mut sprite) in particles.iter_mut() {
        // Update position
        transform.translation += particle.velocity.extend(0.0) * time.delta_seconds();
        
        // Update lifetime
        particle.lifetime -= time.delta_seconds();
        
        // Fade out near end of life
        if particle.lifetime < 2.0 {
            sprite.color.set_alpha(particle.opacity * (particle.lifetime / 2.0));
        }
        
        // Wrap around arena bounds
        if transform.translation.x < arena_bounds.min.x {
            transform.translation.x = arena_bounds.max.x;
        }
        if transform.translation.x > arena_bounds.max.x {
            transform.translation.x = arena_bounds.min.x;
        }
    }
}
```

### Edge Effects
```rust
pub fn render_arena_edges(
    mut gizmos: Gizmos,
    arena_bounds: Res<ArenaBounds>,
    time: Res<Time>,
    config: Res<EdgeConfig>,
) {
    let t = time.elapsed_seconds();
    let pulse = (t * 2.0).sin() * 0.3 + 0.7;
    
    let color = config.edge_color.with_alpha(config.base_opacity * pulse);
    let glow_color = config.glow_color.with_alpha(config.glow_opacity * pulse);
    
    // Draw main boundary
    gizmos.rect_2d(
        arena_bounds.center(),
        arena_bounds.size(),
        color,
    );
    
    // Draw glow layers
    for i in 1..=config.glow_layers {
        let offset = i as f32 * config.glow_spread;
        let alpha = config.glow_opacity * (1.0 - (i as f32 / config.glow_layers as f32));
        
        gizmos.rect_2d(
            arena_bounds.center(),
            arena_bounds.size() + Vec2::splat(offset * 2.0),
            glow_color.with_alpha(alpha * pulse),
        );
    }
    
    // Corner accents
    for corner in arena_bounds.corners() {
        draw_corner_accent(&mut gizmos, corner, t, &config);
    }
}
```

### Background Presets
```rust
pub enum BackgroundPreset {
    GradientBlue,
    GradientPurple,
    GradientSunset,
    SpaceNebula,
    DeepOcean,
    CyberGrid,
}

impl BackgroundPreset {
    pub fn to_config(&self) -> BackgroundConfig {
        match self {
            Self::GradientBlue => BackgroundConfig {
                gradient: GradientConfig {
                    start_color: Color::rgb(0.05, 0.05, 0.2),
                    end_color: Color::rgb(0.02, 0.02, 0.05),
                    angle: 0.0,
                    animation_speed: 0.5,
                    pulse_intensity: 0.1,
                },
                particles: ParticleConfig {
                    density: 0.3,
                    particle_color: Color::rgb(0.6, 0.7, 1.0),
                    ..default()
                },
                ..default()
            },
            // Other presets...
        }
    }
}
```

## Performance Optimization

### Batching Strategy
- Render all parallax shapes in single draw call
- Particle instancing for ambient effects
- Gradient as single fullscreen quad
- Edge effects combined in single pass

### LOD System
```rust
pub fn adjust_particle_density(
    fps: Res<FrameRate>,
    mut config: ResMut<ParticleConfig>,
) {
    if fps.current < 55.0 {
        config.density *= 0.9;
    } else if fps.current > 65.0 && config.density < config.max_density {
        config.density *= 1.1;
    }
}
```

## Success Criteria

- ✅ Gradient backgrounds animate smoothly
- ✅ Parallax creates depth perception
- ✅ Particles add atmosphere without distraction
- ✅ Edge effects clearly define play area
- ✅ Performance impact < 2ms per frame
- ✅ All effects configurable via TOML

## Definition of Done

- [ ] Background renderer crate functional
- [ ] All visual effects implemented
- [ ] Performance targets met
- [ ] Configuration system works
- [ ] Presets look appealing
- [ ] Integration with render pipeline complete
- [ ] No visual artifacts
- [ ] README documents usage

## Notes for Next Sprint

Sprint 9 will add UI/HUD elements:
- Ball and target counters
- Win/lose overlays
- Pause menu
- Debug information display
- Settings menu basics

The atmospheric effects from this sprint will complement the UI elements for a complete visual experience.
