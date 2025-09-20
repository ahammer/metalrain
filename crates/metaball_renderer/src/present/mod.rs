use bevy::prelude::*;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use crate::internal::{FieldTexture, AlbedoTexture};
use crate::settings::MetaballRenderSettings;
use crate::embedded_shaders;
use bevy::window::{PrimaryWindow, WindowResized};

#[derive(Clone, Copy, ShaderType, Debug)]
pub struct MetaballPresentParams {
    // (scale_u, offset_u, scale_v, offset_v)
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
            // Update quad mesh when window size changes.
            .add_systems(Update, (resize_present_quad,));
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
    // Safely get the primary window; if it's not yet available, abort setup for this frame.
    let Ok(window) = windows.single() else { return; };
    let quad = Mesh::from(Rectangle::new(window.width(), window.height()));
    let quad_handle = meshes.add(quad);
    let params = aspect_params(window.width(), window.height());
    let material_handle = materials.add(MetaballDisplayMaterial {
        texture: field.0.clone(),
        albedo: albedo.0.clone(),
        params,
    });
    commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::from_scale(Vec3::splat(1.0)),
        MetaballPresentQuad,
    ));
}

// Regenerate the fullscreen quad mesh when the primary window is resized.
fn resize_present_quad(
    mut resize_events: EventReader<WindowResized>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
    q: Query<(&Mesh2d, &MeshMaterial2d<MetaballDisplayMaterial>), With<MetaballPresentQuad>>,
) {
    let Some(last) = resize_events.read().last().cloned() else { return; };
    for (mesh2d, mat_handle) in q.iter() {
        if let Some(mesh) = meshes.get_mut(&mesh2d.0) {
            *mesh = Mesh::from(Rectangle::new(last.width, last.height));
        }
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.params = aspect_params(last.width, last.height);
        }
    }
}

fn aspect_params(w: f32, h: f32) -> MetaballPresentParams {
    if w <= 0.0 || h <= 0.0 { return MetaballPresentParams::default(); }
    let aspect = w / h;
    if aspect >= 1.0 {
        // Landscape: crop vertically (inset V)
        let scale_v = 1.0 / aspect; // < 1
        let offset_v = (1.0 - scale_v) * 0.5;
        MetaballPresentParams { scale_offset: Vec4::new(1.0, 0.0, scale_v, offset_v) }
    } else {
        // Portrait: crop horizontally (inset U)
        let scale_u = aspect; // < 1
        let offset_u = (1.0 - scale_u) * 0.5;
        MetaballPresentParams { scale_offset: Vec4::new(scale_u, offset_u, 1.0, 0.0) }
    }
}
