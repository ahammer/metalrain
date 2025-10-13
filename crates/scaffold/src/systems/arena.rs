use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use game_rendering::RenderLayer;

use crate::resources::ScaffoldConfig;

const WALL_NAME: [&str; 4] = ["WallBottom", "WallTop", "WallLeft", "WallRight"];

/// Spawns a square arena with four static colliders and simple visual quads.
pub fn spawn_physics_arena(
    mut commands: Commands,
    config: Res<ScaffoldConfig>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;

    let half = config.world_half_extent;
    let thickness = config.wall_thickness;

    let horizontal_size = Vec2::new((half + thickness) * 2.0, thickness * 2.0);
    let vertical_size = Vec2::new(thickness * 2.0, (half + thickness) * 2.0);
    let wall_color = Color::srgba(0.25, 0.45, 0.70, 0.45);

    let positions = [
        Vec3::new(0.0, -half - thickness, 0.0),
        Vec3::new(0.0, half + thickness, 0.0),
        Vec3::new(-half - thickness, 0.0, 0.0),
        Vec3::new(half + thickness, 0.0, 0.0),
    ];

    for (index, position) in positions.iter().enumerate() {
        let (half_extents, size) = if index < 2 {
            (Vec2::new(half + thickness, thickness), horizontal_size)
        } else {
            (Vec2::new(thickness, half + thickness), vertical_size)
        };

        commands
            .spawn((
                Name::new(WALL_NAME[index]),
                RigidBody::Fixed,
                Collider::cuboid(half_extents.x, half_extents.y),
                Transform::from_translation(*position),
                GlobalTransform::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Sprite {
                        color: wall_color,
                        custom_size: Some(size),
                        ..Default::default()
                    },
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    RenderLayers::layer(RenderLayer::GameWorld.order()),
                ));
            });
    }
}
