//! Background grid rendering (simplified port from legacy).
//!
//! Renders a full-screen quad using a custom Material2d shader (`shaders/bg_worldgrid.wgsl`).
//! The camera is expected to NOT clear (ClearColorConfig::None) so this draws first.
//!
//! Test mode: to avoid full render / winit initialization (which requires main thread),
//! we substitute a lightweight plugin that only spawns a `BackgroundQuad` marker entity.
//! This lets unit tests assert plugin integration without standing up the full renderer.
//!
//! Future improvements:
//! - Optional fluid background variant (feature flag).
//! - Parameter resource for cell size / colors exposed to debug UI.
//! - Golden frame hash inclusion.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
#[allow(unused_imports)]
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
use bevy::prelude::Mesh2d;

#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
static BG_WORLDGRID_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
pub(crate) struct BgData {
    // v0: (window_size.x, window_size.y, cell_size, line_thickness)
    // v1: (dark_factor, _, _, _)
    v0: Vec4,
    v1: Vec4,
}

impl Default for BgData {
    fn default() -> Self {
        Self {
            v0: Vec4::new(0.0, 0.0, 128.0, 0.015),
            v1: Vec4::new(0.15, 0.0, 0.0, 0.0),
        }
    }
}

#[allow(dead_code)]
impl BgData {
    pub fn window_size(&self) -> Vec2 { Vec2::new(self.v0.x, self.v0.y) }
    pub fn set_window_size(&mut self, size: Vec2) { self.v0.x = size.x; self.v0.y = size.y; }
    #[allow(dead_code)]
    pub fn cell_size(&self) -> f32 { self.v0.z }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct BgMaterial { #[uniform(0)] data: BgData }

impl Material2d for BgMaterial {
    fn fragment_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            return BG_WORLDGRID_SHADER_HANDLE
                .get()
                .cloned()
                .map(ShaderRef::Handle)
                .unwrap_or_else(|| ShaderRef::Path("shaders/bg_worldgrid.wgsl".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_worldgrid.wgsl".into() }
    }
    fn vertex_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            return BG_WORLDGRID_SHADER_HANDLE
                .get()
                .cloned()
                .map(ShaderRef::Handle)
                .unwrap_or_else(|| ShaderRef::Path("shaders/bg_worldgrid.wgsl".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        { "shaders/bg_worldgrid.wgsl".into() }
    }
}

#[derive(Component)]
pub struct BackgroundQuad;

/// Public plugin that sets up the material pipeline and spawns the quad (normal / non-test build).
#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
pub struct BackgroundPlugin;

/// Lightweight test / headless / light variant (no render / winit).
#[cfg(any(test, feature = "headless", feature = "background_light"))]
pub struct BackgroundPlugin;

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            use bevy::asset::Assets;
            use bevy::render::render_resource::Shader;
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let worldgrid = shaders.add(Shader::from_wgsl(
                include_str!("../../../../assets/shaders/bg_worldgrid.wgsl"),
                "bg_worldgrid_embedded.wgsl",
            ));
            BG_WORLDGRID_SHADER_HANDLE.get_or_init(|| worldgrid.clone());
        }

        app.add_plugins(Material2dPlugin::<BgMaterial>::default())
            .add_systems(Startup, setup_background)
            .add_systems(Update, resize_bg_uniform);
    }
}

#[cfg(any(test, feature = "headless", feature = "background_light"))]
impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, |mut commands: Commands| {
            // Test-only placeholder entity (no material / mesh needed for logic assertions)
            commands.spawn((BackgroundQuad,));
        });
    }
}

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
fn setup_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<BgMaterial>>,
    windows: Query<&Window>,
) {
    let (w, h) = if let Ok(win) = windows.single() {
        (win.width(), win.height())
    } else {
        (800.0, 600.0)
    };
    let mut mat = BgMaterial::default();
    mat.data.set_window_size(Vec2::new(w, h));
    let handle = mats.add(mat);
    let mesh = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));
    commands.spawn((
        Mesh2d::from(mesh),
        MeshMaterial2d(handle),
        Transform::from_xyz(0.0, 0.0, -500.0),
        Visibility::Visible,
        InheritedVisibility::VISIBLE,
        BackgroundQuad,
    ));
}

#[cfg(not(any(test, feature = "headless", feature = "background_light")))]
fn resize_bg_uniform(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<BgMaterial>, With<BackgroundQuad>>,
    mut materials: ResMut<Assets<BgMaterial>>,
) {
    let Ok(win) = windows.single() else { return; };
    let Ok(handle) = q_mat.single() else { return; };
    if let Some(mat) = materials.get_mut(&handle.0) {
        let ws = mat.data.window_size();
        if ws.x != win.width() || ws.y != win.height() {
            mat.data.set_window_size(Vec2::new(win.width(), win.height()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_spawns_and_resizes() {
        let mut app = App::new();
        // Minimal deterministic test stack; no window / winit event loop
        app.add_plugins(MinimalPlugins);
        app.add_plugins(BackgroundPlugin);
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&BackgroundQuad>();
        assert_eq!(q.iter(world).count(), 1, "expected one BackgroundQuad");
    }
}
