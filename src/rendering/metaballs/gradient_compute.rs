//! Half-resolution metaball field + gradient compute prepass (Phase 1)
//! NOTE: Phase 1 writes RGBA16F texture: (field, dF/dx, dF/dy, cluster_id_or_0)
//! The fragment shader does NOT yet sample this texture; visual output remains identical.
//! Cluster dominance channel (A) reserved for Phase 2 (currently always 0.0).
use bevy::prelude::*;
use bevy::render::{
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
    renderer::{RenderContext, RenderDevice},
    render_resource::*,
    Extract,
};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_asset::RenderAssets;
use bevy::render::storage::GpuShaderStorageBuffer;
use bevy::render::texture::GpuImage;
use std::borrow::Cow;

#[cfg(target_arch = "wasm32")] use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")] static METABALLS_GRADIENT_COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
pub fn init_wasm_gradient_shader(world: &mut World) {
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    let handle = shaders.add(Shader::from_wgsl(
        include_str!("../../../assets/shaders/metaballs_gradient_compute.wgsl"),
        "metaballs_gradient_compute_embedded.wgsl",
    ));
    METABALLS_GRADIENT_COMPUTE_SHADER_HANDLE.get_or_init(|| handle);
}

// ------------------------------------------------------------------------------------------------
// Resources
// ------------------------------------------------------------------------------------------------
#[derive(Resource, Default)]
pub struct MetaballsGradientPipeline {
    pub pipeline_id: Option<CachedComputePipelineId>,
    pub shader: Option<Handle<Shader>>,
    pub bind_group_layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>,
    pub logged: bool,
}

#[derive(Resource, Default, Clone)]
pub struct MetaballsGradientImages {
    pub tex: Option<Handle<Image>>,
    pub size: UVec2,
}

#[derive(Resource)]
pub struct MetaballsGradientToggle(pub bool);
impl Default for MetaballsGradientToggle { fn default() -> Self { Self(true) } }

#[derive(Resource, Default)]
pub struct MetaballsGradientStats { pub dispatches: u64 }

// ------------------------------------------------------------------------------------------------
// Systems (Render schedule)
// ------------------------------------------------------------------------------------------------
pub fn prepare_gradient_pipeline(
    mut pipe: ResMut<MetaballsGradientPipeline>,
    pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
) {
    if pipe.shader.is_none() {
        #[cfg(target_arch = "wasm32")] {
            pipe.shader = Some(METABALLS_GRADIENT_COMPUTE_SHADER_HANDLE.get().unwrap().clone());
        }
        #[cfg(not(target_arch = "wasm32"))] {
            pipe.shader = Some(asset_server.load("shaders/metaballs_gradient_compute.wgsl"));
        }
    }
    if pipe.bind_group_layout.is_none() {
        // Layout matches WGSL bindings (group 0):
        // 0: uniform (metaballs data) – mirrored subset, 1..4 storage buffers, 5: storage texture
        let layout = render_device.create_bind_group_layout(
            "metaballs.gradient.bind_group_layout",
            &[
                BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 2, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 3, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 4, visibility: ShaderStages::COMPUTE, ty: BindingType::Buffer { ty: BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 5, visibility: ShaderStages::COMPUTE, ty: BindingType::StorageTexture { access: StorageTextureAccess::WriteOnly, format: TextureFormat::Rgba16Float, view_dimension: TextureViewDimension::D2 }, count: None },
            ],
        );
        pipe.bind_group_layout = Some(layout);
    }
    if pipe.pipeline_id.is_none() {
        let Some(shader) = pipe.shader.clone() else { return; };
        let Some(layout) = &pipe.bind_group_layout else { return; };
        let desc = ComputePipelineDescriptor {
            label: Some("metaballs.gradient.compute".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader,
            entry_point: Cow::from("cs_main"),
            shader_defs: vec![],
            zero_initialize_workgroup_memory: false,
        };
        pipe.pipeline_id = Some(pipeline_cache.queue_compute_pipeline(desc));
    }
}

pub fn prepare_gradient_target_main(
    mut images: ResMut<MetaballsGradientImages>,
    mut images_assets: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return; };
    let (fw, fh) = (window.width().max(1.0), window.height().max(1.0));
    let half = UVec2::new(((fw * 0.5).ceil()) as u32, ((fh * 0.5).ceil()) as u32);
    if images.tex.is_some() && images.size == half { return; }
    // Allocate / reallocate
    let mut img = Image::new_fill(
        Extent3d { width: half.x, height: half.y, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &vec![0u8; (half.x * half.y * 8) as usize],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.texture_descriptor.usage = TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
    if images.tex.is_none() {
        let handle = images_assets.add(img);
        images.tex = Some(handle);
    } else if let Some(handle) = &images.tex {
        if let Some(existing) = images_assets.get_mut(handle) { *existing = img; }
    }
    images.size = half;
}

pub fn log_gradient_once(mut pipe: ResMut<MetaballsGradientPipeline>) {
    if pipe.pipeline_id.is_some() && !pipe.logged {
        info!(target="metaballs", "Gradient compute prepass active (half-res field/gradient)");
        pipe.logged = true;
    }
}

// ------------------------------------------------------------------------------------------------
// Render Graph Node
// ------------------------------------------------------------------------------------------------
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MetaballsGradientComputeNodeLabel;

#[derive(Default)]
pub struct MetaballsGradientComputeNode;

impl Node for MetaballsGradientComputeNode {
    fn run(&self, _graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
        let toggle = world.get_resource::<MetaballsGradientToggle>().map(|t| t.0).unwrap_or(true);
        if !toggle { return Ok(()); }
        let Some(pipe) = world.get_resource::<MetaballsGradientPipeline>() else { return Ok(()); };
        let Some(pid) = pipe.pipeline_id else { return Ok(()); };
        let cache = world.resource::<PipelineCache>();
        let Some(pipeline) = cache.get_compute_pipeline(pid) else { return Ok(()); };
        let Some(bg) = &pipe.bind_group else { return Ok(()); };
        let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("metaballs_gradient_precompute"), timestamp_writes: None });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bg, &[]);
        // Derive dispatch dims from texture view
        let images = world.resource::<MetaballsGradientImages>();
        if images.size.x == 0 || images.size.y == 0 { return Ok(()); }
        let wg_x = (images.size.x + 7) / 8; // workgroup_size 8x8
        let wg_y = (images.size.y + 7) / 8;
        pass.dispatch_workgroups(wg_x, wg_y, 1);
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
// Late bind group assembly (separate to ensure image + buffers exist)
// ------------------------------------------------------------------------------------------------
pub fn assemble_gradient_bind_group(
    mut pipe: ResMut<MetaballsGradientPipeline>,
    images_res: Option<Res<MetaballsGradientImages>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    materials: Option<Res<Assets<crate::rendering::metaballs::material::MetaballsUnifiedMaterial>>>,
    ssbos: Res<RenderAssets<GpuShaderStorageBuffer>>,
) {
    let Some(images_res) = images_res else { return; };
    if pipe.bind_group_layout.is_none() { return; }
    let Some(materials) = materials else { return; };
    let Some(layout) = &pipe.bind_group_layout else { return; };
    let Some(img_handle) = &images_res.tex else { return; };
    let Some(gpu_image) = gpu_images.get(img_handle) else { return; };
    // Pick first material (single fullscreen quad assumption)
    let Some(mat) = materials.iter().next().map(|(_, m)| m.clone()) else { return; };
    // Ensure required buffers exist
    if ssbos.get(&mat.balls).is_none() || ssbos.get(&mat.tile_headers).is_none() || ssbos.get(&mat.tile_ball_indices).is_none() {
        return;
    }
    // Create a tiny uniform buffer snapshot from material uniform (re-using struct layout)
    let mut uniform_bytes: Vec<u8> = vec![0u8; std::mem::size_of::<crate::rendering::metaballs::gpu::MetaballsUniform>()];
    // SAFETY: Plain old data copy
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&mat.data as *const crate::rendering::metaballs::gpu::MetaballsUniform) as *const u8,
            uniform_bytes.as_mut_ptr(),
            uniform_bytes.len(),
        );
    }
    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("metaballs.gradient.uniform"),
        contents: &uniform_bytes,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let view = &gpu_image.texture_view;
    let entries = [
        BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
    BindGroupEntry { binding: 1, resource: ssbos.get(&mat.balls).unwrap().buffer.as_entire_binding() },
    BindGroupEntry { binding: 2, resource: ssbos.get(&mat.tile_headers).unwrap().buffer.as_entire_binding() },
    BindGroupEntry { binding: 3, resource: ssbos.get(&mat.tile_ball_indices).unwrap().buffer.as_entire_binding() },
    BindGroupEntry { binding: 4, resource: if let Some(p) = ssbos.get(&mat.cluster_palette) { p.buffer.as_entire_binding() } else { ssbos.get(&mat.tile_headers).unwrap().buffer.as_entire_binding() } },
    BindGroupEntry { binding: 5, resource: BindingResource::TextureView(view) },
    ];
    let bg = render_device.create_bind_group("metaballs.gradient.bind_group", layout, &entries);
    pipe.bind_group = Some(bg);
}

// Extraction: copy main-world gradient image handle & size into render world each frame.
pub fn extract_gradient_images(mut commands: bevy::ecs::system::Commands, images: Extract<Res<MetaballsGradientImages>>) {
    commands.insert_resource(images.clone());
}

// Approximate stats increment (counts frames where dispatch would occur)
pub fn accumulate_gradient_stats(
    mut stats: ResMut<MetaballsGradientStats>,
    images: Option<Res<MetaballsGradientImages>>,
    pipe: Option<Res<MetaballsGradientPipeline>>,
    toggle: Option<Res<MetaballsGradientToggle>>,
) {
    if let (Some(img), Some(p), Some(t)) = (images, pipe, toggle) {
        if t.0 && p.pipeline_id.is_some() && img.size.x > 0 && img.size.y > 0 { stats.dispatches += 1; }
    }
}
