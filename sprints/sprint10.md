# Sprint 10: Visual Effects & Polish

## Sprint Goal
Add particle effects, screen shake, trail rendering, and other visual polish elements that provide satisfying feedback and enhance the game's visual impact.

## Deliverables

### 1. Effects System Crate (`game_effects`)
- [ ] Create `game_effects` crate structure
- [ ] Implement particle system foundation
- [ ] Add screen shake controller
- [ ] Create trail rendering system
- [ ] Build effect pooling for performance

### 2. Impact Effects
- [ ] Ball-wall collision particles
- [ ] Target hit flash and particles
- [ ] Target destruction explosion
- [ ] Hazard elimination effect
- [ ] Ball spawn animation

### 3. Trail Effects
- [ ] Ball motion trails
- [ ] Fading trail segments
- [ ] Color-coded trails per ball
- [ ] Trail intensity based on speed
- [ ] Performance-optimized rendering

### 4. Screen Effects
- [ ] Camera shake on impacts
- [ ] Intensity based on collision force
- [ ] Decay over time
- [ ] Directional shake support
- [ ] User preference respect

### 5. Demo: Effects Showcase
- [ ] All effects triggered on demand
- [ ] Performance monitoring
- [ ] Effect intensity controls
- [ ] Visual comparison (on/off)
- [ ] Stress test mode

## Technical Specifications

### Particle System
```rust
#[derive(Component)]
pub struct ParticleEmitter {
    pub emission_rate: f32,
    pub lifetime: Range<f32>,
    pub initial_velocity: Range<Vec2>,
    pub acceleration: Vec2,
    pub size: Range<f32>,
    pub color: Gradient,
    pub texture: Handle<Image>,
    pub blend_mode: BlendMode,
    pub max_particles: usize,
}

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub size: f32,
    pub color: Color,
    pub rotation: f32,
    pub angular_velocity: f32,
}

pub fn spawn_impact_particles(
    commands: &mut Commands,
    position: Vec3,
    normal: Vec2,
    impact_force: f32,
) {
    let particle_count = (impact_force * 10.0).min(30.0) as usize;
    
    commands.spawn(ParticleEmitterBundle {
        emitter: ParticleEmitter {
            emission_rate: 0.0, // Burst emission
            lifetime: 0.3..0.8,
            initial_velocity: Range {
                min: normal.rotate(Vec2::from_angle(-PI / 4.0)) * 100.0,
                max: normal.rotate(Vec2::from_angle(PI / 4.0)) * 300.0,
            },
            acceleration: Vec2::new(0.0, -500.0), // Gravity
            size: 2.0..6.0,
            color: Gradient::from_colors(vec![
                Color::WHITE,
                Color::rgb(0.8, 0.9, 1.0),
                Color::rgba(0.5, 0.7, 1.0, 0.0),
            ]),
            ..default()
        },
        transform: Transform::from_translation(position),
        burst: ParticleBurst {
            count: particle_count,
            immediate: true,
        },
    });
}
```

### Trail Rendering
```rust
#[derive(Component)]
pub struct TrailRenderer {
    pub points: VecDeque<TrailPoint>,
    pub max_points: usize,
    pub width: f32,
    pub color: Color,
    pub fade_time: f32,
}

#[derive(Clone)]
pub struct TrailPoint {
    pub position: Vec2,
    pub timestamp: f32,
    pub velocity: Vec2,
}

pub fn update_ball_trails(
    mut trails: Query<(&mut TrailRenderer, &Transform, &Velocity), With<Ball>>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_seconds();
    
    for (mut trail, transform, velocity) in trails.iter_mut() {
        // Add new point if moved enough
        let position = transform.translation.xy();
        
        if let Some(last_point) = trail.points.back() {
            let distance = position.distance(last_point.position);
            if distance > 5.0 { // Minimum distance between points
                trail.points.push_back(TrailPoint {
                    position,
                    timestamp: current_time,
                    velocity: velocity.linvel,
                });
            }
        } else {
            trail.points.push_back(TrailPoint {
                position,
                timestamp: current_time,
                velocity: velocity.linvel,
            });
        }
        
        // Remove old points
        while trail.points.len() > trail.max_points {
            trail.points.pop_front();
        }
        
        // Remove expired points
        trail.points.retain(|point| {
            current_time - point.timestamp < trail.fade_time
        });
    }
}

pub fn render_trails(
    trails: Query<&TrailRenderer>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    let current_time = time.elapsed_seconds();
    
    for trail in trails.iter() {
        if trail.points.len() < 2 {
            continue;
        }
        
        // Draw trail segments with fading
        for window in trail.points.windows(2) {
            let start = &window[0];
            let end = &window[1];
            
            let age = current_time - start.timestamp;
            let alpha = (1.0 - age / trail.fade_time).max(0.0);
            let width = trail.width * alpha;
            
            let color = trail.color.with_alpha(trail.color.alpha() * alpha);
            
            gizmos.line_2d(
                start.position,
                end.position,
                color,
            );
            
            // Optional: Add glow effect
            for i in 1..=3 {
                let glow_width = width * (1.0 + i as f32 * 0.5);
                let glow_alpha = alpha * (0.3 / i as f32);
                let glow_color = color.with_alpha(glow_alpha);
                
                gizmos.line_2d(
                    start.position,
                    end.position,
                    glow_color,
                );
            }
        }
    }
}
```

### Screen Shake
```rust
#[derive(Resource)]
pub struct ScreenShake {
    pub intensity: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub frequency: f32,
    pub direction: Option<Vec2>,
    pub decay: ShakeDecay,
}

#[derive(Clone, Copy)]
pub enum ShakeDecay {
    Linear,
    Exponential,
    Elastic,
}

impl ScreenShake {
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        if intensity > self.intensity {
            self.intensity = intensity;
            self.duration = duration;
            self.elapsed = 0.0;
        }
    }
    
    pub fn trigger_directional(&mut self, intensity: f32, duration: f32, direction: Vec2) {
        self.trigger(intensity, duration);
        self.direction = Some(direction.normalize());
    }
}

pub fn apply_screen_shake(
    mut shake: ResMut<ScreenShake>,
    mut camera: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
    settings: Res<GameSettings>,
) {
    if !settings.screen_shake || shake.intensity <= 0.0 {
        return;
    }
    
    shake.elapsed += time.delta_seconds();
    
    if shake.elapsed >= shake.duration {
        shake.intensity = 0.0;
        if let Ok(mut transform) = camera.get_single_mut() {
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
        return;
    }
    
    let progress = shake.elapsed / shake.duration;
    let decay_multiplier = match shake.decay {
        ShakeDecay::Linear => 1.0 - progress,
        ShakeDecay::Exponential => (-5.0 * progress).exp(),
        ShakeDecay::Elastic => (1.0 - progress) * (progress * PI * 4.0).cos(),
    };
    
    let current_intensity = shake.intensity * decay_multiplier;
    
    if let Ok(mut transform) = camera.get_single_mut() {
        let offset = if let Some(dir) = shake.direction {
            // Directional shake
            let wave = (shake.elapsed * shake.frequency).sin();
            dir * wave * current_intensity
        } else {
            // Random shake
            let x = (shake.elapsed * shake.frequency).sin() * current_intensity;
            let y = (shake.elapsed * shake.frequency * 1.3).cos() * current_intensity;
            Vec2::new(x, y)
        };
        
        transform.translation.x = offset.x;
        transform.translation.y = offset.y;
    }
}
```

### Target Destruction Effect
```rust
pub fn spawn_target_destruction(
    commands: &mut Commands,
    position: Vec3,
    target_color: Color,
    target_size: Vec2,
) {
    // Glass shatter particles
    for _ in 0..12 {
        let angle = rand::random::<f32>() * TAU;
        let speed = rand::random::<f32>() * 200.0 + 100.0;
        let velocity = Vec2::from_angle(angle) * speed;
        
        commands.spawn(ShardBundle {
            shard: GlassShard {
                velocity,
                angular_velocity: rand::random::<f32>() * 10.0 - 5.0,
                lifetime: rand::random::<f32>() * 0.5 + 0.5,
                size: Vec2::new(
                    rand::random::<f32>() * 10.0 + 5.0,
                    rand::random::<f32>() * 5.0 + 2.0,
                ),
            },
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: target_color.with_alpha(0.8),
                    ..default()
                },
                transform: Transform::from_translation(position),
                ..default()
            },
        });
    }
    
    // Ring explosion
    commands.spawn(RingExplosionBundle {
        ring: RingExplosion {
            expansion_rate: 300.0,
            fade_rate: 2.0,
            max_radius: 100.0,
            thickness: 3.0,
            color: target_color,
        },
        transform: Transform::from_translation(position),
    });
    
    // Flash effect
    commands.spawn(FlashBundle {
        flash: ScreenFlash {
            color: Color::WHITE.with_alpha(0.3),
            duration: 0.1,
        },
    });
}
```

### Effect Configuration
```toml
[effects]
# Particles
particle_quality = "high" # low, medium, high
max_particles = 1000
particle_pooling = true

# Trails
trail_enabled = true
trail_length = 20
trail_fade_time = 0.5
trail_width = 3.0

# Screen shake
shake_enabled = true
shake_intensity_multiplier = 1.0
shake_frequency = 30.0

# Impact effects
wall_impact_particles = true
target_hit_particles = true
destruction_effects = true

[performance]
auto_adjust_effects = true
target_fps = 60
min_particle_density = 0.3
```

## Performance Considerations

### Object Pooling
```rust
pub struct ParticlePool {
    inactive: Vec<Entity>,
    active: HashSet<Entity>,
    max_size: usize,
}

impl ParticlePool {
    pub fn spawn(&mut self, commands: &mut Commands) -> Option<Entity> {
        if let Some(entity) = self.inactive.pop() {
            self.active.insert(entity);
            Some(entity)
        } else if self.active.len() < self.max_size {
            let entity = commands.spawn_empty().id();
            self.active.insert(entity);
            Some(entity)
        } else {
            None // Pool exhausted
        }
    }
    
    pub fn despawn(&mut self, entity: Entity) {
        if self.active.remove(&entity) {
            self.inactive.push(entity);
        }
    }
}
```

## Success Criteria

- ✅ All effects trigger appropriately
- ✅ Performance impact < 5ms per frame
- ✅ Effects enhance gameplay feel
- ✅ No visual artifacts or glitches
- ✅ Settings respected (can disable)
- ✅ Effects scale with impact force

## Definition of Done

- [ ] Effects system crate created
- [ ] All particle effects implemented
- [ ] Trail rendering working smoothly
- [ ] Screen shake feels satisfying
- [ ] Performance targets met
- [ ] Object pooling functional
- [ ] Settings integration complete
- [ ] README documents effect system

## Notes for Next Sprint

Sprint 11 will add audio:
- Sound effect integration
- Impact sounds
- Background music
- Audio mixing
- 3D positional audio basics

The visual effects will be complemented by audio for complete sensory feedback.
