use bevy::prelude::*;
use bevy::window::WindowResized;
use bevy_rapier2d::prelude::*;

use crate::config::GameConfig;

pub struct PhysicsSetupPlugin; // our wrapper to configure Rapier & arena

impl Plugin for PhysicsSetupPlugin {
    fn build(&self, app: &mut App) {
    app.add_plugins((RapierPhysicsPlugin::<NoUserData>::default(),))
    .add_systems(Startup, (configure_gravity, spawn_walls))
    .add_systems(Update, resize_walls);
    }
}

#[derive(Component)]
struct ArenaWall;

fn configure_gravity(mut rapier_cfg: ResMut<RapierConfiguration>, _game_cfg: Res<GameConfig>) {
    // Global gravity disabled for custom radial gravity system.
    rapier_cfg.gravity = Vect::new(0.0, 0.0);
}

fn spawn_walls(mut commands: Commands, windows: Query<&Window>) {
    let window = match windows.get_single() { Ok(w) => w, Err(_) => return };
    create_walls(&mut commands, window);
}

fn resize_walls(
    mut commands: Commands,
    mut evr: EventReader<WindowResized>,
    windows: Query<&Window>,
    existing: Query<Entity, With<ArenaWall>>,
) {
    if evr.read().next().is_some() {
        for e in &existing { commands.entity(e).despawn_recursive(); }
        if let Ok(window) = windows.get_single() { create_walls(&mut commands, window); }
    }
}

fn create_walls(commands: &mut Commands, window: &Window) {
    let half_w = window.width() * 0.5;
    let half_h = window.height() * 0.5;
    let thickness = 10.0;
    // Provide vertical gap above the visible window so we can spawn balls off-screen and let them fall in.
    let top_gap = 200.0; // height of extra spawn space above visible area
    // Left
    commands.spawn((Collider::cuboid(thickness * 0.5, half_h), RigidBody::Fixed, Transform::from_xyz(-half_w - thickness * 0.5, 0.0, 0.0), GlobalTransform::default(), ArenaWall));
    // Right
    commands.spawn((Collider::cuboid(thickness * 0.5, half_h), RigidBody::Fixed, Transform::from_xyz(half_w + thickness * 0.5, 0.0, 0.0), GlobalTransform::default(), ArenaWall));
    // Bottom
    commands.spawn((Collider::cuboid(half_w, thickness * 0.5), RigidBody::Fixed, Transform::from_xyz(0.0, -half_h - thickness * 0.5, 0.0), GlobalTransform::default(), ArenaWall));
    // Top (shifted upward to leave spawn gap)
    commands.spawn((Collider::cuboid(half_w, thickness * 0.5), RigidBody::Fixed, Transform::from_xyz(0.0, half_h + top_gap + thickness * 0.5, 0.0), GlobalTransform::default(), ArenaWall));
}
