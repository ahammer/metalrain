use bevy::prelude::*;
use rand::prelude::*;

use crate::compute::types::{Ball as GpuBall, BallBuffer as GpuBallBuffer, ParamsUniform};

// World half extent for simulation (logical space: -EXTENT..EXTENT in both axes)
pub const HALF_EXTENT: f32 = 200.0; // requested 200 half extents (-200..200)
pub const WORLD_SIZE: f32 = HALF_EXTENT * 2.0;

pub const MAX_BOUNCY_BALLS: usize = 512; // reduced per request

// Sentinel: negative radius indicates end-of-list for GPU consumption (optional usage)
pub const SENTINEL_RADIUS: f32 = -1.0;

#[derive(Resource)]
pub struct BouncyParams {
    pub gravity: Vec2,
    pub restitution: f32,
    pub enable_gravity: bool,
    pub sync_to_gpu: bool,
    pub speed_dampen: f32,
}
impl Default for BouncyParams {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, -480.0),
            restitution: 0.92,
            enable_gravity: false,
            sync_to_gpu: true,
            speed_dampen: 0.5, // further slowdown per request
        }
    }
}

#[derive(Clone, Copy)]
pub struct BouncyBall {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
}

#[derive(Resource)]
pub struct BouncyBalls {
    pub balls: Vec<BouncyBall>,
    pub active: usize,
}

pub struct BouncyBallSimulationPlugin;
impl Plugin for BouncyBallSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BouncyParams>()
            .add_systems(Startup, spawn_bouncy_balls)
            .add_systems(Update, (update_bouncy_balls, sync_bouncy_into_gpu, bouncy_debug_input).chain());
    }
}

fn spawn_bouncy_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    // Preallocate full capacity once; we'll maintain active count separately.
    let mut balls = Vec::with_capacity(MAX_BOUNCY_BALLS);

    let desired = MAX_BOUNCY_BALLS; // spawn full capacity (can reduce later)
    for _ in 0..desired {
    let radius = rng.gen_range(7.5..15.0); // 25% smaller than previous (10..20)
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let speed = rng.gen_range(10.0..40.0); // halved speed range
        let vel = Vec2::from_angle(angle) * speed;
        balls.push(BouncyBall { pos: Vec2::new(x, y), vel, radius });
    }

    commands.insert_resource(BouncyBalls { balls, active: desired });
    info!("Spawned {desired} bouncy balls");
}

fn update_bouncy_balls(time: Res<Time>, mut sim: ResMut<BouncyBalls>, params: Res<BouncyParams>) {
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }

    let grav = if params.enable_gravity { params.gravity } else { Vec2::ZERO } * params.speed_dampen;
    let rest = params.restitution;

    let active = sim.active;
    for b in sim.balls.iter_mut().take(active) {
        b.vel += grav * dt;
        b.pos += b.vel * dt;

        // Boundary collision (-HALF_EXTENT .. HALF_EXTENT)
        let min = -HALF_EXTENT + b.radius;
        let max = HALF_EXTENT - b.radius;

        if b.pos.x < min { b.pos.x = min; b.vel.x = -b.vel.x * rest; }
        else if b.pos.x > max { b.pos.x = max; b.vel.x = -b.vel.x * rest; }

        if b.pos.y < min { b.pos.y = min; b.vel.y = -b.vel.y * rest; }
        else if b.pos.y > max { b.pos.y = max; b.vel.y = -b.vel.y * rest; }
    }
}

// Map world (-HALF_EXTENT..HALF_EXTENT) to pixel space (0..screen) given screen size.
fn world_to_pixel(p: Vec2, screen_w: f32, screen_h: f32) -> Vec2 {
    // Normalize into 0..1 then scale to screen. Assume square world mapping anchored center.
    // We treat x and y independently; if screen is not square we stretch.
    let nx = (p.x + HALF_EXTENT) / WORLD_SIZE; // 0..1
    let ny = (p.y + HALF_EXTENT) / WORLD_SIZE; // 0..1
    Vec2::new(nx * screen_w, ny * screen_h)
}

fn sync_bouncy_into_gpu(
    sim: Res<BouncyBalls>,
    mut gpu_buf: ResMut<GpuBallBuffer>,
    mut params_u: ResMut<ParamsUniform>,
) {
    // Ensure GPU vector is already preallocated externally; avoid allocations.
    // If size mismatch, we truncate to existing length but NEVER reallocate here.
    let capacity = gpu_buf.balls.len();
    if capacity == 0 { return; }

    let screen_w = params_u.screen_size[0];
    let screen_h = params_u.screen_size[1];

    let to_copy = sim.active.min(capacity - 1); // leave space for sentinel if using

    let palette: [ [f32;4]; 4 ] = [
        [1.0, 0.3, 0.3, 1.0], // red-ish
        [0.3, 1.0, 0.3, 1.0], // green-ish
        [0.3, 0.3, 1.0, 1.0], // blue-ish
        [1.0, 1.0, 0.3, 1.0], // yellow-ish
    ];

    for (i, b) in sim.balls.iter().take(to_copy).enumerate() {
        let pix = world_to_pixel(b.pos, screen_w, screen_h);
        let dst = &mut gpu_buf.balls[i];
        dst.center = [pix.x, pix.y];
        dst.radius = b.radius; // keep radius in world units; shader can treat as world radius if coordinate mapping matches.
        // Assign cluster id and color by index for now (round-robin into 4 clusters)
        let cid = (i % 4) as i32;
        dst.cluster_id = cid;
        dst.color = palette[cid as usize];
    }

    // Sentinel after last active
    if to_copy < capacity { gpu_buf.balls[to_copy].radius = SENTINEL_RADIUS; }

    // Keep updating uniform num_balls for now (shader currently uses it). We set actual active.
    params_u.num_balls = to_copy as u32;
}

fn bouncy_debug_input(keys: Res<ButtonInput<KeyCode>>, mut params: ResMut<BouncyParams>) {
    if keys.just_pressed(KeyCode::KeyG) {
        params.enable_gravity = !params.enable_gravity;
        info!("Gravity {}", if params.enable_gravity { "ON" } else { "OFF" });
    }
    if keys.just_pressed(KeyCode::KeyS) {
        params.sync_to_gpu = !params.sync_to_gpu;
        info!("Sync {}", if params.sync_to_gpu { "ON" } else { "OFF" });
    }
}
