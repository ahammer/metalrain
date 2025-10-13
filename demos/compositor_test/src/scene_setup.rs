//! Scene setup systems for spawning initial entities.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

use game_rendering::RenderLayer;
use metaball_renderer::{MetaBall, MetaBallCluster, MetaBallColor};

use crate::components::EffectsPulse;
use crate::constants::*;

/// Sets up the initial scene with backdrop and overlay sprites.
pub fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.06, 0.12, 0.20, 0.65),
            custom_size: Some(Vec2::splat(HALF_EXTENT * 2.0 + 32.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        RenderLayers::layer(RenderLayer::GameWorld.order()),
        Name::new("GameWorld::PlayfieldBackdrop"),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(0.2, 0.6, 1.0, 0.18),
            custom_size: Some(Vec2::new(620.0, 620.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 30.0),
        RenderLayers::layer(RenderLayer::Effects.order()),
        EffectsPulse,
        Name::new("Effects::PulseOverlay"),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.06),
            custom_size: Some(Vec2::new(300.0, 80.0)),
            ..Default::default()
        },
        Transform::from_xyz(-480.0, 320.0, 200.0),
        RenderLayers::layer(RenderLayer::Ui.order()),
        Name::new("Ui::Placeholder"),
    ));
}

/// Configures the metaball presentation quad to render on the Metaballs layer.
pub fn configure_metaball_presentation(
    mut commands: Commands,
    mut done: Local<bool>,
    query: Query<(Entity, &Name), Without<RenderLayers>>,
) {
    if *done {
        return;
    }

    for (entity, name) in &query {
        if name.as_str() == "MetaballPresentationQuad" {
            commands
                .entity(entity)
                .insert(RenderLayers::layer(RenderLayer::Metaballs.order()));
            *done = true;
            info!(target: "compositor_demo", "Metaball presentation routed to Metaballs layer");
            break;
        }
    }
}

/// Spawns physics-enabled balls with metaball rendering.
pub fn spawn_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    let palette = [
        LinearRgba::new(1.0, 0.3, 0.3, 1.0),
        LinearRgba::new(0.3, 1.0, 0.4, 1.0),
        LinearRgba::new(0.3, 0.4, 1.0, 1.0),
        LinearRgba::new(1.0, 0.9, 0.3, 1.0),
    ];

    for i in 0..NUM_BALLS {
        let radius = rng.gen_range(3.0..12.0);
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(10.0..120.0);
        let vel = Vec2::from_angle(angle) * speed;
        let cluster = (i % palette.len()) as i32;
        let base_color = palette[cluster as usize];
        let sprite_color =
            Color::linear_rgba(base_color.red, base_color.green, base_color.blue, 0.2);

        let mut entity = commands.spawn((
            Sprite {
                color: sprite_color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            RenderLayers::layer(RenderLayer::GameWorld.order()),
            Name::new(format!("Ball#{i}")),
        ));

        entity.insert((
            RigidBody::Dynamic,
            Collider::ball(radius),
            Restitution::coefficient(0.8),
            Damping {
                linear_damping: 0.2,
                angular_damping: 0.8,
            },
            Velocity {
                linvel: vel,
                angvel: rng.gen_range(-5.0..5.0),
            },
            GravityScale(GRAVITY_SCALE),
            Ccd::disabled(),
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
        ));

        entity.insert((
            MetaBall {
                radius_world: radius,
            },
            MetaBallColor(base_color),
            MetaBallCluster(cluster),
            ExternalForce::default(),
        ));
    }

    info!("Spawned {NUM_BALLS} balls in compositor demo");
}
