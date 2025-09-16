use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use crate::constants::*;
use crate::compute::types::{Ball, BallBuffer, ParamsUniform, TimeUniform, MetaballTarget, MetaballAlbedoTarget};
use crate::bouncy::{BouncyBallSimulationPlugin, MAX_BOUNCY_BALLS, SENTINEL_RADIUS};

pub struct MetaballSimulationPlugin;

impl Plugin for MetaballSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BouncyBallSimulationPlugin)
            .add_systems(Startup, setup_compute_target)
            .add_systems(Update, (debug_input,));
    }
}

fn setup_compute_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let mut image = Image::new(
        Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0u8; (WIDTH * HEIGHT * 8) as usize], // 4 channels * 2 bytes (f16) each
        TextureFormat::Rgba16Float,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let handle = images.add(image);

    // Albedo texture (8-bit RGBA) for color outputs from compute
    let mut albedo = Image::new(
        Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![0u8; (WIDTH * HEIGHT * 4) as usize], // 4 channels * 1 byte each
        TextureFormat::Rgba8Unorm,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    albedo.texture_descriptor.usage =
        TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    let albedo_handle = images.add(albedo);

    // Preallocate GPU-side ball buffer to maximum capacity; fill sentinel radius.
    let balls = vec![
        Ball { center: [0.0, 0.0], radius: SENTINEL_RADIUS, cluster_id: 0, color: [0.0,0.0,0.0,0.0] };
        MAX_BOUNCY_BALLS
    ];

    commands.insert_resource(MetaballTarget { texture: handle });
    commands.insert_resource(MetaballAlbedoTarget { texture: albedo_handle });
    commands.insert_resource(BallBuffer { balls });
    commands.insert_resource(TimeUniform::default());
    commands.insert_resource(ParamsUniform {
        screen_size: [WIDTH as f32, HEIGHT as f32],
        num_balls: 0, // will be updated each frame by sync system
        debug_mode: 0,
        iso: 2.2,
        ambient: 0.25,
        rim_power: 2.5,
        show_centers: 1,
        clustering_enabled: 1,
        _pad: 0.0,
    });
}

fn debug_input(keys: Res<ButtonInput<KeyCode>>, mut params: ResMut<ParamsUniform>) {
    if keys.just_pressed(KeyCode::KeyF) {
        params.debug_mode = (params.debug_mode + 1) % 5;
        info!("Switched debug mode to {}", params.debug_mode);
    }
    if keys.just_pressed(KeyCode::KeyC) {
        params.show_centers = 1 - params.show_centers;
        info!("Centers {}", if params.show_centers == 1 { "ON" } else { "OFF" });
    }
}
