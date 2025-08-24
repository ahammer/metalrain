use bevy::prelude::Mesh2d;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
static BG_WORLDGRID_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
struct BgData {
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
impl BgData {
    fn window_size(&self) -> Vec2 {
        Vec2::new(self.v0.x, self.v0.y)
    }
    fn set_window_size(&mut self, size: Vec2) {
        self.v0.x = size.x;
        self.v0.y = size.y;
    }
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
struct BgMaterial {
    #[uniform(0)]
    data: BgData,
}
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
        {
            "shaders/bg_worldgrid.wgsl".into()
        }
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
        {
            "shaders/bg_worldgrid.wgsl".into()
        }
    }
}
#[derive(Component)]
pub struct BackgroundQuad;
pub struct BackgroundPlugin;
impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            use bevy::asset::Assets;
            use bevy::render::render_resource::Shader;
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let worldgrid = shaders.add(Shader::from_wgsl(
                include_str!("../../../assets/shaders/bg_worldgrid.wgsl"),
                "bg_worldgrid_embedded.wgsl",
            ));
            BG_WORLDGRID_SHADER_HANDLE.get_or_init(|| worldgrid.clone());
        }
        app.add_plugins(Material2dPlugin::<BgMaterial>::default())
            .add_systems(Startup, setup_background)
            .add_systems(Update, resize_bg_uniform);
    }
}
fn setup_background(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut grid_mats: ResMut<Assets<BgMaterial>>,
    windows: Query<&Window>,
) {
    let (w, h) = if let Ok(win) = windows.single() {
        (win.width(), win.height())
    } else {
        (800.0, 600.0)
    };
    let mut grid_mat = BgMaterial::default();
    grid_mat.data.set_window_size(Vec2::new(w, h));
    let grid_handle = grid_mats.add(grid_mat);
    let mesh = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));
    commands.spawn((
        Mesh2d::from(mesh),
        MeshMaterial2d(grid_handle),
        Transform::from_xyz(0.0, 0.0, -500.0),
        Visibility::Visible,
        InheritedVisibility::VISIBLE,
        BackgroundQuad,
    ));
    info!("Background grid spawned");
}
fn resize_bg_uniform(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<BgMaterial>, With<BackgroundQuad>>,
    mut materials: ResMut<Assets<BgMaterial>>,
) {
    let Ok(win) = windows.single() else {
        return;
    };
    let Ok(handle) = q_mat.single() else {
        return;
    };
    if let Some(mat) = materials.get_mut(&handle.0) {
        let ws = mat.data.window_size();
        if ws.x != win.width() || ws.y != win.height() {
            mat.data
                .set_window_size(Vec2::new(win.width(), win.height()));
        }
    }
}
