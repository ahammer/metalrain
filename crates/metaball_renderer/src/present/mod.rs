use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::window::{PrimaryWindow, WindowResized};
use bevy::render::camera::{ScalingMode, Projection};

use crate::internal::{FieldTexture, AlbedoTexture};
use crate::settings::MetaballRenderSettings;
use crate::embedded_shaders;

const SIM_TEXTURE_SIZE: f32 = 512.0;

#[derive(Clone, Copy, ShaderType, Debug)]
pub struct MetaballPresentParams {
    // (scale_u, offset_u, scale_v, offset_v) — still identity (UVs unchanged)
    pub scale_offset: Vec4,
}

impl Default for MetaballPresentParams {
    fn default() -> Self {
        Self { scale_offset: Vec4::new(1.0, 0.0, 1.0, 0.0) }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MetaballDisplayMaterial {
    #[texture(0)]
    texture: Handle<Image>,
    #[texture(1)]
    #[sampler(2)]
    albedo: Handle<Image>,
    #[uniform(3)]
    params: MetaballPresentParams,
}

impl Material2d for MetaballDisplayMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(embedded_shaders::present_handle())
    }
}

pub struct MetaballDisplayPlugin;

impl Plugin for MetaballDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<MetaballDisplayMaterial>::default())
            .add_systems(PostStartup, setup_present)
            // Projection-based cover fit (runs on resize events).
            .add_systems(Update, update_projection_cover);
    }
}

#[derive(Component)]
struct MetaballCameraTag;

#[derive(Component)]
struct MetaballPresentQuad;

fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
    field: Res<FieldTexture>,
    albedo: Res<AlbedoTexture>,
    mut commands: Commands,
    _settings: Res<MetaballRenderSettings>,
    existing_cameras: Query<Entity, With<Camera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if existing_cameras.is_empty() {
        // Spawn a 2D camera at origin; projection will be adjusted later.
        commands.spawn((Camera2d, MetaballCameraTag));
    }

    let Ok(_window) = windows.single() else { return; };

    // Fixed 512x512 mesh centered at origin (Rectangle already centered).
    let quad = Mesh::from(Rectangle::new(SIM_TEXTURE_SIZE, SIM_TEXTURE_SIZE));
    let quad_handle = meshes.add(quad);

    let material_handle = materials.add(MetaballDisplayMaterial {
        texture: field.0.clone(),
        albedo: albedo.0.clone(),
        params: MetaballPresentParams::default(),
    });

    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::IDENTITY, // No scaling; projection handles zoom/cropping.
        MetaballPresentQuad,
    ));
}

// Adjust orthographic projection so the larger window dimension maps to 512 world units (cover fit).
fn update_projection_cover(
    mut resize_events: EventReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cams: Query<&mut Projection, With<MetaballCameraTag>>,
) {
    // Run only on the latest resize event (ignore intermediate).
    let Some(_last) = resize_events.read().last().cloned() else { return; };
    let Ok(window) = windows.single() else { return; };
    let Ok(mut projection) = cams.get_single_mut() else { return; };

    let w = window.width().max(1.0);
    let h = window.height().max(1.0);

    if let Projection::Orthographic(ref mut ortho) = *projection {
        // Cover logic: fix the larger screen axis to 512 units so the square overflows (crops) on the other axis.
        if w >= h {
            // Landscape: width larger -> constrain width to 512 (crop vertically)
            ortho.scaling_mode = ScalingMode::FixedHorizontal { viewport_width: SIM_TEXTURE_SIZE };
        } else {
            // Portrait: height larger -> constrain height to 512 (crop horizontally)
            ortho.scaling_mode = ScalingMode::FixedVertical { viewport_height: SIM_TEXTURE_SIZE };
        }
    }
}
