use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings, MetaballShaderSourcePlugin, MetaBall, MetaBallColor, MetaBallCluster};

// Logical half-extent of the square world: coordinates range roughly in [-HALF_EXTENT, HALF_EXTENT]
const HALF_EXTENT: f32 = 256.0; // mirrors metaballs demo scale
const TEX_SIZE: UVec2 = UVec2::new(512,512); // texture used by metaball renderer
const WALL_THICKNESS: f32 = 10.0;
const NUM_BALLS: usize = 400; // reasonable default; adjust as desired
// We rely on Rapier's default gravity (approx -9.81 on Y). To exaggerate the effect, we apply a GravityScale > 1 on each ball.
const GRAVITY_SCALE: f32 = 0.0; // amplifies default gravity strength per ball

// --- Burst force (random area) tuning ---
const BURST_INTERVAL_SECONDS: f32 = 3.0;      // time between burst starts
const BURST_ACTIVE_SECONDS: f32 = 0.6;        // how long the burst applies force
const BURST_RADIUS: f32 = 110.0;              // influence radius in world units
const BURST_STRENGTH: f32 = 1400.0;           // peak outward force at center (Newtons-equivalent units for rapier)

// --- Wall repulsion pulse tuning ---
const WALL_PULSE_INTERVAL_SECONDS: f32 = 10.0; // time between wall repulsion pulses
const WALL_PULSE_ACTIVE_SECONDS: f32 = 0.8;    // duration of inward push from walls
const WALL_PULSE_DISTANCE: f32 = 120.0;        // distance from a wall within which bodies are affected
const WALL_PULSE_STRENGTH: f32 = 2200.0;       // strength scale for combined inward force

#[derive(Resource, Debug)]
struct BurstForceState {
    interval_timer: Timer,
    active_timer: Option<Timer>,
    center: Vec2,
}

impl Default for BurstForceState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(BURST_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
            center: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Debug)]
struct WallPulseState {
    interval_timer: Timer,
    active_timer: Option<Timer>,
}

impl Default for WallPulseState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(WALL_PULSE_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        // Metaball shader source (hot reload friendly) BEFORE DefaultPlugins just like other demo
        .add_plugins(MetaballShaderSourcePlugin)
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::with(MetaballRenderSettings { present: true, texture_size: TEX_SIZE, enable_clustering: true }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        // .add_plugins(RapierDebugRenderPlugin::default()) // optional physics debug
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .add_systems(Startup, (spawn_walls, spawn_balls))
        .add_systems(PreUpdate, (update_burst_force_state, apply_burst_forces).chain())
        // Move wall pulse to Update so Time has advanced; ensures timer fires correctly
        .add_systems(Update, (update_wall_pulse_state, apply_wall_pulse_forces).chain())
        .add_systems(PostUpdate, sync_metaballs)
        .run();
}

// (Removed unused setup_camera function that previously spawned a Camera2d)

fn spawn_walls(mut commands: Commands) {
    // Four axis-aligned fixed walls forming a box.
    // Use large cuboids slightly beyond the half-extent to ensure containment.
    let hx = HALF_EXTENT;
    let hy = HALF_EXTENT;
    let t = WALL_THICKNESS;

    // Bottom
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(hx + t, t),
        Transform::from_translation(Vec3::new(0.0, -hy - t, 0.0)),
        GlobalTransform::default(),
        Name::new("WallBottom"),
    ));
    // Top
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(hx + t, t),
        Transform::from_translation(Vec3::new(0.0, hy + t, 0.0)),
        GlobalTransform::default(),
        Name::new("WallTop"),
    ));
    // Left
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(t, hy + t),
        Transform::from_translation(Vec3::new(-hx - t, 0.0, 0.0)),
        GlobalTransform::default(),
        Name::new("WallLeft"),
    ));
    // Right
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(t, hy + t),
        Transform::from_translation(Vec3::new(hx + t, 0.0, 0.0)),
        GlobalTransform::default(),
        Name::new("WallRight"),
    ));
}

fn spawn_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    for i in 0..NUM_BALLS {
        let radius = rng.gen_range(3.0..12.0);
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(10.0..120.0);
        let vel = Vec2::from_angle(angle) * speed;
        // Simple small palette cycling for clusters/colors
        let palette = [
            LinearRgba::new(1.0,0.3,0.3,1.0),
            LinearRgba::new(0.3,1.0,0.4,1.0),
            LinearRgba::new(0.3,0.4,1.0,1.0),
            LinearRgba::new(1.0,0.9,0.3,1.0),
        ];
        let cluster = (i % palette.len()) as i32;

        commands
            .spawn((
                RigidBody::Dynamic,
                Collider::ball(radius),
                Restitution::coefficient(0.8),
                Damping { linear_damping: 0.2, angular_damping: 0.8 },
                Velocity { linvel: vel, angvel: rng.gen_range(-5.0..5.0) },
                GravityScale(GRAVITY_SCALE),
                Ccd::disabled(),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
                Transform::from_translation(Vec3::new(x, y, 0.0)),
                GlobalTransform::default(),
                Name::new(format!("Ball#{i}")),
                // Metaball components
                MetaBall { center: world_to_tex(Vec2::new(x,y)), radius },
                MetaBallColor(palette[cluster as usize]),
                MetaBallCluster(cluster),
            ))
            .insert(ExternalForce::default());
    }
    info!("Spawned {NUM_BALLS} balls in GameBoard_Test demo");
}

// Convert world coordinates (-HALF_EXTENT..HALF_EXTENT) into metaball texture space (0..tex_w)
fn world_to_tex(p: Vec2) -> Vec2 {
    let tex_w = TEX_SIZE.x as f32; let tex_h = TEX_SIZE.y as f32;
    Vec2::new(((p.x + HALF_EXTENT)/(HALF_EXTENT*2.0))*tex_w, ((p.y + HALF_EXTENT)/(HALF_EXTENT*2.0))*tex_h)
}

// Sync system: update MetaBall centers from Transform each frame (after physics)
fn sync_metaballs(mut q: Query<(&Transform, &mut MetaBall)>) {
    for (tr, mut mb) in &mut q {
        let pos = tr.translation.truncate();
        mb.center = world_to_tex(pos);
    }
}

// ---- Random Burst Force Systems ----

// Tick timers, choose new burst center when interval elapses.
fn update_burst_force_state(time: Res<Time>, mut state: ResMut<BurstForceState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() { state.active_timer = None; }
    }
    if state.interval_timer.just_finished() {
        // Pick a new random center within world bounds (slightly inset so radius stays inside)
        let mut rng = thread_rng();
        let margin = BURST_RADIUS * 0.5;
        let x = rng.gen_range(-HALF_EXTENT + margin .. HALF_EXTENT - margin);
        let y = rng.gen_range(-HALF_EXTENT + margin .. HALF_EXTENT - margin);
        state.center = Vec2::new(x, y);
        state.active_timer = Some(Timer::from_seconds(BURST_ACTIVE_SECONDS, TimerMode::Once));
        info!("Burst force activated at ({x:.1},{y:.1})");
    }
}

// Apply outward radial force while burst is active
fn apply_burst_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<BurstForceState>,
) {
    let Some(active) = state.active_timer.as_ref() else { return; };
    if active.finished() { return; }
    let center = state.center;
    let r2 = BURST_RADIUS * BURST_RADIUS;
    for (tr, mut force) in &mut q {
        let pos = tr.translation.truncate();
        let to_ball = pos - center; // outward
        let dist2 = to_ball.length_squared();
        if dist2 > r2 || dist2 < 1.0 { continue; }
        let dist = dist2.sqrt();
        let falloff = 1.0 - (dist / BURST_RADIUS); // linear falloff
        let dir = to_ball / dist;
        // Accumulate force (Rapier resets each step after integration)
        force.force += dir * BURST_STRENGTH * falloff;
    }
}

// ---- Wall Repulsion Pulse Systems ----

fn update_wall_pulse_state(time: Res<Time>, mut state: ResMut<WallPulseState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() { state.active_timer = None; }
    }
    if state.interval_timer.just_finished() {
        state.active_timer = Some(Timer::from_seconds(WALL_PULSE_ACTIVE_SECONDS, TimerMode::Once));
        info!("Wall repulsion pulse active");
    }
}

fn apply_wall_pulse_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<WallPulseState>,
) {
    let Some(active) = state.active_timer.as_ref() else { return; };
    if active.finished() { return; }
    let max_dist = WALL_PULSE_DISTANCE;
    for (tr, mut force) in &mut q {
        let p = tr.translation.truncate();
        let mut accum = Vec2::ZERO;
        // Distance to each wall; if within threshold push inward.
        let left_d = (p.x - (-HALF_EXTENT)).max(0.0);
        if left_d < max_dist { let f = 1.0 - left_d / max_dist; accum.x += f; }
        let right_d = (HALF_EXTENT - p.x).max(0.0);
        if right_d < max_dist { let f = 1.0 - right_d / max_dist; accum.x -= f; }
        let bottom_d = (p.y - (-HALF_EXTENT)).max(0.0);
        if bottom_d < max_dist { let f = 1.0 - bottom_d / max_dist; accum.y += f; }
        let top_d = (HALF_EXTENT - p.y).max(0.0);
        if top_d < max_dist { let f = 1.0 - top_d / max_dist; accum.y -= f; }
        if accum.length_squared() > 0.0001 {
            // Scale by strength and length (so corner overlaps stronger naturally)
            let magnitude = accum.length();
            let dir = accum / magnitude;
            force.force += dir * WALL_PULSE_STRENGTH * magnitude;
        }
    }
}
