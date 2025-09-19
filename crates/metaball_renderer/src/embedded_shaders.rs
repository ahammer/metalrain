use bevy::prelude::*;
use std::sync::OnceLock;

static COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
static PRESENT_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

const COMPUTE_WGSL: &str = include_str!("../assets/shaders/compute_metaballs.wgsl");
#[cfg(feature = "present")] const PRESENT_WGSL: &str = include_str!("../assets/shaders/present_fullscreen.wgsl");

pub fn ensure_loaded(world: &mut World) {
    if !world.contains_resource::<Assets<Shader>>() { 
        world.insert_resource(Assets::<Shader>::default()); 
    }
    // Create synthetic handles by loading from memory (Bevy 0.16 supports AssetServer::load with path only; we insert directly instead)
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    COMPUTE_SHADER_HANDLE.get_or_init(|| {
        let shader = Shader::from_wgsl(COMPUTE_WGSL, file!());
        shaders.add(shader)
    });
    #[cfg(feature = "present")]
    PRESENT_SHADER_HANDLE.get_or_init(|| {
        let shader = Shader::from_wgsl(PRESENT_WGSL, file!());
        shaders.add(shader)
    });
}

pub fn compute_handle() -> Handle<Shader> { COMPUTE_SHADER_HANDLE.get().cloned().expect("compute shader loaded") }
#[cfg(feature = "present")]
pub fn present_handle() -> Handle<Shader> { PRESENT_SHADER_HANDLE.get().cloned().expect("present shader loaded") }
