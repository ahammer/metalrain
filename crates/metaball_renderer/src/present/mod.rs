use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use crate::internal::{FieldTexture, AlbedoTexture};
use crate::settings::MetaballRenderSettings;
use crate::embedded_shaders;
use bevy::window::{PrimaryWindow, WindowResized};

const SIM_TEXTURE_SIZE: f32 = 512.0;

#[derive(Clone, Copy, ShaderType, Debug)]
pub struct MetaballPresentParams {
    // (scale_u, offset_u, scale_v, offset_v) — now always identity since we rely on geometry scaling/cropping
    pub scale_offset: Vec4,
}

impl Default for MetaballPresentParams {
    fn default() -> Self { Self { scale_offset: Vec4::new(1.0, 0.0, 1.0, 0.0) } }
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
            .add_systems(PostStartup, (setup_present,))
            .add_systems(Update, (resize_fit_cover,));
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
        commands.spawn((Camera2d, MetaballCameraTag));
    }

    let Ok(window) = windows.single() else { return; };

    // Fixed-size square mesh centered at origin.
    let quad = Mesh::from(Rectangle::new(SIM_TEXTURE_SIZE, SIM_TEXTURE_SIZE));
    let quad_handle = meshes.add(quad);

    let material_handle = materials.add(MetaballDisplayMaterial {
        texture: field.0.clone(),
        albedo: albedo.0.clone(),
        params: MetaballPresentParams::default(),
    });

    let scale = cover_scale(window.width(), window.height());

    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::new(scale, scale, 1.0)), // uniform scale for cover fit
        MetaballPresentQuad,
    ));
}

// Update scaling on resize so quad "covers" the screen (cropping one axis).
fn resize_fit_cover(
    mut resize_events: EventReader<WindowResized>,
    mut q: Query<&mut Transform, With<MetaballPresentQuad>>,
) {
    let Some(last) = resize_events.read().last().cloned() else { return; };
    let scale = cover_scale(last.width, last.height);
    for mut transform in &mut q {
        // Preserve any Z / rotation (currently none).
        transform.scale.x = scale;
        transform.scale.y = scale;
    }
}

// Compute uniform scale so a SIM_TEXTURE_SIZE square covers the window (cropping on the smaller axis).
fn cover_scale(w: f32, h: f32) -> f32 {
    if w <= 0.0 || h <= 0.0 { return 1.0; }
    let target_side = w.max(h); // cover: match the larger screen dimension
    // target_side / SIM_TEXTURE_SIZE / 2.0
    1.0
}
