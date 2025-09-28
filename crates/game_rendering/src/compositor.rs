use bevy::prelude::*;
use bevy::render::camera::{
    Camera, CameraProjection, ClearColorConfig, OrthographicProjection, Projection, ScalingMode,
};
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::render::view::RenderLayers;
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};

use crate::layers::{BlendMode, LayerToggleState, RenderLayer};
use crate::targets::{RenderTargetHandles, RenderTargets};

const FINAL_LAYER_MASK: usize = 31;

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct LayerBlendState {
    pub blend_modes: [BlendMode; 5],
}

impl Default for LayerBlendState {
    fn default() -> Self {
        Self {
            blend_modes: [
                BlendMode::Normal,
                BlendMode::Normal,
                BlendMode::Additive,
                BlendMode::Additive,
                BlendMode::Normal,
            ],
        }
    }
}

impl LayerBlendState {
    pub fn blend_for(&self, layer: RenderLayer) -> BlendMode {
        self.blend_modes[layer.order()]
    }

    pub fn set_blend_for(&mut self, layer: RenderLayer, mode: BlendMode) {
        self.blend_modes[layer.order()] = mode;
    }
}

#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct CompositorSettings {
    pub exposure: f32,
    pub debug_layer_boundaries: bool,
}

impl Default for CompositorSettings {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            debug_layer_boundaries: false,
        }
    }
}

#[derive(Clone, Copy, ShaderType, Debug)]
pub struct CompositorUniforms {
    pub settings: Vec4,
    pub enabled_low: Vec4,
    pub enabled_high: Vec4,
    pub blend_modes_low: Vec4,
    pub blend_modes_high: Vec4,
}

#[derive(Asset, TypePath, Debug, Clone, AsBindGroup)]
pub struct CompositorMaterial {
    #[uniform(0)]
    pub uniforms: CompositorUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub background: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub game_world: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub metaballs: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub effects: Handle<Image>,
    #[texture(9)]
    #[sampler(10)]
    pub ui: Handle<Image>,
}

impl Default for CompositorMaterial {
    fn default() -> Self {
        Self {
            uniforms: CompositorUniforms {
                settings: Vec4::new(1.0, 0.0, 0.0, 0.0),
                enabled_low: Vec4::splat(1.0),
                enabled_high: Vec4::new(1.0, 0.0, 0.0, 0.0),
                blend_modes_low: Vec4::ZERO,
                blend_modes_high: Vec4::ZERO,
            },
            background: Handle::default(),
            game_world: Handle::default(),
            metaballs: Handle::default(),
            effects: Handle::default(),
            ui: Handle::default(),
        }
    }
}

impl Material2d for CompositorMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/compositor.wgsl".into())
    }
}

pub struct CompositorMaterialPlugin;
impl Plugin for CompositorMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<CompositorMaterial>::default());
    }
}

#[derive(Resource, Default, Debug)]
pub struct CompositorPresentation {
    pub quad_entity: Option<Entity>,
    pub camera_entity: Option<Entity>,
    pub material: Option<Handle<CompositorMaterial>>,
    pub mesh_handle: Option<Handle<Mesh>>,
}

pub fn setup_compositor_pass(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CompositorMaterial>>,
    mut presentation: ResMut<CompositorPresentation>,
    targets: Res<RenderTargets>,
    handles: Res<RenderTargetHandles>,
    layer_state: Res<LayerToggleState>,
    blend_state: Res<LayerBlendState>,
    settings: Res<CompositorSettings>,
) {
    if presentation.quad_entity.is_some() {
        return;
    }

    let resolution = targets.resolution.max(UVec2::new(1, 1));

    let mesh_handle = meshes.add(Rectangle::new(resolution.x as f32, resolution.y as f32));

    let mut material = CompositorMaterial::default();
    apply_layer_textures(&mut material, &handles);
    update_uniforms(
        &mut material.uniforms,
        &settings,
        &layer_state,
        &blend_state,
    );
    let material_handle = materials.add(material);

    let quad_entity = commands
        .spawn((
            Name::new("CompositorQuad"),
            Mesh2d(mesh_handle.clone()),
            MeshMaterial2d(material_handle.clone()),
            Transform::IDENTITY,
            RenderLayers::layer(FINAL_LAYER_MASK),
        ))
        .id();

    let mut projection = make_orthographic(resolution);
    projection.update(resolution.x as f32, resolution.y as f32);

    let camera_entity = commands
        .spawn((
            Name::new("CompositorCamera"),
            Camera2d,
            Camera {
                order: 1000,
                clear_color: ClearColorConfig::None,
                ..Default::default()
            },
            Projection::from(projection),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            RenderLayers::layer(FINAL_LAYER_MASK),
        ))
        .id();

    presentation.quad_entity = Some(quad_entity);
    presentation.camera_entity = Some(camera_entity);
    presentation.material = Some(material_handle);
    presentation.mesh_handle = Some(mesh_handle);
}

pub fn sync_compositor_material(
    layer_state: Res<LayerToggleState>,
    blend_state: Res<LayerBlendState>,
    settings: Res<CompositorSettings>,
    handles: Res<RenderTargetHandles>,
    mut materials: ResMut<Assets<CompositorMaterial>>,
    presentation: Res<CompositorPresentation>,
) {
    if presentation.material.is_none() {
        return;
    }
    if !(layer_state.is_changed()
        || blend_state.is_changed()
        || settings.is_changed()
        || handles.is_changed())
    {
        return;
    }

    if let Some(material_handle) = presentation.material.as_ref() {
        if let Some(material) = materials.get_mut(material_handle) {
            apply_layer_textures(material, &handles);
            update_uniforms(
                &mut material.uniforms,
                &settings,
                &layer_state,
                &blend_state,
            );
        }
    }
}

pub fn sync_compositor_geometry(
    targets: Res<RenderTargets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut presentation: ResMut<CompositorPresentation>,
    mut mesh_query: Query<&mut Mesh2d>,
    mut projection_query: Query<&mut Projection>,
) {
    if !targets.is_changed() {
        return;
    }

    let resolution = targets.resolution.max(UVec2::new(1, 1));

    if let Some(entity) = presentation.quad_entity {
        if let Ok(mut mesh_handle) = mesh_query.get_mut(entity) {
            let new_mesh = meshes.add(Rectangle::new(resolution.x as f32, resolution.y as f32));
            presentation.mesh_handle = Some(new_mesh.clone());
            *mesh_handle = Mesh2d(new_mesh);
        }
    }

    if let Some(camera_entity) = presentation.camera_entity {
        if let Ok(mut projection) = projection_query.get_mut(camera_entity) {
            let mut ortho = make_orthographic(resolution);
            ortho.update(resolution.x as f32, resolution.y as f32);
            *projection = Projection::from(ortho);
        }
    }
}

fn apply_layer_textures(material: &mut CompositorMaterial, handles: &RenderTargetHandles) {
    material.background = texture_for(handles, RenderLayer::Background);
    material.game_world = texture_for(handles, RenderLayer::GameWorld);
    material.metaballs = texture_for(handles, RenderLayer::Metaballs);
    material.effects = texture_for(handles, RenderLayer::Effects);
    material.ui = texture_for(handles, RenderLayer::Ui);
}

fn texture_for(handles: &RenderTargetHandles, layer: RenderLayer) -> Handle<Image> {
    handles.layers.get(&layer).cloned().unwrap_or_default()
}

fn update_uniforms(
    uniforms: &mut CompositorUniforms,
    settings: &CompositorSettings,
    layer_state: &LayerToggleState,
    blend_state: &LayerBlendState,
) {
    let mut enabled_low = [0.0f32; 4];
    let mut enabled_high = [0.0f32; 4];
    let mut blend_low = [0.0f32; 4];
    let mut blend_high = [0.0f32; 4];

    for (idx, layer) in RenderLayer::ALL.iter().enumerate() {
        let enabled = layer_state
            .config(*layer)
            .map(|cfg| cfg.enabled)
            .unwrap_or(true);
        let blend_mode = blend_state.blend_for(*layer);
        let enabled_value = if enabled { 1.0 } else { 0.0 };
        let blend_value = encode_blend_mode(blend_mode) as f32;

        if idx < 4 {
            enabled_low[idx] = enabled_value;
            blend_low[idx] = blend_value;
        } else {
            enabled_high[idx - 4] = enabled_value;
            blend_high[idx - 4] = blend_value;
        }
    }

    uniforms.settings = Vec4::new(
        settings.exposure,
        if settings.debug_layer_boundaries { 1.0 } else { 0.0 },
        0.0,
        0.0,
    );
    uniforms.enabled_low = Vec4::from_array(enabled_low);
    uniforms.enabled_high = Vec4::from_array(enabled_high);
    uniforms.blend_modes_low = Vec4::from_array(blend_low);
    uniforms.blend_modes_high = Vec4::from_array(blend_high);
}

fn encode_blend_mode(mode: BlendMode) -> u32 {
    match mode {
        BlendMode::Normal => 0,
        BlendMode::Additive => 1,
        BlendMode::Multiply => 2,
    }
}

fn make_orthographic(resolution: UVec2) -> OrthographicProjection {
    OrthographicProjection {
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
    }
}
