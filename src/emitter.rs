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

    // Omni-directional edge spawn
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(cfg.balls.radius_range.min..cfg.balls.radius_range.max);
    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let edge = rng.gen_range(0..4);
    let (x, y) = match edge {
        0 => (-half_w - radius, rng.gen_range(-half_h..half_h)),       // left outside
        1 => ( half_w + radius, rng.gen_range(-half_h..half_h)),       // right outside
        2 => (rng.gen_range(-half_w..half_w), -half_h - radius),       // bottom
        _ => (rng.gen_range(-half_w..half_w),  half_h + radius),       // top
    };
    let pos = Vec2::new(x, y);
    let to_center = (-pos).normalize_or_zero();
    // Random speed magnitude based on configured velocity ranges (reuse spread heuristically)
    let base_speed = rng.gen_range(cfg.balls.vel_x_range.min..cfg.balls.vel_x_range.max).abs();
    let jitter = Vec2::new(
        rng.gen_range(cfg.balls.vel_x_range.min..cfg.balls.vel_x_range.max),
        rng.gen_range(cfg.balls.vel_y_range.min..cfg.balls.vel_y_range.max),
    );
    let vel = to_center * base_speed + jitter * 0.2; // mostly inward
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
    cfg.draw_circles,
    );
}
