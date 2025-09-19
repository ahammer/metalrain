use bevy::prelude::*;
// Bouncy simulation demo
// Controls:
//   G - toggle gravity
//   C - toggle clustering pass (repacked into compute uniforms)
// Mirrors core behavioral parameters from original PoC while using the
// structured MetaballRendererPlugin packing + compute pipeline.
use rand::prelude::*;
use metaball_renderer::{MetaBall, MetaBallColor, MetaBallCluster, MetaballRenderSettings, consts::MAX_BALLS};

// World half extent for simulation (logical space: -EXTENT..EXTENT in both axes)
const HALF_EXTENT: f32 = 200.0;
const WORLD_SIZE: f32 = HALF_EXTENT * 2.0;

#[derive(Component, Clone, Copy)]
struct Velocity(Vec2);

#[derive(Resource, Clone)]
struct BouncyParams {
    gravity: Vec2,
    restitution: f32,
    enable_gravity: bool,
    speed_dampen: f32,
}
impl Default for BouncyParams { fn default() -> Self { Self { gravity: Vec2::new(0.0, -480.0), restitution: 0.92, enable_gravity: false, speed_dampen: 0.5 } } }

pub struct BouncySimulationPlugin;
impl Plugin for BouncySimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BouncyParams>()
            .add_systems(Startup, spawn_balls)
            .add_systems(Update, (
                update_balls,
                input_toggles,
            ));
    }
}

fn spawn_balls(mut commands: Commands, settings: Res<MetaballRenderSettings>) {
    let tex_w = settings.texture_size.x as f32; // square assumed but keep flexible
    let tex_h = settings.texture_size.y as f32;
    let mut rng = StdRng::from_entropy();
    let desired = MAX_BALLS; // spawn full capacity for parity with PoC
    for i in 0..desired {
        let radius = rng.gen_range(7.5..15.0);
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(10.0..40.0);
        let vel = Vec2::from_angle(angle) * speed;
        let color_palette = [
            LinearRgba::new(1.0,0.3,0.3,1.0),
            LinearRgba::new(0.3,1.0,0.3,1.0),
            LinearRgba::new(0.3,0.3,1.0,1.0),
            LinearRgba::new(1.0,1.0,0.3,1.0),
        ];
        let cluster = (i % color_palette.len()) as i32;
        commands.spawn((
            MetaBall { center: world_to_tex(Vec2::new(x,y), tex_w, tex_h), radius },
            MetaBallColor(color_palette[cluster as usize]),
            MetaBallCluster(cluster),
            Velocity(vel),
        ));
    }
    info!("Spawned {desired} metaballs");
}

fn update_balls(
    time: Res<Time>,
    params: Res<BouncyParams>,
    settings: Res<MetaballRenderSettings>,
    mut q: Query<(&mut MetaBall, &mut Velocity)>
) {
    let dt = time.delta_secs(); if dt <= 0.0 { return; }
    let tex_w = settings.texture_size.x as f32; let tex_h = settings.texture_size.y as f32;
    let grav = if params.enable_gravity { params.gravity * params.speed_dampen } else { Vec2::ZERO };
    for (mut mb, mut vel) in q.iter_mut() {
        // Convert tex space back to world for physics
        let mut pos = tex_to_world(Vec2::new(mb.center.x, mb.center.y), tex_w, tex_h);
        vel.0 += grav * dt;
        pos += vel.0 * dt;
        // Bounds
        let min = -HALF_EXTENT + mb.radius;
        let max = HALF_EXTENT - mb.radius;
        if pos.x < min { pos.x = min; vel.0.x = -vel.0.x * params.restitution; }
        else if pos.x > max { pos.x = max; vel.0.x = -vel.0.x * params.restitution; }
        if pos.y < min { pos.y = min; vel.0.y = -vel.0.y * params.restitution; }
        else if pos.y > max { pos.y = max; vel.0.y = -vel.0.y * params.restitution; }
        mb.center = world_to_tex(pos, tex_w, tex_h);
    }
}

fn input_toggles(
    keys: Res<ButtonInput<KeyCode>>,
    mut bouncy: ResMut<BouncyParams>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        bouncy.enable_gravity = !bouncy.enable_gravity;
        info!("Gravity {}", if bouncy.enable_gravity { "ON" } else { "OFF" });
    }
}

// Mapping helpers parameterized by texture dimensions.
fn world_to_tex(p: Vec2, tex_w: f32, tex_h: f32) -> Vec2 {
    Vec2::new(((p.x + HALF_EXTENT)/WORLD_SIZE)*tex_w, ((p.y + HALF_EXTENT)/WORLD_SIZE)*tex_h)
}
fn tex_to_world(p: Vec2, tex_w: f32, tex_h: f32) -> Vec2 {
    Vec2::new((p.x / tex_w)*WORLD_SIZE - HALF_EXTENT, (p.y / tex_h)*WORLD_SIZE - HALF_EXTENT)
}
