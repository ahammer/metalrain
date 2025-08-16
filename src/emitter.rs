use bevy::prelude::*;
use rand::Rng;

use crate::components::{Ball, BallRadius};
use crate::config::GameConfig;
use crate::spawn::{spawn_ball_entity, CircleMesh};
use crate::materials::{BallDisplayMaterials, BallPhysicsMaterials};

pub struct BallEmitterPlugin;

impl Plugin for BallEmitterPlugin {
    fn build(&self, app: &mut App) {
    app.insert_resource(EmitterTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .insert_resource(EmitterControl { enabled: true })
            .add_systems(Update, emit_balls);
    }
}

#[derive(Resource, Deref, DerefMut)]
struct EmitterTimer(Timer);

#[derive(Resource)]
struct EmitterControl { enabled: bool }

#[allow(clippy::too_many_arguments)] // Clear mapping to distinct engine resources; refactoring would reduce readability here.
fn emit_balls(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EmitterTimer>,
    circle: Option<Res<CircleMesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>, // in case circle mesh missing
    cfg: Res<GameConfig>,
    windows: Query<&Window>,
    q_balls: Query<&BallRadius, With<Ball>>,
    mut control: ResMut<EmitterControl>,
    display_palette: Option<Res<BallDisplayMaterials>>,
    physics_palette: Option<Res<BallPhysicsMaterials>>,
) {
    if !control.enabled { return; }
    let Ok(window) = windows.get_single() else { return; };

    // Ensure we have a shared circle mesh
    let circle_handle = if let Some(circle) = circle { circle.0.clone() } else {
        let handle = meshes.add(Mesh::from(Circle { radius: 0.5 }));
        commands.insert_resource(CircleMesh(handle.clone()));
        handle
    };

    // Compute coverage
    let total_ball_area: f32 = q_balls.iter().map(|r| std::f32::consts::PI * r.0 * r.0).sum();
    let field_area = window.width() * window.height();
    if total_ball_area / field_area >= 0.80 { control.enabled = false; return; }

    timer.tick(time.delta());
    if !timer.finished() { return; }

    // Spawn one ball above the visible top edge at a random horizontal position
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(cfg.balls.radius_range.min..cfg.balls.radius_range.max);
    let half_w = window.width() * 0.5;
    let x = rng.gen_range(-half_w + radius .. half_w - radius);
    // Match the top gap used in wall creation (keep in sync with rapier_physics.rs top_gap)
    let top_gap = 200.0;
    // Spawn just below the bottom surface of the (raised) top wall so they can fall into view.
    // Wall bottom is at half_h + top_gap; place center below that by radius + small offset.
    let y = window.height() * 0.5 + top_gap - radius - 5.0; // off-screen but inside arena
    // Random downward angle: pick a horizontal component within a spread and a downward (negative) vertical speed.
    // Horizontal spread kept lower than vertical to bias motion mostly downward.
    let horizontal = rng.gen_range(-25.0..25.0);
    let vertical = rng.gen_range(-60.0..-20.0); // negative -> downward
    let vel = Vec2::new(horizontal, vertical);
    let variant_idx = if let Some(p) = &display_palette { rng.gen_range(0..p.0.len()) } else { 0 };
    let (material, restitution) = if let (Some(disp), Some(phys)) = (&display_palette, &physics_palette) {
        (disp.0[variant_idx].clone(), phys.0[variant_idx].restitution)
    } else {
        let color = Color::srgb(
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
            rng.gen::<f32>() * 0.9 + 0.1,
        );
        (materials.add(color), cfg.bounce.restitution)
    };

    spawn_ball_entity(
        &mut commands,
        &circle_handle,
        Vec3::new(x, y, 0.0),
        vel,
        radius,
        material,
        restitution,
        variant_idx,
    );
}
