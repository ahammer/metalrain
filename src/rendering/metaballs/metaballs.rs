use bevy::prelude::*; use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType}; use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d}; use bevy::prelude::Mesh2d; use bevy::input::ButtonInput; use bevy::input::keyboard::KeyCode; #[cfg(target_arch = "wasm32")] use std::sync::OnceLock; #[cfg(target_arch = "wasm32")] static METABALLS_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new(); #[cfg(target_arch = "wasm32")] static METABALLS_BEVEL_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new(); #[cfg(target_arch = "wasm32")] static METABALLS_UNIFIED_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new(); use crate::physics::clustering::cluster::Clusters; use crate::core::components::{Ball, BallRadius}; use crate::core::config::GameConfig; use crate::rendering::materials::materials::BallMaterialIndex; use crate::rendering::palette::palette::color_for_index; pub const MAX_BALLS: usize = 1024; pub const MAX_CLUSTERS: usize = 256; #[repr(C, align(16))] #[derive(Clone, Copy, ShaderType, Debug)] pub(crate) struct MetaballsUniform { v0: Vec4, v1: Vec4, v2: Vec4, balls: [Vec4; MAX_BALLS], cluster_colors: [Vec4; MAX_CLUSTERS], } impl Default for MetaballsUniform { fn default() -> Self { Self { v0: Vec4::new(0.0, 0.0, 1.0, 0.6), v1: Vec4::new(1.0, 1.0, 1.0, 0.0), v2: Vec4::new(0.0, 0.0, 0.0, 0.0), balls: [Vec4::ZERO; MAX_BALLS], cluster_colors: [Vec4::ZERO; MAX_CLUSTERS], } } } #[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)] pub struct MetaballsMaterial { #[uniform(0)] data: MetaballsUniform } impl MetaballsMaterial { #[cfg(feature = "debug")] pub fn set_debug_view(&mut self, view: u32) { self.data.v1.w = view as f32; } } impl Material2d for MetaballsMaterial { fn fragment_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs.wgsl".into() } } fn vertex_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs.wgsl".into() } } }
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)] pub struct MetaballsBevelMaterial { #[uniform(0)] data: MetaballsUniform }
impl Material2d for MetaballsBevelMaterial { fn fragment_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_BEVEL_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs_bevel.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs_bevel.wgsl".into() } } fn vertex_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_BEVEL_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs_bevel.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs_bevel.wgsl".into() } } }

// Unified material consolidating classic + bevel gray + bevel noise variants.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)] pub struct MetaballsUnifiedMaterial { #[uniform(0)] data: MetaballsUniform }
impl Material2d for MetaballsUnifiedMaterial { fn fragment_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_UNIFIED_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs_unified.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs_unified.wgsl".into() } } fn vertex_shader() -> ShaderRef { #[cfg(target_arch = "wasm32")] { return METABALLS_UNIFIED_SHADER_HANDLE.get().cloned().map(ShaderRef::Handle).unwrap_or_else(|| ShaderRef::Path("shaders/metaballs_unified.wgsl".into())); } #[cfg(not(target_arch = "wasm32"))] { "shaders/metaballs_unified.wgsl".into() } } }

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)] pub enum MetaballRenderMode { Classic, BevelGray, BevelNoise } impl MetaballRenderMode { pub const ALL: [Self;3] = [Self::Classic, Self::BevelGray, Self::BevelNoise]; }
#[derive(Resource, Debug)] pub struct MetaballMode { pub idx: usize } impl Default for MetaballMode { fn default() -> Self { Self { idx: 0 } } } impl MetaballMode { pub fn current(&self) -> MetaballRenderMode { MetaballRenderMode::ALL[self.idx % MetaballRenderMode::ALL.len()] } }

#[derive(Component)] pub struct MetaballsQuad; #[derive(Component)] pub struct MetaballsBevelQuad; #[derive(Component)] pub struct MetaballsUnifiedQuad; // unified shader quad
#[derive(Resource, Default)] pub struct MetaballsToggle(pub bool); #[derive(Resource, Debug, Clone)] pub struct MetaballsParams { pub iso: f32, pub normal_z_scale: f32, pub radius_multiplier: f32 } impl Default for MetaballsParams { fn default() -> Self { Self { iso: 0.6, normal_z_scale: 1.0, radius_multiplier: 1.0, } } }
pub struct MetaballsPlugin; impl Plugin for MetaballsPlugin { fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")] {
            use bevy::asset::Assets; use bevy::render::render_resource::Shader; let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let handle = shaders.add(Shader::from_wgsl(include_str!("../../../assets/shaders/metaballs.wgsl"), "metaballs_embedded.wgsl")); METABALLS_SHADER_HANDLE.get_or_init(|| handle.clone());
            let bevel_handle = shaders.add(Shader::from_wgsl(include_str!("../../../assets/shaders/metaballs_bevel.wgsl"), "metaballs_bevel_embedded.wgsl")); METABALLS_BEVEL_SHADER_HANDLE.get_or_init(|| bevel_handle.clone());
            let unified_handle = shaders.add(Shader::from_wgsl(include_str!("../../../assets/shaders/metaballs_unified.wgsl"), "metaballs_unified_embedded.wgsl")); METABALLS_UNIFIED_SHADER_HANDLE.get_or_init(|| unified_handle.clone());
        }
        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            .init_resource::<MetaballMode>()
            .add_plugins((Material2dPlugin::<MetaballsMaterial>::default(), Material2dPlugin::<MetaballsBevelMaterial>::default(), Material2dPlugin::<MetaballsUnifiedMaterial>::default(),))
            .add_systems(Startup, (initialize_toggle_from_config, apply_config_to_params, setup_metaballs))
            .add_systems(Update, (update_metaballs_material, update_metaballs_bevel_material, update_metaballs_unified_material, cycle_metaball_mode, apply_metaball_mode, resize_fullscreen_quad, tweak_metaballs_params)); } }

fn initialize_toggle_from_config(mut toggle: ResMut<MetaballsToggle>, cfg: Res<GameConfig>) { toggle.0 = cfg.metaballs_enabled; }
fn apply_config_to_params(mut params: ResMut<MetaballsParams>, cfg: Res<GameConfig>) { params.iso = cfg.metaballs.iso; params.normal_z_scale = cfg.metaballs.normal_z_scale; params.radius_multiplier = cfg.metaballs.radius_multiplier.max(0.0001); }
fn setup_metaballs(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<MetaballsMaterial>>, mut bevel_mats: ResMut<Assets<MetaballsBevelMaterial>>, mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>, windows: Query<&Window>) {
    let (w, h) = if let Ok(window) = windows.single() { (window.width(), window.height()) } else { (800.0, 600.0) };
    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));
    // Classic
    let mut mat = MetaballsMaterial::default(); mat.data.v2.x = w; mat.data.v2.y = h; let classic_handle = materials.add(mat);
    // Bevel (same initial uniform values to reuse update logic)
    let mut bmat = MetaballsBevelMaterial::default(); bmat.data.v2.x = w; bmat.data.v2.y = h; let bevel_handle = bevel_mats.add(bmat);
    // Unified (hidden initially until validated)
    let mut umat = MetaballsUnifiedMaterial::default(); umat.data.v2.x = w; umat.data.v2.y = h; let unified_handle = unified_mats.add(umat);
    // Spawn all, only classic visible initially
    commands.spawn((Mesh2d::from(mesh_handle.clone()), MeshMaterial2d(classic_handle), Transform::from_xyz(0.0, 0.0, 50.0), Visibility::Visible, MetaballsQuad));
    commands.spawn((Mesh2d::from(mesh_handle.clone()), MeshMaterial2d(bevel_handle), Transform::from_xyz(0.0, 0.0, 50.0), Visibility::Hidden, MetaballsBevelQuad));
    commands.spawn((Mesh2d::from(mesh_handle), MeshMaterial2d(unified_handle), Transform::from_xyz(0.0, 0.0, 50.0), Visibility::Hidden, MetaballsUnifiedQuad));
}
fn cycle_metaball_mode(mut mode: ResMut<MetaballMode>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::PageUp) { mode.idx = (mode.idx + 1) % MetaballRenderMode::ALL.len(); info!(target: "metaballs", "Render mode switched to {:?}", mode.current()); }
    if keys.just_pressed(KeyCode::PageDown) { mode.idx = (mode.idx + MetaballRenderMode::ALL.len() - 1) % MetaballRenderMode::ALL.len(); info!(target: "metaballs", "Render mode switched to {:?}", mode.current()); }
}
fn apply_metaball_mode(mode: Res<MetaballMode>, mut q_classic: Query<&mut Visibility, (With<MetaballsQuad>, Without<MetaballsBevelQuad>, Without<MetaballsUnifiedQuad>)>, mut q_bevel: Query<&mut Visibility, (With<MetaballsBevelQuad>, Without<MetaballsQuad>, Without<MetaballsUnifiedQuad>)>, mut q_unified: Query<&mut Visibility, (With<MetaballsUnifiedQuad>, Without<MetaballsQuad>, Without<MetaballsBevelQuad>)>, mut q_bg: Query<&mut Visibility, (With<crate::rendering::background::background::BackgroundQuad>, Without<MetaballsQuad>, Without<MetaballsBevelQuad>, Without<MetaballsUnifiedQuad>)>) {
    let current = mode.current();
    let Ok(mut vis_classic) = q_classic.single_mut() else { return; };
    let Ok(mut vis_bevel) = q_bevel.single_mut() else { return; };
    let Ok(mut vis_unified) = q_unified.single_mut() else { return; };
    match current {
        MetaballRenderMode::Classic => { *vis_classic = Visibility::Visible; *vis_bevel = Visibility::Hidden; *vis_unified = Visibility::Hidden; for mut v in q_bg.iter_mut() { *v = Visibility::Visible; } }
        MetaballRenderMode::BevelGray => { *vis_classic = Visibility::Hidden; *vis_bevel = Visibility::Visible; *vis_unified = Visibility::Hidden; for mut v in q_bg.iter_mut() { *v = Visibility::Hidden; } }
        MetaballRenderMode::BevelNoise => { *vis_classic = Visibility::Hidden; *vis_bevel = Visibility::Hidden; *vis_unified = Visibility::Visible; for mut v in q_bg.iter_mut() { *v = Visibility::Hidden; } }
    }
}
fn update_metaballs_material(clusters: Res<Clusters>, q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>, mut materials: ResMut<Assets<MetaballsMaterial>>, q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>, toggle: Res<MetaballsToggle>, params: Res<MetaballsParams>) { if !toggle.0 { return; } let Ok(handle_comp) = q_mat.single() else { return; }; let Some(mat) = materials.get_mut(&handle_comp.0) else { return; }; mat.data.v0.w = params.iso; mat.data.v1.x = params.normal_z_scale; mat.data.v1.y = 1.0; mat.data.v1.z = params.radius_multiplier.max(0.0001); let iso = params.iso.clamp(1e-4, 0.9999); let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt(); mat.data.v0.z = 1.0 / k; let mut color_count = 0usize; for cl in clusters.0.iter() { if color_count >= MAX_CLUSTERS { break; } let color = color_for_index(cl.color_index); let srgb = color.to_srgba(); mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0); color_count += 1; } mat.data.v0.y = color_count as f32; let mut ball_count = 0usize; for (tf, radius, color_idx) in q_balls.iter() { if ball_count >= MAX_BALLS { break; } let pos = tf.translation.truncate(); let mut cluster_slot = 0u32; for (i, cl) in clusters.0.iter().enumerate() { if cl.color_index == color_idx.0 { cluster_slot = i as u32; break; } } mat.data.balls[ball_count] = Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32); ball_count += 1; } mat.data.v0.x = ball_count as f32; }
fn update_metaballs_bevel_material(clusters: Res<Clusters>, q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>, mut materials: ResMut<Assets<MetaballsBevelMaterial>>, q_mat: Query<&MeshMaterial2d<MetaballsBevelMaterial>, With<MetaballsBevelQuad>>, toggle: Res<MetaballsToggle>, params: Res<MetaballsParams>) {
    if !toggle.0 { return; }
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };
    // reuse logic; keep in sync with classic (consider refactor if extended further)
    mat.data.v0.w = params.iso; mat.data.v1.x = params.normal_z_scale; mat.data.v1.y = 1.0; mat.data.v1.z = params.radius_multiplier.max(0.0001);
    let iso = params.iso.clamp(1e-4, 0.9999); let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt(); mat.data.v0.z = 1.0 / k;
    let mut color_count = 0usize; for cl in clusters.0.iter() { if color_count >= MAX_CLUSTERS { break; } let color = color_for_index(cl.color_index); let srgb = color.to_srgba(); mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0); color_count += 1; } mat.data.v0.y = color_count as f32;
    let mut ball_count = 0usize; for (tf, radius, color_idx) in q_balls.iter() { if ball_count >= MAX_BALLS { break; } let pos = tf.translation.truncate(); let mut cluster_slot = 0u32; for (i, cl) in clusters.0.iter().enumerate() { if cl.color_index == color_idx.0 { cluster_slot = i as u32; break; } } mat.data.balls[ball_count] = Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32); ball_count += 1; } mat.data.v0.x = ball_count as f32; }
fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_classic: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
    q_bevel: Query<&MeshMaterial2d<MetaballsBevelMaterial>, With<MetaballsBevelQuad>>,
    q_unified: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>,
    mut classic_mats: ResMut<Assets<MetaballsMaterial>>,
    mut bevel_mats: ResMut<Assets<MetaballsBevelMaterial>>,
    mut unified_mats: ResMut<Assets<MetaballsUnifiedMaterial>>,
) {
    let Ok(window) = windows.single() else { return; };
    if let Ok(handle_comp) = q_classic.single() { if let Some(mat) = classic_mats.get_mut(&handle_comp.0) { if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() { mat.data.v2.x = window.width(); mat.data.v2.y = window.height(); } } }
    if let Ok(handle_comp) = q_bevel.single() { if let Some(mat) = bevel_mats.get_mut(&handle_comp.0) { if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() { mat.data.v2.x = window.width(); mat.data.v2.y = window.height(); } } }
    if let Ok(handle_comp) = q_unified.single() { if let Some(mat) = unified_mats.get_mut(&handle_comp.0) { if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() { mat.data.v2.x = window.width(); mat.data.v2.y = window.height(); } } }
}
fn tweak_metaballs_params(mut params: ResMut<MetaballsParams>, input_map: Option<Res<crate::interaction::inputmap::types::InputMap>>) { if let Some(im) = input_map { let mut dirty = false; if im.just_pressed("MetaballIsoDec") { params.iso = (params.iso - 0.05).max(0.2); dirty = true; } if im.just_pressed("MetaballIsoInc") { params.iso = (params.iso + 0.05).min(1.5); dirty = true; } if dirty { info!("Metaballs params updated: iso={:.2}", params.iso); } } }

fn update_metaballs_unified_material(time: Res<Time>, mode: Res<MetaballMode>, clusters: Res<Clusters>, q_balls: Query<(&Transform, &BallRadius, &BallMaterialIndex), With<Ball>>, mut materials: ResMut<Assets<MetaballsUnifiedMaterial>>, q_mat: Query<&MeshMaterial2d<MetaballsUnifiedMaterial>, With<MetaballsUnifiedQuad>>, toggle: Res<MetaballsToggle>, params: Res<MetaballsParams>) {
    if !toggle.0 { return; }
    let Ok(handle_comp) = q_mat.single() else { return; };
    let Some(mat) = materials.get_mut(&handle_comp.0) else { return; };
    // Uniform field packing retained (struct size & alignment unchanged)
    mat.data.v0.w = params.iso; // iso
    mat.data.v1.x = params.normal_z_scale; // normal z scale
    mat.data.v1.y = match mode.current() { MetaballRenderMode::Classic => 0.0, MetaballRenderMode::BevelGray => 1.0, MetaballRenderMode::BevelNoise => 2.0 }; // render_mode selector
    mat.data.v1.z = params.radius_multiplier.max(0.0001); // radius multiplier (was already used similarly)
    mat.data.v2.z = time.elapsed_secs(); // animated time for noise background
    // Derived radius scale maintaining legacy behavior
    let iso = params.iso.clamp(1e-4, 0.9999); let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt(); mat.data.v0.z = 1.0 / k;
    // Cluster colors
    let mut color_count = 0usize; for cl in clusters.0.iter() { if color_count >= MAX_CLUSTERS { break; } let color = color_for_index(cl.color_index); let srgb = color.to_srgba(); mat.data.cluster_colors[color_count] = Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0); color_count += 1; } mat.data.v0.y = color_count as f32;
    // Balls
    let mut ball_count = 0usize; for (tf, radius, color_idx) in q_balls.iter() { if ball_count >= MAX_BALLS { break; } let pos = tf.translation.truncate(); let mut cluster_slot = 0u32; for (i, cl) in clusters.0.iter().enumerate() { if cl.color_index == color_idx.0 { cluster_slot = i as u32; break; } } mat.data.balls[ball_count] = Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32); ball_count += 1; } mat.data.v0.x = ball_count as f32;
}
