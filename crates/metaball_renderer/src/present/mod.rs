use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};

use crate::internal::{AlbedoTexture, FieldTexture, NormalTexture};
use crate::settings::MetaballRenderSettings;

// This module implements an optional presentation quad that simply maps the offscreen
// metaball textures (field, albedo, normals) onto a rectangle covering the configured
// `world_bounds`. Camera responsibility is entirely external; the plugin will NOT spawn
// a camera (keeping architectural decoupling). Users typically add a `Camera2d` with an
// orthographic scaling mode that fits their content.

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MetaballDisplayMaterial {
    #[texture(0)]
    texture: Handle<Image>,
    #[texture(1)]
    #[sampler(2)]
    albedo: Handle<Image>,
    #[texture(3)]
    normals: Handle<Image>,
}

impl Material2d for MetaballDisplayMaterial {
    fn fragment_shader() -> ShaderRef {
        // Present shader now always loaded via AssetServer path (embedded variant removed)
        ShaderRef::Path("shaders/present_fullscreen.wgsl".into())
    }
}

pub struct MetaballDisplayPlugin;

impl Plugin for MetaballDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<MetaballDisplayMaterial>::default())
            .add_systems(PostStartup, setup_present)
            .add_systems(PostStartup, log_presentation_quad.after(setup_present));
    }
}
#[derive(Component, Debug)]
pub struct MetaballPresentationQuad;

fn setup_present(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballDisplayMaterial>>,
    field: Res<FieldTexture>,
    albedo: Res<AlbedoTexture>,
    normals: Res<NormalTexture>,
    mut commands: Commands,
    settings: Res<MetaballRenderSettings>,
) {
    // Build rectangle matching world bounds extents.
    let world_size = settings.world_bounds.size();
    let quad_mesh = Mesh::from(Rectangle::new(world_size.x, world_size.y));
    let quad_handle = meshes.add(quad_mesh);
    let material_handle = materials.add(MetaballDisplayMaterial {
        texture: field.0.clone(),
        albedo: albedo.0.clone(),
        normals: normals.0.clone(),
    });
    let mut entity = commands.spawn((
        Mesh2d(quad_handle),
        MeshMaterial2d(material_handle),
        Transform::IDENTITY,
        MetaballPresentationQuad,
        Name::new("MetaballPresentationQuad"),
    ));
    if let Some(layer) = settings.presentation_layer {
        entity.insert(RenderLayers::layer(layer as usize));
        info!(target: "metaballs", "Presentation quad assigned to explicit layer {layer}");
    } else {
        info!(target: "metaballs", "Presentation quad using default layer (0)");
    }
    info!(target: "metaballs", "Spawned presentation quad covering world bounds {:?}", settings.world_bounds);
}

/// Debug instrumentation: logs the name + layers of the presentation quad after spawn.
fn log_presentation_quad(query: Query<(Entity, &Name, Option<&RenderLayers>), With<MetaballPresentationQuad>>) {
    for (entity, name, layers) in &query {
        let layers_bits = layers.map(|l| format!("{:?}", l.bits())).unwrap_or_else(|| "<none>".to_string());
        info!(target: "metaballs", ?entity, quad=?name, layers=%layers_bits, "Metaball presentation quad state");
    }
}
