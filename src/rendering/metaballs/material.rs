//! Material definition and shader binding setup for unified metaballs.
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite::Material2d;

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
static METABALLS_UNIFIED_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

use crate::rendering::metaballs::gpu::{MetaballsUniform, NoiseParamsUniform, SurfaceNoiseParamsUniform};

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct MetaballsUnifiedMaterial {
    #[uniform(0)] pub data: MetaballsUniform,
    #[uniform(1)] pub noise: NoiseParamsUniform,
    #[uniform(2)] pub surface_noise: SurfaceNoiseParamsUniform,
    #[storage(3, read_only)] pub balls: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)] pub tile_headers: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)] pub tile_ball_indices: Handle<ShaderStorageBuffer>,
    #[storage(6, read_only)] pub cluster_palette: Handle<ShaderStorageBuffer>,
    #[texture(7)] #[sampler(9)] pub sdf_atlas_tex: Option<Handle<Image>>,
    #[storage(8, read_only)] pub sdf_shape_meta: Handle<ShaderStorageBuffer>,
}

impl MetaballsUnifiedMaterial {
    #[cfg(feature = "debug")]
    pub fn set_debug_view(&mut self, view: u32) { self.data.v1.w = view as f32; }
    pub fn debug_counts(&self) -> (u32, u32) { (self.data.v0.x as u32, self.data.v0.y as u32) }
}

impl Material2d for MetaballsUnifiedMaterial {
    fn fragment_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")] { ShaderRef::Handle(METABALLS_UNIFIED_SHADER_HANDLE.get().unwrap().clone()) }
        #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs_unified.wgsl".into() }
    }
    fn vertex_shader() -> ShaderRef { Self::fragment_shader() }
}

#[cfg(target_arch = "wasm32")]
pub fn init_wasm_shader(world: &mut World) {
    use bevy::asset::Assets; use bevy::render::render_resource::Shader;
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    let unified_handle = shaders.add(Shader::from_wgsl(include_str!("../../../assets/shaders/metaballs_unified.wgsl"), "metaballs_unified_embedded.wgsl"));
    METABALLS_UNIFIED_SHADER_HANDLE.get_or_init(|| unified_handle.clone());
}
