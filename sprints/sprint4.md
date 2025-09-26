# Sprint 4: Game World Elements (Walls, Targets, Hazards)

## Sprint Goal
Create the visual representation of game world elements including walls, targets, and hazards. Establish the widget renderer crate and implement collision boundaries with appealing visual feedback.

## Deliverables

### 1. Widget Renderer Crate (`widget_renderer`)
- [ ] Create `widget_renderer` crate structure
- [ ] Set up 2D shape rendering pipeline
- [ ] Implement line/edge rendering with glow effects
- [ ] Add sprite and mesh support for targets
- [ ] Create animated shader effects

### 2. Wall Rendering System
- [ ] Line segment-based wall definition
- [ ] Subtle glow effect on edges
- [ ] Corner highlighting
- [ ] Collision boundary visualization (debug)
- [ ] Support for curved walls (bezier)

### 3. Target Rendering
- [ ] Fragile appearance with glass/crystal aesthetic
- [ ] Hit state animations (crack, shatter)
- [ ] Color-coded targets (future compatibility)
- [ ] Pulsing idle animation
- [ ] Health indicator (multi-hit targets)

### 4. Hazard Zone Rendering
- [ ] Warning edge visualization
- [ ] Animated danger patterns
- [ ] Semi-transparent fill
- [ ] Particle effects at boundaries
- [ ] Different hazard types (pit, void, electric)

### 5. Demo: Game World Showcase
- [ ] Arena with all element types
- [ ] Interactive target destruction
- [ ] Ball interactions with elements
- [ ] Visual style consistency check
- [ ] Performance with complex levels

## Technical Specifications

### Widget Components
```rust
// Wall Component
#[derive(Component)]
pub struct Wall {
    pub segments: Vec<LineSegment>,
    pub thickness: f32,
    pub glow_intensity: f32,
    pub color: Color,
}

// Target Component  
#[derive(Component)]
pub struct Target {
    pub health: u8,
    pub max_health: u8,
    pub size: Vec2,
    pub color: GameColor,
    pub hit_animation: f32, // 0.0 to 1.0
}

// Hazard Component
#[derive(Component)]
pub struct Hazard {
    pub zone_type: HazardType,
    pub bounds: Rect,
    pub edge_width: f32,
    pub animation_speed: f32,
}

#[derive(Clone, Copy)]
pub enum HazardType {
    Pit,        // Instant ball removal
    Void,       // Gradual pull before removal
    Electric,   // Sparking edges
}
```

### Wall Rendering Shader
```wgsl
// wall_glow.wgsl
struct WallVertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) distance: f32, // Distance from line center
}

@fragment
fn fs_main(in: WallVertex) -> @location(0) vec4<f32> {
    let base_color = vec4<f32>(0.8, 0.8, 1.0, 1.0);
    let glow_color = vec4<f32>(0.4, 0.6, 1.0, 1.0);
    
    // Glow falloff from edge
    let glow = exp(-in.distance * 2.0);
    
    // Combine base and glow
    let color = mix(base_color, glow_color, glow);
    return vec4<f32>(color.rgb, color.a * (0.5 + glow * 0.5));
}
```

### Target Animation States
```rust
pub enum TargetState {
    Idle,
    Hit { progress: f32 },
    Destroying { progress: f32 },
    Destroyed,
}

impl Target {
    pub fn update_animation(&mut self, delta: f32) {
        match self.state {
            TargetState::Idle => {
                // Gentle pulse
                self.pulse_phase += delta * 2.0;
                self.scale = 1.0 + (self.pulse_phase.sin() * 0.05);
            }
            TargetState::Hit { ref mut progress } => {
                *progress += delta * 3.0;
                if *progress >= 1.0 {
                    self.state = TargetState::Idle;
                }
            }
            TargetState::Destroying { ref mut progress } => {
                *progress += delta * 2.0;
                self.scale = 1.0 - (*progress * 0.5);
                self.rotation += delta * 10.0;
                if *progress >= 1.0 {
                    self.state = TargetState::Destroyed;
                }
            }
            _ => {}
        }
    }
}
```

### Hazard Rendering Effects
```rust
pub fn render_hazard(
    hazards: Query<(&Hazard, &Transform)>,
    time: Res<Time>,
    mut gizmos: Gizmos,
) {
    for (hazard, transform) in hazards.iter() {
        let t = time.elapsed_seconds();
        
        match hazard.zone_type {
            HazardType::Pit => {
                // Pulsing red edge
                let intensity = (t * 2.0).sin() * 0.5 + 0.5;
                let color = Color::rgb(1.0, intensity * 0.2, 0.0);
                draw_hazard_edge(&mut gizmos, hazard.bounds, color, 3.0);
            }
            HazardType::Void => {
                // Spiraling void effect
                let rotation = t * 0.5;
                draw_void_spiral(&mut gizmos, hazard.bounds.center(), rotation);
            }
            HazardType::Electric => {
                // Random lightning arcs
                if rand::random::<f32>() < 0.1 {
                    draw_lightning_arc(&mut gizmos, hazard.bounds);
                }
            }
        }
    }
}
```

### Visual Style Configuration
```rust
pub struct WidgetStyle {
    // Walls
    pub wall_base_color: Color,
    pub wall_glow_color: Color,
    pub wall_thickness: f32,
    pub wall_glow_radius: f32,
    
    // Targets
    pub target_idle_color: Color,
    pub target_hit_flash: Color,
    pub target_pulse_rate: f32,
    pub target_shatter_particles: u32,
    
    // Hazards
    pub hazard_edge_intensity: f32,
    pub hazard_warning_color: Color,
    pub hazard_animation_speed: f32,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            wall_base_color: Color::rgb(0.7, 0.7, 0.8),
            wall_glow_color: Color::rgb(0.4, 0.6, 1.0),
            wall_thickness: 4.0,
            wall_glow_radius: 8.0,
            
            target_idle_color: Color::rgb(0.2, 0.8, 0.9),
            target_hit_flash: Color::WHITE,
            target_pulse_rate: 2.0,
            target_shatter_particles: 12,
            
            hazard_edge_intensity: 0.8,
            hazard_warning_color: Color::rgb(1.0, 0.2, 0.1),
            hazard_animation_speed: 1.0,
        }
    }
}
```

## Demo Features

### Test Arena Layout
```
+------------------+
|  T    W    T     |  T = Target
|       W          |  W = Wall segment
|  T    W    T     |  H = Hazard zone
|       W          |  
| HHHHHHHHHHHHHH   |
+------------------+
```

### Interactive Elements
- Click targets to simulate hits
- Drag to create temporary walls
- Press H to toggle hazard visibility
- Space to spawn test balls
- R to reset all elements

## Performance Requirements

- Render 100+ wall segments at 60 FPS
- 50 targets with animations
- Particle effects < 2ms per frame
- Total widget rendering < 5ms

## Success Criteria

- ✅ Walls render with appealing glow effect
- ✅ Targets animate smoothly when hit
- ✅ Hazards clearly communicate danger
- ✅ Visual cohesion with metaball aesthetic
- ✅ Performance targets met
- ✅ All elements visible in game world layer

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Visual style mismatch | High | Consistent color palette, review |
| Performance with many walls | Medium | Batching, instanced rendering |
| Particle system overhead | Medium | Object pooling, LOD system |
| Shader compatibility | High | Fallback shaders for older GPUs |

## Dependencies

### From Previous Sprints
- Sprint 1: Core components (Wall, Target, Hazard)
- Sprint 2: Physics for collision detection
- Sprint 3: Render layer system (Layer 1)

### External Crates
- `bevy_prototype_lyon` (for 2D shapes)
- `rand` (for effects randomization)

### Assets
- Shader files for effects
- Optional: Target sprite textures
- Particle textures

## Definition of Done

- [ ] Widget renderer crate compiles
- [ ] All three element types render correctly
- [ ] Visual effects implemented and tunable
- [ ] Demo showcases all features
- [ ] Performance requirements met
- [ ] Integration with render pipeline complete
- [ ] Visual style guide documented
- [ ] README.md with usage examples

## Notes for Next Sprint

Sprint 5 will implement core gameplay:
- Win/lose condition checking
- Ball elimination by hazards
- Target destruction logic
- Game state management
- Round restart functionality

The visual elements from this sprint will provide the interactive components needed for actual gameplay mechanics.
