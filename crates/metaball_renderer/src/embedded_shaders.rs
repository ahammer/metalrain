use bevy::prelude::*;
use bevy::render::render_resource::ShaderRef;
#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
use std::sync::OnceLock;

// Embedded shader handles (fallback + wasm path). Hot reload path uses ShaderRef::Path.
#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
static COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
static NORMALS_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[cfg(all(
    feature = "present",
    any(target_arch = "wasm32", not(feature = "shader_hot_reload"))
))]
static PRESENT_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
const COMPUTE_WGSL: &str = include_str!("../assets/shaders/compute_metaballs.wgsl");
#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
const NORMALS_WGSL: &str = include_str!("../assets/shaders/compute_3d_normals.wgsl");
#[cfg(all(
    feature = "present",
    any(target_arch = "wasm32", not(feature = "shader_hot_reload"))
))]
const PRESENT_WGSL: &str = include_str!("../assets/shaders/present_fullscreen.wgsl");

#[allow(unused_variables)]
pub fn ensure_loaded(world: &mut World) {
    #[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
    {
        if !world.contains_resource::<Assets<Shader>>() {
            world.insert_resource(Assets::<Shader>::default());
        }
        let mut shaders = world.resource_mut::<Assets<Shader>>();
        COMPUTE_SHADER_HANDLE.get_or_init(|| {
            let shader = Shader::from_wgsl(COMPUTE_WGSL, file!());
            shaders.add(shader)
        });
        NORMALS_SHADER_HANDLE.get_or_init(|| {
            let shader = Shader::from_wgsl(NORMALS_WGSL, file!());
            shaders.add(shader)
        });
        #[cfg(feature = "present")]
        {
            PRESENT_SHADER_HANDLE.get_or_init(|| {
                let shader = Shader::from_wgsl(PRESENT_WGSL, file!());
                shaders.add(shader)
            });
        }
    }
}

#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
pub fn compute_handle() -> Handle<Shader> {
    COMPUTE_SHADER_HANDLE
        .get()
        .cloned()
        .expect("compute shader loaded")
}
#[cfg(any(target_arch = "wasm32", not(feature = "shader_hot_reload")))]
pub fn normals_handle() -> Handle<Shader> {
    NORMALS_SHADER_HANDLE
        .get()
        .cloned()
        .expect("normals shader loaded")
}
#[cfg(all(
    feature = "present",
    any(target_arch = "wasm32", not(feature = "shader_hot_reload"))
))]
pub fn present_handle() -> Handle<Shader> {
    PRESENT_SHADER_HANDLE
        .get()
        .cloned()
        .expect("present shader loaded")
}

// Public helpers returning a ShaderRef that is path-based when hot reload is enabled on native.
// (Hot-reload aware handles are selected in pipeline construction with AssetServer.)
#[cfg(feature = "present")]
pub fn present_shader_ref() -> ShaderRef {
    #[cfg(all(feature = "shader_hot_reload", not(target_arch = "wasm32")))]
    {
        ShaderRef::Path("metaball://shaders/present_fullscreen.wgsl".into())
    }
    #[cfg(any(not(feature = "shader_hot_reload"), target_arch = "wasm32"))]
    {
        ShaderRef::Handle(present_handle())
    }
}

/// Plugin registering a custom asset source for hot reloading shaders inside this crate.
/// Safe to add regardless of feature/target (gates internally).
pub struct MetaballShaderSourcePlugin;
impl Plugin for MetaballShaderSourcePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(all(feature = "shader_hot_reload", not(target_arch = "wasm32")))]
        {
            use bevy::asset::io::AssetSourceBuilder;
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
            let path_str = path.to_string_lossy();
            // NOTE: This must run BEFORE AssetPlugin finalizes to take effect. If the user adds
            // this plugin after `DefaultPlugins` the source won't register; we silently rely on
            // embedded shaders then. Documented in crate docs.
            app.register_asset_source(
                "metaball",
                AssetSourceBuilder::platform_default(&path_str, None),
            );
            info!(target: "metaballs", "Metaball shader hot reload enabled (source 'metaball' -> {path_str})");
        }
    }
}

/// Debug system: log shader asset events for this crate (optional; can be disabled later).
#[allow(dead_code)]
pub(crate) fn log_shader_events(
    mut events: EventReader<bevy::asset::AssetEvent<Shader>>,
    asset_server: Res<AssetServer>,
) {
    for ev in events.read() {
        match ev {
            bevy::asset::AssetEvent::Modified { id } => {
                if let Some(path) = asset_server.get_path(*id) {
                    if path.path().to_string_lossy().contains("metaballs")
                        || path.to_string().contains("metaball://shaders")
                    {
                        info!(target: "metaballs", "Shader modified -> {path}");
                    }
                }
            }
            _ => {}
        }
    }
}
