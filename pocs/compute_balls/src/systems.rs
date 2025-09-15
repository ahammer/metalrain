use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use crate::constants::*;
use crate::compute::types::{Ball, BallBuffer, ParamsUniform, TimeUniform, MetaballTarget};

pub struct MetaballSimulationPlugin;

impl Plugin for MetaballSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_compute_target)
            .add_systems(Update, (animate_balls, debug_input));
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

    let balls = (0..12)
        .map(|i| {
            let t = i as f32 / 12.0 * std::f32::consts::TAU;
            Ball {
                center: [WIDTH as f32 * 0.5 + t.cos() * 120.0, HEIGHT as f32 * 0.5 + t.sin() * 80.0],
                radius: 40.0 + (i as f32 % 3.0) * 10.0,
                _pad: 0.0,
            }
        })
        .collect();

    commands.insert_resource(MetaballTarget { texture: handle });
    commands.insert_resource(BallBuffer { balls });
    commands.insert_resource(TimeUniform::default());
    commands.insert_resource(ParamsUniform {
        screen_size: [WIDTH as f32, HEIGHT as f32],
        num_balls: 12,
        debug_mode: 0,
        iso: 2.2,
        ambient: 0.25,
        rim_power: 2.5,
        show_centers: 1,
    });
}

fn animate_balls(
    time: Res<Time>,
    mut bufs: ResMut<BallBuffer>,
    params: Res<ParamsUniform>,
    mut time_u: ResMut<TimeUniform>,
) {
    time_u.time += time.delta_secs();
    let t = time_u.time;
    let n = params.num_balls.min(bufs.balls.len() as u32) as usize;
    for (i, b) in bufs.balls.iter_mut().take(n).enumerate() {
        let phase = i as f32 * 0.37;
        b.center[0] += (t * 0.9 + phase).sin() * 1.0;
        b.center[1] += (t * 0.7 + phase * 1.7).cos() * 1.0;
    }
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
