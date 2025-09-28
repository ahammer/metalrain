use bevy::prelude::*;
// Bouncy simulation demo
// Controls:
//   G - toggle gravity
//   C - toggle clustering pass (repacked into compute uniforms)
// Mirrors core behavioral parameters from original PoC while using the
// structured MetaballRendererPlugin packing + compute pipeline.
use rand::prelude::*;
use metaball_renderer::{MetaBall, MetaBallColor, MetaBallCluster, MetaballRenderSettings, RuntimeSettings};

// World half extent for simulation (logical space: -HALF_EXTENT..HALF_EXTENT in both axes)
pub const HALF_EXTENT: f32 = 256.0; // made public for debug viz / debug visualization overlays
// Interior collision walls are inset by a fixed padding to keep balls away from texture edge artifacts.
pub const COLLISION_PADDING: f32 = 64.0; // requested collision padding (wall inset)
const WORLD_SIZE: f32 = HALF_EXTENT * 2.0; // full logical span used for tex mapping (padding only affects collisions)

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
            .add_systems(Update, (update_balls, input_toggles).chain());
    }
}

fn spawn_balls(mut commands: Commands, _settings: Res<MetaballRenderSettings>) {
    let mut rng = StdRng::from_entropy();
    // Dynamic count – heuristic based on world area (mirrors previous texture based sizing)
    let area = (HALF_EXTENT * 2.0).powi(2).max(1.0);
    let mut desired = (area / (24.0*24.0)) as usize;
    desired = desired.clamp(64, 10_000); // arbitrary safety cap
    for i in 0..desired {
        let radius = rng.gen_range(1.0..8.0);
        // Spawn inside padded collision bounds
        let x = rng.gen_range((-HALF_EXTENT + COLLISION_PADDING) + radius..(HALF_EXTENT - COLLISION_PADDING) - radius);
        let y = rng.gen_range((-HALF_EXTENT + COLLISION_PADDING) + radius..(HALF_EXTENT - COLLISION_PADDING) - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(2.0..10.0);
        let vel = Vec2::from_angle(angle) * speed;
        let color_palette = [
            LinearRgba::new(1.0,0.3,0.3,1.0),
            // LinearRgba::new(0.3,1.0,0.3,1.0),
            // LinearRgba::new(0.3,0.3,1.0,1.0),
            // LinearRgba::new(1.0,1.0,0.3,1.0),
        ];
        let cluster = (i % color_palette.len()) as i32;
        commands.spawn((
            Transform::from_translation(Vec3::new(x,y,0.0)),
            GlobalTransform::default(),
            MetaBall { radius_world: radius },
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
    mut q: Query<(&mut Transform, &MetaBall, &mut Velocity)>
) {
    let dt = time.delta_secs(); if dt <= 0.0 { return; }
    let grav = if params.enable_gravity { params.gravity * params.speed_dampen } else { Vec2::ZERO };
    for (mut tr, mb, mut vel) in q.iter_mut() {
        let mut pos = tr.translation.truncate();
        vel.0 += grav * dt;
        pos += vel.0 * dt;
        // Collision bounds (inset by COLLISION_PADDING)
        let min = -HALF_EXTENT + COLLISION_PADDING + mb.radius_world;
        let max = HALF_EXTENT - COLLISION_PADDING - mb.radius_world;
        if pos.x < min { pos.x = min; vel.0.x = -vel.0.x * params.restitution; }
        else if pos.x > max { pos.x = max; vel.0.x = -vel.0.x * params.restitution; }
        if pos.y < min { pos.y = min; vel.0.y = -vel.0.y * params.restitution; }
        else if pos.y > max { pos.y = max; vel.0.y = -vel.0.y * params.restitution; }
        tr.translation.x = pos.x; tr.translation.y = pos.y;
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

// Legacy world<->texture mapping removed – handled internally during packing.
