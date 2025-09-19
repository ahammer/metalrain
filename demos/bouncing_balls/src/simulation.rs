use bevy::prelude::*;
use rand::prelude::*;
use metaball_renderer::{MetaBall, MetaBallColor, MetaBallCluster};

const HALF_EXTENT: f32 = 200.0;
const WORLD_SIZE: f32 = HALF_EXTENT * 2.0;
const MAX_BALLS: usize = 512;

#[derive(Component, Clone, Copy)]
struct Velocity(Vec2);

pub struct BouncySimulationPlugin;
impl Plugin for BouncySimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_balls)
           .add_systems(Update, (update_balls,));
    }
}

fn spawn_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    let desired = MAX_BALLS.min(400); // spawn subset for clarity
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
            MetaBall { center: world_to_tex(Vec2::new(x,y)), radius },
            MetaBallColor(color_palette[cluster as usize]),
            MetaBallCluster(cluster),
            Velocity(vel),
        ));
    }
}

fn update_balls(time: Res<Time>, mut q: Query<(&mut MetaBall, &mut Velocity)>) {
    let dt = time.delta_secs(); if dt <= 0.0 { return; }
    for (mut mb, mut vel) in q.iter_mut() {
        // Convert tex space back to world for physics
        let mut pos = tex_to_world(Vec2::new(mb.center.x, mb.center.y));
        pos += vel.0 * dt;
        // Bounds
        let min = -HALF_EXTENT + mb.radius;
        let max = HALF_EXTENT - mb.radius;
        if pos.x < min { pos.x = min; vel.0.x = -vel.0.x * 0.92; }
        else if pos.x > max { pos.x = max; vel.0.x = -vel.0.x * 0.92; }
        if pos.y < min { pos.y = min; vel.0.y = -vel.0.y * 0.92; }
        else if pos.y > max { pos.y = max; vel.0.y = -vel.0.y * 0.92; }
        mb.center = world_to_tex(pos);
    }
}

// Texture space is 0..(texture_size) but we don't have direct access here; assume 1024 (default setting).
const TEX_SIZE: f32 = 1024.0;
fn world_to_tex(p: Vec2) -> Vec2 { Vec2::new(((p.x + HALF_EXTENT)/WORLD_SIZE)*TEX_SIZE, ((p.y + HALF_EXTENT)/WORLD_SIZE)*TEX_SIZE) }
fn tex_to_world(p: Vec2) -> Vec2 { Vec2::new((p.x / TEX_SIZE)*WORLD_SIZE - HALF_EXTENT, (p.y / TEX_SIZE)*WORLD_SIZE - HALF_EXTENT) }
