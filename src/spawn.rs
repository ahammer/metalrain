use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::components::{Ball, BallRadius};
use crate::materials::{BallDisplayMaterials, BallPhysicsMaterials, BallMaterialIndex, BallMaterialsInitSet};
use crate::config::GameConfig;

pub struct BallSpawnPlugin;

#[derive(Resource, Clone)]
pub struct CircleMesh(pub Handle<Mesh>);

impl Plugin for BallSpawnPlugin {
    fn build(&self, app: &mut App) {
        // Ensure we spawn only after materials have been initialized so palette-based randomization works.
        app.add_systems(Startup, spawn_balls.after(BallMaterialsInitSet));
    }
}

fn spawn_balls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GameConfig>,
    display_palette: Option<Res<BallDisplayMaterials>>,
    physics_palette: Option<Res<BallPhysicsMaterials>>,
) {
    // Create shared unit circle mesh once (radius 0.5 so scale = diameter)
    let circle_handle = meshes.add(Mesh::from(Circle { radius: 0.5 }));
    commands.insert_resource(CircleMesh(circle_handle.clone()));
    let mut rng = rand::thread_rng();
    let c = &cfg.balls;

    // Determine ring radius OFF-SCREEN: choose a circle whose radius exceeds the window half-diagonal
    // so every spawn starts fully outside view (even in corners).
    // half diagonal = sqrt((w/2)^2 + (h/2)^2). Add max ball radius + small margin.
    let half_w = cfg.window.width * 0.5;
    let half_h = cfg.window.height * 0.5;
    let half_diag = (half_w * half_w + half_h * half_h).sqrt();
    let max_ball_r = c.radius_range.max.max(c.radius_range.min);
    let ring_radius = half_diag + max_ball_r + 10.0; // 10px safety margin
    let center = Vec2::ZERO;
    let count = c.count.max(1);

    for i in 0..count {
        let t = i as f32 / count as f32;
        let angle = t * std::f32::consts::TAU;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let radius = if c.radius_range.min < c.radius_range.max {
            rng.gen_range(c.radius_range.min..c.radius_range.max)
        } else { c.radius_range.min };
    let pos2 = center + dir * ring_radius;

        // Derive a reasonable max speed from provided velocity ranges (use absolute extremes)
        let vx_ext = c.vel_x_range.max.abs().max(c.vel_x_range.min.abs());
        let vy_ext = c.vel_y_range.max.abs().max(c.vel_y_range.min.abs());
        let max_speed = vx_ext.max(vy_ext).max(1.0); // ensure > 0
        // Sample a base speed between 30% and 100% of that max for variety
        let base_speed = rng.gen_range(0.30 * max_speed..max_speed);

        // Chaos: add tangential component + random scalar jitter so motion not perfectly radial.
        let tangential = Vec2::new(-dir.y, dir.x);
        let tangential_factor = rng.gen_range(-0.6..0.6); // spin variation
        let radial_jitter = rng.gen_range(0.85..1.15);    // vary inward magnitude
        let extra_noise = Vec2::new(
            rng.gen_range(-0.25..0.25) * base_speed,
            rng.gen_range(-0.25..0.25) * base_speed,
        );
        let vel = (-dir * base_speed * radial_jitter) + (tangential * base_speed * tangential_factor) + extra_noise;

        // Choose material variant index & restitution
        let (material, restitution, variant_idx) = if let (Some(disp), Some(phys)) = (&display_palette, &physics_palette) {
            if !disp.0.is_empty() && !phys.0.is_empty() {
                let idx_range_end = disp.0.len().min(phys.0.len());
                let idx = if idx_range_end > 1 { rng.gen_range(0..idx_range_end) } else { 0 };
                (disp.0[idx].clone(), phys.0[idx].restitution, idx)
            } else {
                let color = Color::srgb(
                    rng.gen::<f32>() * 0.9 + 0.1,
                    rng.gen::<f32>() * 0.9 + 0.1,
                    rng.gen::<f32>() * 0.9 + 0.1,
                );
                (materials.add(color), cfg.bounce.restitution, 0)
            }
        } else {
            // Fallback to random color if palettes missing (should not happen after MaterialsPlugin loads)
            let color = Color::srgb(
                rng.gen::<f32>() * 0.9 + 0.1,
                rng.gen::<f32>() * 0.9 + 0.1,
                rng.gen::<f32>() * 0.9 + 0.1,
            );
            (materials.add(color), cfg.bounce.restitution, 0)
        };

        spawn_ball_entity(
            &mut commands,
            &circle_handle,
            Vec3::new(pos2.x, pos2.y, 0.0),
            vel,
            radius,
            material,
            restitution,
            variant_idx,
            cfg.draw_circles,
        );
    }

    info!("spawned {} balls in off-screen ring (radius {:.1})", count, ring_radius);
}

/// Reusable helper to spawn a single ball.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ball_entity(
    commands: &mut Commands,
    circle_mesh: &Handle<Mesh>,
    translation: Vec3,
    linear_vel: Vec2,
    radius: f32,
    material: Handle<ColorMaterial>,
    restitution: f32,
    variant_idx: usize,
    draw_circles: bool,
) {
    commands
        .spawn((
            Transform::from_translation(translation),
            GlobalTransform::default(),
            VisibilityBundle::default(),
            RigidBody::Dynamic,
            Collider::ball(radius),
            Velocity::linear(linear_vel),
            Restitution::coefficient(restitution),
            Damping { linear_damping: 0.0, angular_damping: 0.0 },
            ActiveEvents::COLLISION_EVENTS,
            Ball,
            BallRadius(radius),
            BallMaterialIndex(variant_idx),
        ))
        .with_children(|parent| {
            if draw_circles {
                parent.spawn(bevy::sprite::MaterialMesh2dBundle {
                    mesh: circle_mesh.clone().into(),
                    material,
                    transform: Transform::from_scale(Vec3::splat(radius * 2.0)),
                    ..default()
                });
            }
        });
}
