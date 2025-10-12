use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::camera::{
    Camera, CameraProjection, ClearColorConfig, OrthographicProjection, Projection, RenderTarget,
    ScalingMode,
};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::RenderLayers;
use bevy::window::{PrimaryWindow, WindowResized};
use std::collections::HashMap;

use crate::camera::GameCamera;
use crate::layers::{LayerConfig, LayerToggleState, RenderLayer};

#[derive(Resource, Debug, Clone)]
pub struct RenderSurfaceSettings {
    pub base_resolution: UVec2,
    pub use_window_resolution: bool,
    pub format: TextureFormat,
}

impl Default for RenderSurfaceSettings {
    fn default() -> Self {
        Self {
            base_resolution: UVec2::new(1280, 720),
            use_window_resolution: true,
            format: TextureFormat::Bgra8UnormSrgb,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct RenderTargetHandles {
    pub layers: HashMap<RenderLayer, Handle<Image>>,
    pub final_composite: Handle<Image>,
}

impl Default for RenderTargetHandles {
    fn default() -> Self {
        Self {
            layers: HashMap::new(),
            final_composite: Handle::default(),
        }
    }
}

#[derive(Resource, Debug)]
pub struct RenderTargets {
    pub resolution: UVec2,
    pub layers: HashMap<RenderLayer, LayerRenderTarget>,
    pub final_composite: Handle<Image>,
    pub camera_rig: Option<Entity>,
}

impl Default for RenderTargets {
    fn default() -> Self {
        Self {
            resolution: UVec2::ONE,
            layers: HashMap::new(),
            final_composite: Handle::default(),
            camera_rig: None,
        }
    }
}

impl RenderTargets {
    pub fn layer_handle(&self, layer: RenderLayer) -> Option<Handle<Image>> {
        self.layers.get(&layer).map(|lt| lt.image.clone())
    }
}

#[derive(Debug, Clone)]
pub struct LayerRenderTarget {
    pub layer: RenderLayer,
    pub image: Handle<Image>,
    pub camera: Entity,
    pub config: LayerConfig,
}

pub fn setup_render_targets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut targets: ResMut<RenderTargets>,
    mut handles: ResMut<RenderTargetHandles>,
    settings: Res<RenderSurfaceSettings>,
    layer_state: Res<LayerToggleState>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let window_opt = primary_window.iter().next();
    let resolution = determine_resolution(&settings, window_opt);
    targets.resolution = resolution;

    let camera_entity = commands
        .spawn((
            Name::new("GameCameraRig"),
            Transform::default(),
            GlobalTransform::default(),
            GameCamera {
                base_resolution: Vec2::new(resolution.x as f32, resolution.y as f32),
                ..Default::default()
            },
        ))
        .id();

    let mut layer_map = HashMap::new();
    handles.layers.clear();

    for (index, config) in layer_state.configs.iter().enumerate() {
        let image =
            create_color_image(&mut images, resolution, settings.format, config.clear_color);
        let camera = spawn_layer_camera(
            &mut commands,
            config,
            resolution,
            image.clone(),
            index as u8,
        );

        layer_map.insert(
            config.layer,
            LayerRenderTarget {
                layer: config.layer,
                image: image.clone(),
                camera,
                config: config.clone(),
            },
        );
        handles.layers.insert(config.layer, image);
    }

    let final_image = create_color_image(&mut images, resolution, settings.format, Color::BLACK);
    handles.final_composite = final_image.clone();

    targets.layers = layer_map;
    targets.final_composite = final_image;
    targets.camera_rig = Some(camera_entity);
}

fn spawn_layer_camera(
    commands: &mut Commands,
    config: &LayerConfig,
    resolution: UVec2,
    image: Handle<Image>,
    mask_index: u8,
) -> Entity {
    let mut projection = OrthographicProjection {
        near: -1000.0,
        far: 1000.0,
        viewport_origin: Vec2::splat(0.5),
        scaling_mode: ScalingMode::Fixed {
            width: resolution.x as f32,
            height: resolution.y as f32,
        },
        scale: 1.0,
        area: Rect::from_center_size(
            Vec2::ZERO,
            Vec2::new(resolution.x as f32, resolution.y as f32),
        ),
    };
    projection.update(resolution.x as f32, resolution.y as f32);

    let camera_entity = commands
        .spawn((
            Name::new(format!("LayerCamera::{:?}", config.layer)),
            Camera2d,
            Camera {
                target: RenderTarget::Image(image.clone().into()),
                order: config.layer.order() as isize,
                clear_color: ClearColorConfig::Custom(config.clear_color),
                ..Default::default()
            },
            Projection::from(projection),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            RenderLayers::layer(mask_index as usize),
        ))
        .id();

    camera_entity
}

pub fn handle_window_resize(
    mut resize_events: EventReader<WindowResized>,
    settings: Res<RenderSurfaceSettings>,
    mut targets: ResMut<RenderTargets>,
    mut handles: ResMut<RenderTargetHandles>,
    mut images: ResMut<Assets<Image>>,
) {
    if !settings.use_window_resolution {
        return;
    }

    let Some(last) = resize_events.read().last().cloned() else {
        return;
    };

    let new_resolution = UVec2::new(last.width.max(1.0) as u32, last.height.max(1.0) as u32);
    if new_resolution == targets.resolution {
        return;
    }

    targets.resolution = new_resolution;

    for target in targets.layers.values() {
        if let Some(image) = images.get_mut(&target.image) {
            resize_image(image, new_resolution);
        }
    }

    if let Some(final_image) = images.get_mut(&targets.final_composite) {
        resize_image(final_image, new_resolution);
    }

    for (layer, target) in targets.layers.iter() {
        handles.layers.insert(*layer, target.image.clone());
    }
    handles.final_composite = targets.final_composite.clone();
}

fn resize_image(image: &mut Image, resolution: UVec2) {
    image.resize(Extent3d {
        width: resolution.x,
        height: resolution.y,
        depth_or_array_layers: 1,
    });
}

fn determine_resolution(settings: &RenderSurfaceSettings, window: Option<&Window>) -> UVec2 {
    if settings.use_window_resolution {
        if let Some(window) = window {
            let width = window.physical_width().max(1);
            let height = window.physical_height().max(1);
            return UVec2::new(width, height);
        }
    }
    settings.base_resolution
}

pub fn create_color_image(
    images: &mut Assets<Image>,
    size: UVec2,
    format: TextureFormat,
    clear_color: Color,
) -> Handle<Image> {
    let pixel_count = (size.x * size.y) as usize;
    let mut data = vec![0u8; pixel_count * 4];

    if clear_color != Color::NONE {
        let srgba = clear_color.to_srgba();
        let bytes = [
            (srgba.red.clamp(0.0, 1.0) * 255.0) as u8,
            (srgba.green.clamp(0.0, 1.0) * 255.0) as u8,
            (srgba.blue.clamp(0.0, 1.0) * 255.0) as u8,
            (srgba.alpha.clamp(0.0, 1.0) * 255.0) as u8,
        ];
        for chunk in data.chunks_mut(4) {
            chunk.copy_from_slice(&bytes);
        }
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        format,
        RenderAssetUsages::default(),
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();

    images.add(image)
}
