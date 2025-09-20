use bevy::prelude::*;
use std::collections::HashMap;
// Bouncy simulation demo
// Controls:
//   G - toggle gravity
//   C - toggle clustering pass (repacked into compute uniforms)
// Mirrors core behavioral parameters from original PoC while using the
// structured MetaballRendererPlugin packing + compute pipeline.
use rand::prelude::*;
use metaball_renderer::{MetaBall, MetaBallColor, MetaBallCluster, MetaballRenderSettings, RuntimeSettings};

// World half extent for simulation (logical space: -EXTENT..EXTENT in both axes)
pub const HALF_EXTENT: f32 = 256.0; // made public for debug viz
const WORLD_SIZE: f32 = HALF_EXTENT * 2.0;

#[derive(Component, Clone, Copy)]
pub(crate) struct Velocity(pub Vec2);

#[derive(Resource, Clone)]
pub(crate) struct BouncyParams {
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
            // Order: integrate -> collision resolution -> input toggles (for responsive toggles after physics)
            .add_systems(Update, (update_balls, resolve_collisions, input_toggles).chain());
    }
}

fn spawn_balls(mut commands: Commands, settings: Res<MetaballRenderSettings>) {
    let tex_w = settings.texture_size.x as f32; // square assumed but keep flexible
    let tex_h = settings.texture_size.y as f32;
    let mut rng = StdRng::from_entropy();
    // Dynamic count – choose based on texture area heuristic (1 ball per ~ (32x32) tile), clamp.
    let area = (tex_w * tex_h).max(1.0);
    let mut desired = (area / (32.0*32.0)) as usize;
    desired = desired.clamp(64, 10_000); // arbitrary safety cap
    for i in 0..desired {
        let radius = rng.gen_range(2.5..5.0);
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

/// Broad phase + narrow phase basic elastic collisions.
/// Positions & velocities are maintained in (logical) world space during resolution
/// then mapped back to texture space.
pub(crate) fn resolve_collisions(
    params: Res<BouncyParams>,
    settings: Res<MetaballRenderSettings>,
    mut q: Query<(&mut MetaBall, &mut Velocity)>
) {
    let tex_w = settings.texture_size.x as f32; let tex_h = settings.texture_size.y as f32;
    // Early exit if trivially small set
    let count = q.iter_mut().count();
    if count <= 1 { return; }
    // Collect snapshot (world positions) so we can freely mutate after pair processing.
    // Scope to drop first mutable borrow before second pass.
    struct Temp { pos: Vec2, radius: f32, vel: Vec2, mass: f32 }
    let mut temps: Vec<Temp> = Vec::with_capacity(count);
    {
        for (mb, vel) in q.iter_mut() {
            let world = tex_to_world(Vec2::new(mb.center.x, mb.center.y), tex_w, tex_h);
            let r = mb.radius;
            // Approximate mass proportional to area (r^2) for more natural momentum exchange.
            temps.push(Temp { pos: world, radius: r, vel: vel.0, mass: r * r });
        }
    }
    // NOTE: Entity ids not required presently.

    // Spatial hash (uniform grid) to reduce O(n^2) cost. Cell size ~ average diameter: pick 2 * median radius ~ use 64 as coarse default.
    let cell_size: f32 = 64.0;
    fn cell_key(p: Vec2, cell: f32) -> (i32,i32) { (((p.x + HALF_EXTENT)/cell) as i32, ((p.y + HALF_EXTENT)/cell) as i32) }
    let mut grid: HashMap<(i32,i32), Vec<usize>> = HashMap::new();
    for (i, t) in temps.iter().enumerate() { grid.entry(cell_key(t.pos, cell_size)).or_default().push(i); }

    // Pairwise resolution within cell + neighbors.
    let restitution = params.restitution;
    for i in 0..temps.len() {
        let (cx, cy) = cell_key(temps[i].pos, cell_size);
        for nx in (cx-1)..=(cx+1) { for ny in (cy-1)..=(cy+1) {
            if let Some(indices) = grid.get(&(nx,ny)) {
                for &j in indices { if j <= i { continue; }
                    let (ra, rb) = (temps[i].radius, temps[j].radius);
                    let sum_r = ra + rb;
                    let delta = temps[j].pos - temps[i].pos;
                    let dist2 = delta.length_squared();
                    if dist2 >= sum_r * sum_r || dist2 == 0.0 { continue; }
                    let dist = dist2.sqrt();
                    let penetration = sum_r - dist;
                    // Normalized direction
                    let n = if dist > 0.0 { delta / dist } else { Vec2::X };
                    // Positional correction (distribute by mass)
                    let (ma, mb) = (temps[i].mass, temps[j].mass);
                    let inv_sum = 1.0 / (ma + mb);
                    let move_a = -n * penetration * (mb * inv_sum);
                    let move_b =  n * penetration * (ma * inv_sum);
                    // Safe dual mutable borrow via split
                    let (ai, aj) = if i < j {
                        let (first, second) = temps.split_at_mut(j);
                        (&mut first[i], &mut second[0])
                    } else { continue; };
                    ai.pos += move_a;
                    aj.pos += move_b;
                    // Relative velocity along normal
                    let rel_v = ai.vel - aj.vel;
                    let rel_norm = rel_v.dot(n);
                    if rel_norm > 0.0 { continue; } // moving apart after positional solve
                    // 1D elastic impulse along normal with masses
                    let impulse_mag = -(1.0 + restitution) * rel_norm / ( (1.0/ma) + (1.0/mb) );
                    let impulse = n * impulse_mag;
                    ai.vel += impulse / ma;
                    aj.vel -= impulse / mb;
                }
            }
        }}
    }

    // Write back
    // Second pass to mutate query safely
    for ((mut mb, mut vel), t) in q.iter_mut().zip(temps.into_iter()) {
        mb.center = world_to_tex(t.pos, tex_w, tex_h);
        vel.0 = t.vel;
    }
}

fn input_toggles(
    keys: Res<ButtonInput<KeyCode>>,
    mut bouncy: ResMut<BouncyParams>,
    rt: Option<ResMut<RuntimeSettings>>,
) {
    if keys.just_pressed(KeyCode::KeyG) {
        bouncy.enable_gravity = !bouncy.enable_gravity;
        info!("Gravity {}", if bouncy.enable_gravity { "ON" } else { "OFF" });
    }
    if keys.just_pressed(KeyCode::KeyC) {
        if let Some(mut rt) = rt { rt.clustering_enabled = !rt.clustering_enabled; info!("Clustering {}", if rt.clustering_enabled {"ON"} else {"OFF"}); }
    }
}

// Mapping helpers parameterized by texture dimensions.
fn world_to_tex(p: Vec2, tex_w: f32, tex_h: f32) -> Vec2 {
    Vec2::new(((p.x + HALF_EXTENT)/WORLD_SIZE)*tex_w, ((p.y + HALF_EXTENT)/WORLD_SIZE)*tex_h)
}
fn tex_to_world(p: Vec2, tex_w: f32, tex_h: f32) -> Vec2 {
    Vec2::new((p.x / tex_w)*WORLD_SIZE - HALF_EXTENT, (p.y / tex_h)*WORLD_SIZE - HALF_EXTENT)
}
