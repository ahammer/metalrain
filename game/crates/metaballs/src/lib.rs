// Phase 7: Initial metaballs plugin scaffold (ported/minimized from legacy).
// Goals of this initial pass:
// - Provide toggle & params resources driven from GameConfigRes.
// - Define MetaballsMaterial with uniform buffer layout compatible with legacy shader.
// - Spawn a full‑screen quad using the metaballs material (Material2d).
// - Populate uniform each frame from Balls + (optionally) Clusters (if present).
// - Keep dependency surface small; we deliberately depend on bm_gameplay for Clusters
//   in this first pass even though long‑term architecture may extract a thin cluster
//   summary resource into bm_core or a shared crate. This can be refactored later.
//
// Deferred (future Phase 7 increments):
// - Advanced debug views (debug feature).
// - Color blend exponent variance.
// - GPU capture golden hash integration.
// - Performance optimizations (spatial culling, compute path).
// - WASM embedded shader handle indirection (use direct path for now).

use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")]
use bevy::render::render_resource::Shader;
#[cfg(target_arch = "wasm32")]
static WASM_METABALLS_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::sprite::{Material2d, Material2dPlugin, MeshMaterial2d};
use bevy::prelude::Mesh2d;

use bm_core::{Ball, BallRadius, BallColorIndex, GameConfigRes, PostPhysicsAdjustSet};
use bm_rendering::color_for_index;
#[cfg(feature = "golden")]
use bm_rendering::{GoldenPreimage, GoldenCaptureSet};
use bm_gameplay::Clusters;

pub const MAX_BALLS: usize = 1024;
pub const MAX_CLUSTERS: usize = 256;

#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug)]
struct MetaballsUniform {
    // v0: (ball_count, cluster_color_count, radius_scale, iso)
    v0: Vec4,
    // v1: (normal_z_scale, color_blend_exponent, radius_multiplier, debug_view)
    v1: Vec4,
    // v2: (window_size.x, window_size.y, reserved, reserved)
    v2: Vec4,
    balls: [Vec4; MAX_BALLS],
    cluster_colors: [Vec4; MAX_CLUSTERS],
}

impl Default for MetaballsUniform {
    fn default() -> Self {
        Self {
            v0: Vec4::new(0.0, 0.0, 1.0, 0.6),
            v1: Vec4::new(1.0, 1.0, 1.0, 0.0),
            v2: Vec4::ZERO,
            balls: [Vec4::ZERO; MAX_BALLS],
            cluster_colors: [Vec4::ZERO; MAX_CLUSTERS],
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct MetaballsMaterial {
    #[uniform(0)]
    data: MetaballsUniform,
}

impl Material2d for MetaballsMaterial {
    fn fragment_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            return WASM_METABALLS_SHADER_HANDLE
                .get()
                .cloned()
                .map(ShaderRef::Handle)
                .unwrap_or_else(|| ShaderRef::Path("shaders/metaballs.wgsl".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            "shaders/metaballs.wgsl".into()
        }
    }
    fn vertex_shader() -> ShaderRef {
        #[cfg(target_arch = "wasm32")]
        {
            return WASM_METABALLS_SHADER_HANDLE
                .get()
                .cloned()
                .map(ShaderRef::Handle)
                .unwrap_or_else(|| ShaderRef::Path("shaders/metaballs.wgsl".into()));
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            "shaders/metaballs.wgsl".into()
        }
    }
}

#[derive(Resource, Default)]
pub struct MetaballsToggle(pub bool);

#[derive(Resource, Debug, Clone)]
pub struct MetaballsParams {
    pub iso: f32,
    pub normal_z_scale: f32,
    pub radius_multiplier: f32,
}
impl Default for MetaballsParams {
    fn default() -> Self {
        Self {
            iso: 0.6,
            normal_z_scale: 1.0,
            radius_multiplier: 1.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct MetaballsDebugView(pub u32);

#[derive(Resource, Deref, DerefMut)]
pub struct MetaballsMaterialHandle(pub Handle<MetaballsMaterial>);

#[derive(Component)]
struct MetaballsQuad;

pub struct MetaballsPlugin;

impl Plugin for MetaballsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            // Embed shader on WASM (parity with legacy metaballs) to avoid separate asset fetch.
            let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
            let handle = shaders.add(Shader::from_wgsl(
                include_str!("../../assets/shaders/metaballs.wgsl"),
                "metaballs_embedded.wgsl",
            ));
            WASM_METABALLS_SHADER_HANDLE.get_or_init(|| handle.clone());
        }
        app.init_resource::<MetaballsToggle>()
            .init_resource::<MetaballsParams>()
            // Ensure asset collections exist even under reduced plugin sets (integration tests).
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<MetaballsMaterial>>()
            .add_plugins(Material2dPlugin::<MetaballsMaterial>::default())
            .add_systems(Startup, (apply_config_to_toggle_and_params, setup_metaballs))
            .add_systems(
                Update,
                (
                    // Ensure clusters are computed (in PostPhysicsAdjustSet) before we read them
                    // so uniform cluster_color_count is stable on the first frame after spawning.
                    update_metaballs_material.after(PostPhysicsAdjustSet),
                    resize_fullscreen_quad,
                    basic_param_tweaks,
                ),
            );

        // Contribute metaballs uniform summary bytes to golden hash preimage (if golden feature active).
        #[cfg(feature = "golden")]
        {
            use bevy::prelude::PostUpdate;
            app.add_systems(
                PostUpdate,
                contribute_golden_preimage
                    .before(GoldenCaptureSet)
            );
        }
    }
}

fn apply_config_to_toggle_and_params(
    mut toggle: ResMut<MetaballsToggle>,
    mut params: ResMut<MetaballsParams>,
    cfg: Res<GameConfigRes>,
) {
    toggle.0 = cfg.0.metaballs_enabled;
    params.iso = cfg.0.metaballs.iso;
    params.normal_z_scale = cfg.0.metaballs.normal_z_scale;
    params.radius_multiplier = cfg.0.metaballs.radius_multiplier.max(0.0001);
}

fn setup_metaballs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    windows: Query<&Window>,
) {
    let (w, h) = windows
        .iter()
        .next()
        .map(|win| (win.width(), win.height()))
        .unwrap_or((800.0, 600.0));

    let mesh_handle = meshes.add(Mesh::from(Rectangle::new(2.0, 2.0)));

    let mut mat = MetaballsMaterial::default();
    mat.data.v2.x = w;
    mat.data.v2.y = h;
    let material_handle = materials.add(mat);
    // Store handle for tests / future debug tooling without requiring ECS queries.
    commands.insert_resource(MetaballsMaterialHandle(material_handle.clone()));

    commands.spawn((
        Mesh2d::from(mesh_handle),
        MeshMaterial2d(material_handle),
        Transform::from_xyz(0.0, 0.0, 50.0),
        Visibility::Visible,
        MetaballsQuad,
    ));
}

fn update_metaballs_material(
    toggle: Res<MetaballsToggle>,
    params: Res<MetaballsParams>,
    debug_view: Option<Res<MetaballsDebugView>>,
    q_balls: Query<(&Transform, &BallRadius, &BallColorIndex), With<Ball>>,
    clusters: Option<Res<Clusters>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
    q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
) {
    if !toggle.0 {
        return;
    }
    let handle_comp = if let Some(h) = q_mat.iter().next() { h } else {
        return;
    };
    let Some(mat) = materials.get_mut(&handle_comp.0) else {
        return;
    };

    // Params
    mat.data.v0.w = params.iso;
    mat.data.v1.x = params.normal_z_scale;
    mat.data.v1.z = params.radius_multiplier;
    if let Some(view) = debug_view {
        mat.data.v1.w = view.0 as f32;
    }
    // radius_scale based on iso (see legacy derivation)
    let iso = params.iso.clamp(1e-4, 0.9999);
    let k = (1.0 - iso.powf(1.0 / 3.0)).max(1e-4).sqrt();
    mat.data.v0.z = 1.0 / k;

    // Cluster colors
    let mut cluster_color_count = 0usize;
    if let Some(clusters_ref) = clusters.as_ref() {
        for cl in clusters_ref.0.iter() {
            if cluster_color_count >= MAX_CLUSTERS {
                break;
            }
            let color = color_for_index(cl.color_index);
            let srgb = color.to_srgba();
            mat.data.cluster_colors[cluster_color_count] =
                Vec4::new(srgb.red, srgb.green, srgb.blue, 1.0);
            cluster_color_count += 1;
        }
    }
    mat.data.v0.y = cluster_color_count as f32;

    // Balls
    let mut ball_count = 0usize;
    for (tf, radius, color_idx) in q_balls.iter() {
        if ball_count >= MAX_BALLS {
            break;
        }
        let pos = tf.translation.truncate();
        // Map ball color to first matching cluster index (linear scan)
        let mut cluster_slot = 0u32;
        if let Some(clusters) = clusters.as_ref() {
            for (i, cl) in clusters.0.iter().enumerate() {
                if cl.color_index == color_idx.0 as usize {
                    cluster_slot = i as u32;
                    break;
                }
            }
        }
        mat.data.balls[ball_count] =
            Vec4::new(pos.x, pos.y, radius.0, cluster_slot as f32);
        ball_count += 1;
    }
    mat.data.v0.x = ball_count as f32;
}

fn resize_fullscreen_quad(
    windows: Query<&Window>,
    q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
    mut materials: ResMut<Assets<MetaballsMaterial>>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let handle_comp = if let Some(h) = q_mat.iter().next() { h } else {
        return;
    };
    if let Some(mat) = materials.get_mut(&handle_comp.0) {
        if mat.data.v2.x != window.width() || mat.data.v2.y != window.height() {
            mat.data.v2.x = window.width();
            mat.data.v2.y = window.height();
        }
    }
}

// Minimal tweak handler (same keybindings as legacy; optional).
/// Extended metaballs parameter tweak handler (Phase 7 incremental port).
/// Key bindings (headless-test friendly; avoid overlap with likely gameplay keys):
/// [ / ] : iso -/+ (clamped 0.2 .. 1.5) step 0.05
/// K / L : normal_z_scale -/+ (clamped 0.1 .. 5.0) step 0.1
/// Comma / Period : radius_multiplier -/+ (clamped 0.1 .. 5.0) step 0.1
/// R : reset all params to defaults
fn apply_param_key_events(params: &mut MetaballsParams, keys: &ButtonInput<KeyCode>) -> bool {
    // In tests (headless manual invocation) just_pressed semantics can be skipped due to
    // absence of frame progression; allow pressed() as a fallback trigger under #[cfg(test)].
    #[cfg(test)]
    let key_trigger = |k: KeyCode| keys.just_pressed(k) || keys.pressed(k);
    #[cfg(not(test))]
    let key_trigger = |k: KeyCode| keys.just_pressed(k);

    let mut dirty = false;

    // Iso surface threshold (treat simultaneous press as a single net action: prefer increment)
    let dec_iso = key_trigger(KeyCode::BracketLeft);
    let inc_iso = key_trigger(KeyCode::BracketRight);
    if inc_iso && !dec_iso {
        params.iso = (params.iso + 0.05).min(1.5);
        dirty = true;
    } else if dec_iso && !inc_iso {
        params.iso = (params.iso - 0.05).max(0.2);
        dirty = true;
    } else if inc_iso && dec_iso {
        // Both pressed: bias toward increment for deterministic net effect
        params.iso = (params.iso + 0.05).min(1.5);
        dirty = true;
    }

    // Normal Z scale (affects lighting normal reconstruction intensity)
    if key_trigger(KeyCode::KeyK) {
        params.normal_z_scale = (params.normal_z_scale - 0.1).max(0.1);
        dirty = true;
    }
    if key_trigger(KeyCode::KeyL) {
        params.normal_z_scale = (params.normal_z_scale + 0.1).min(5.0);
        dirty = true;
    }

    // Radius visual multiplier (expands/contracts influence radius for field composition)
    if key_trigger(KeyCode::Comma) {
        params.radius_multiplier = (params.radius_multiplier - 0.1).max(0.1);
        dirty = true;
    }
    if key_trigger(KeyCode::Period) {
        params.radius_multiplier = (params.radius_multiplier + 0.1).min(5.0);
        dirty = true;
    }

    // Reset
    if key_trigger(KeyCode::KeyR) {
        let d = MetaballsParams::default();
        params.iso = d.iso;
        params.normal_z_scale = d.normal_z_scale;
        params.radius_multiplier = d.radius_multiplier;
        dirty = true;
    }

    dirty
}

fn basic_param_tweaks(mut params: ResMut<MetaballsParams>, keys: Res<ButtonInput<KeyCode>>) {
    if apply_param_key_events(&mut params, &keys) {
        info!(
            "Metaballs params updated: iso={:.2} normal_z_scale={:.2} radius_mul={:.2}",
            params.iso, params.normal_z_scale, params.radius_multiplier
        );
    }
}

#[cfg(feature = "golden")]
fn contribute_golden_preimage(
    preimage: Option<ResMut<GoldenPreimage>>,
    q_mat: Query<&MeshMaterial2d<MetaballsMaterial>, With<MetaballsQuad>>,
    materials: Res<Assets<MetaballsMaterial>>,
) {
    use std::mem;

    let Some(mut preimage) = preimage else { return; };
    // Only write once (idempotent before capture).
    if !preimage.0.is_empty() {
        return;
    }
    let Some(handle_comp) = q_mat.iter().next() else { return; };
    let Some(mat) = materials.get(&handle_comp.0) else { return; };
    let data = &mat.data;

    // Deterministic compact summary:
    // version tag + counts + first N ball entries + first M cluster colors + key params.
    const VERSION: &[u8] = b"metaballs-u1";
    preimage.0.extend_from_slice(VERSION);

    let ball_count = data.v0.x as u32;
    let cluster_count = data.v0.y as u32;
    let iso = data.v0.w;
    let radius_scale = data.v0.z;
    let normal_z_scale = data.v1.x;
    let radius_mul = data.v1.z;

    preimage.0.extend_from_slice(&ball_count.to_le_bytes());
    preimage.0.extend_from_slice(&cluster_count.to_le_bytes());
    for f in [iso, radius_scale, normal_z_scale, radius_mul] {
        preimage.0.extend_from_slice(&f.to_le_bytes());
    }

    // Serialize up to 16 balls (x,y,radius,cluster_slot)
    let sample_balls = ball_count.min(16);
    preimage.0.extend_from_slice(&sample_balls.to_le_bytes());
    for i in 0..sample_balls as usize {
        let v = data.balls[i];
        for f in [v.x, v.y, v.z, v.w] {
            preimage.0.extend_from_slice(&f.to_le_bytes());
        }
    }

    // Serialize up to 8 cluster colors (r,g,b)
    let sample_clusters = cluster_count.min(8);
    preimage.0.extend_from_slice(&sample_clusters.to_le_bytes());
    for i in 0..sample_clusters as usize {
        let c = data.cluster_colors[i];
        for f in [c.x, c.y, c.z] {
            preimage.0.extend_from_slice(&f.to_le_bytes());
        }
    }

    // Final trailing checksum (simple xor of bytes) to detect truncation.
    let checksum = preimage.0.iter().fold(0u8, |acc, b| acc ^ b);
    preimage.0.push(checksum);

    // Ensure alignment invariance (length encoded already via length-prefix in golden harness).
    let _ = mem::size_of_val(&preimage.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_inits_resources() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        // Provide minimal resources required by the metaballs startup systems WITHOUT pulling full Render/DefaultPlugins:
        // - Assets<Mesh> for setup_metaballs (adds quad mesh)
        // - Assets<MetaballsMaterial> for material allocation
        // - AssetServer (needed by Material2dPlugin asset registration)
        // - InputPlugin for key resources used by tweak system
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        app.insert_resource(GameConfigRes(Default::default()));
        app.add_plugins(MetaballsPlugin);
        app.update();
        assert!(app.world().get_resource::<MetaballsToggle>().is_some());
        assert!(app.world().get_resource::<MetaballsParams>().is_some());
    }

    #[test]
    fn updates_uniform_with_ball_and_cluster() {
        use bm_core::{Ball, BallRadius, BallColorIndex};

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        // Enable metaballs via config resource
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        app.insert_resource(cfg);
        app.add_plugins(MetaballsPlugin);
        // Run startup (spawns quad + material)
        app.update();

        // Insert one ball
        app.world_mut().spawn((
            Ball,
            BallRadius(5.0),
            BallColorIndex(2),
            Transform::from_xyz(10.0, -4.0, 0.0),
            GlobalTransform::default(),
        ));

        // Run update systems (populate uniform)
        app.update();

        // Fetch material uniform via stored handle resource (simplifies borrow rules)
        {
            let world = app.world();
            let handle = world
                .get_resource::<MetaballsMaterialHandle>()
                .expect("MetaballsMaterialHandle present")
                .0
                .clone();
            let materials = world.resource::<Assets<MetaballsMaterial>>();
            let mat = materials.get(&handle).expect("material exists");
            // ball_count in v0.x
            assert_eq!(
                mat.data.v0.x as u32, 1,
                "expected 1 ball encoded in uniform"
            );
            // cluster_color_count in v0.y (no clusters inserted)
            assert_eq!(
                mat.data.v0.y as u32, 0,
                "expected 0 cluster colors encoded"
            );
            // First ball entry radius matches
            let ball_entry = mat.data.balls[0];
            assert!(
                (ball_entry.z - 5.0).abs() < 1e-4,
                "radius propagated"
            );
            // cluster slot assigned (0)
            assert_eq!(ball_entry.w as u32, 0, "cluster slot expected 0");
        }
    }

    #[test]
    fn param_tweak_keys_adjust_values() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        app.insert_resource(cfg);
        app.add_plugins(MetaballsPlugin);
        app.update(); // startup

        // Initial
        {
            let p = app.world().resource::<MetaballsParams>();
            assert!((p.iso - 0.6).abs() < 1e-5);
            assert!((p.normal_z_scale - 1.0).abs() < 1e-5);
            assert!((p.radius_multiplier - 1.0).abs() < 1e-5);
        }

        // Decrement iso (invoke helper directly to avoid schedule/input timing nuances under MinimalPlugins)
        {
            let world = app.world_mut();
            let keys_clone;
            {
                let mut keys_res = world.resource_mut::<ButtonInput<KeyCode>>();
                keys_res.press(KeyCode::BracketLeft);
                // Copy so borrow rules allow borrow sequencing (avoid simultaneous mutable borrows)
                keys_clone = keys_res.clone();
            }
            let mut params = world.resource_mut::<MetaballsParams>();
            // Manually apply after releasing keys_res
            super::apply_param_key_events(&mut params, &keys_clone);
        }
        {
            let p = app.world().resource::<MetaballsParams>();
            assert!((p.iso - 0.55).abs() < 1e-5, "iso decremented");
        }
        // Clear input state to avoid stale just_pressed from prior key before next simulated presses
        app.world_mut().insert_resource(ButtonInput::<KeyCode>::default());

        // Increment iso, increase normal_z_scale & radius_multiplier
        {
            let world = app.world_mut();
            let keys_clone;
            {
                let mut keys_res = world.resource_mut::<ButtonInput<KeyCode>>();
                // Release prior decrement key so both +/- are not simultaneously active (would net out).
                keys_res.release(KeyCode::BracketLeft);
                for k in [KeyCode::BracketRight, KeyCode::KeyL, KeyCode::Period] {
                    keys_res.press(k);
                }
                keys_clone = keys_res.clone();
            }
            let mut params = world.resource_mut::<MetaballsParams>();
            super::apply_param_key_events(&mut params, &keys_clone);
        }
        {
            let p = app.world().resource::<MetaballsParams>();
            assert!((p.iso - 0.60).abs() < 1e-5, "iso increment back");
            assert!((p.normal_z_scale - 1.1).abs() < 1e-5, "normal_z_scale incremented");
            assert!((p.radius_multiplier - 1.1).abs() < 1e-5, "radius_multiplier incremented");
        }

        // Lower bound clamp
        {
            let world = app.world_mut();
            {
                let mut params = world.resource_mut::<MetaballsParams>();
                params.normal_z_scale = 0.11;
                params.radius_multiplier = 0.11;
            }
            // Clear prior pressed keys (e.g., KeyL / Period) so only decrement keys influence this step
            world.insert_resource(ButtonInput::<KeyCode>::default());
            let keys_clone;
            {
                let mut keys_res = world.resource_mut::<ButtonInput<KeyCode>>();
                for k in [KeyCode::KeyK, KeyCode::Comma, KeyCode::KeyK, KeyCode::Comma] {
                    keys_res.press(k);
                }
                keys_clone = keys_res.clone();
            }
            let mut params = world.resource_mut::<MetaballsParams>();
            super::apply_param_key_events(&mut params, &keys_clone);
        }
        {
            let p = app.world().resource::<MetaballsParams>();
            assert!((p.normal_z_scale - 0.1).abs() < 1e-5, "normal_z_scale clamped at lower bound");
            assert!((p.radius_multiplier - 0.1).abs() < 1e-5, "radius_multiplier clamped at lower bound");
        }
    }

    #[test]
    fn param_tweak_reset_restores_defaults() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        app.insert_resource(cfg);
        app.add_plugins(MetaballsPlugin);
        app.update();

        // Mutate params away from defaults
        {
            let mut params = app.world_mut().resource_mut::<MetaballsParams>();
            params.iso = 1.2;
            params.normal_z_scale = 2.3;
            params.radius_multiplier = 3.4;
        }

        // Simulate reset via helper
        {
            let world = app.world_mut();
            let keys_clone;
            {
                let mut keys_res = world.resource_mut::<ButtonInput<KeyCode>>();
                keys_res.press(KeyCode::KeyR);
                keys_clone = keys_res.clone();
            }
            let mut params = world.resource_mut::<MetaballsParams>();
            super::apply_param_key_events(&mut params, &keys_clone);
        }

        let p = app.world().resource::<MetaballsParams>();
        let d = MetaballsParams::default();
        assert!((p.iso - d.iso).abs() < 1e-5, "iso reset");
        assert!((p.normal_z_scale - d.normal_z_scale).abs() < 1e-5, "normal_z_scale reset");
        assert!((p.radius_multiplier - d.radius_multiplier).abs() < 1e-5, "radius_multiplier reset");
    }

    #[test]
    fn debug_view_uniform_applied() {
        use bm_core::{Ball, BallRadius, BallColorIndex};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        app.insert_resource(cfg);
        app.add_plugins(MetaballsPlugin);
        // Insert debug view resource before update
        app.insert_resource(super::MetaballsDebugView(1));
        // Spawn one ball so material updates run meaningful path
        app.world_mut().spawn((
            Ball,
            BallRadius(3.0),
            BallColorIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        // Run startup + update
        app.update();
        app.update();

        let world = app.world();
        let handle = world
            .get_resource::<MetaballsMaterialHandle>()
            .expect("material handle")
            .0
            .clone();
        let materials = world.resource::<Assets<MetaballsMaterial>>();
        let mat = materials.get(&handle).expect("material exists");
        assert!(
            (mat.data.v1.w - 1.0).abs() < 1e-5,
            "debug view uniform slot (v1.w) should reflect MetaballsDebugView resource"
        );
    }

    #[test]
    #[ignore]
    fn perf_smoke_metaballs_300_frames() {
        use std::time::Instant;
        use bm_core::{Ball, BallRadius, BallColorIndex};

        // Build minimal app with metaballs enabled
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        app.insert_resource(cfg);
        app.add_plugins(MetaballsPlugin);
        app.update(); // startup (spawns quad & material)

        // Seed a moderate number of balls (exercise uniform population & loops)
        let ball_count = 200usize.min(MAX_BALLS);
        for i in 0..ball_count {
            let angle = i as f32 * 0.0314;
            let r = 150.0;
            let x = r * angle.cos();
            let y = r * angle.sin();
            app.world_mut().spawn((
                Ball,
                BallRadius(4.0),
                BallColorIndex((i % 8) as u8),
                Transform::from_xyz(x, y, 0.0),
                GlobalTransform::default(),
            ));
        }

        // Warmup frames (avoid first-frame shader/material setup noise)
        for _ in 0..10 {
            app.update();
        }

        const SAMPLE_FRAMES: usize = 300;
        let mut durations = Vec::with_capacity(SAMPLE_FRAMES);
        for _ in 0..SAMPLE_FRAMES {
            let start = Instant::now();
            app.update();
            durations.push(start.elapsed());
        }

        // Compute basic stats (ns)
        let mut nanos: Vec<f64> = durations
            .iter()
            .map(|d| d.as_secs_f64() * 1e9)
            .collect();
        nanos.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = nanos.iter().sum::<f64>() / nanos.len() as f64;
        let p95_idx = ((nanos.len() as f64) * 0.95).ceil() as usize - 1;
        let p95 = nanos[p95_idx];

        let json = format!(
            r#"{{"frames":{},"mean_ns":{},"p95_ns":{},"ball_count":{}}}"#,
            nanos.len(),
            mean as u64,
            p95 as u64,
            ball_count
        );
        println!("[perf_smoke] {}", json);

        // Optional file output (developer can set PERF_SMOKE_OUT env var)
        if let Ok(path) = std::env::var("PERF_SMOKE_OUT") {
            if let Err(e) = std::fs::write(&path, &json) {
                eprintln!("Failed to write PERF_SMOKE_OUT {}: {}", path, e);
            }
        }

        // This test is #[ignore] so it does not gate CI; assertions could be added later to
        // compare against stored baseline values.
    }

    #[test]
    fn uniform_stable_across_frames() {
        use bm_core::{Ball, BallRadius, BallColorIndex, GameConfigRes};
        use bm_gameplay::GameplayPlugin;
        use bm_core::CorePlugin;

        // Build app with gameplay (clusters) + metaballs.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Mesh>::default());
        app.insert_resource(Assets::<MetaballsMaterial>::default());
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
        ));

        // Config: enable metaballs, suppress initial ring spawn (balls.count = 0).
        let mut cfg = GameConfigRes(Default::default());
        cfg.0.metaballs_enabled = true;
        cfg.0.balls.count = 0;
        app.insert_resource(cfg);

        // Core + gameplay (spawns clustering systems) then metaballs.
        app.add_plugins(CorePlugin);
        app.add_plugins(GameplayPlugin);
        app.add_plugins(MetaballsPlugin);

        // Run startup (metaballs quad/material setup + gameplay spawning (0 balls)).
        app.update();

        // Spawn controlled deterministic set of balls forming two clusters:
        // Cluster A (color 0): two touching balls
        // Cluster B (color 1): singleton separated far away
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(0),
            Transform::from_xyz(20.0, 0.0, 0.0), // touching (r+r=20)
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(8.0),
            BallColorIndex(1),
            Transform::from_xyz(120.0, 40.0, 0.0),
            GlobalTransform::default(),
        ));

        // Helper to snapshot uniform relevant bytes deterministically.
        fn uniform_fingerprint(world: &World) -> Vec<u8> {
            let handle = world
                .get_resource::<MetaballsMaterialHandle>()
                .expect("MetaballsMaterialHandle present").0.clone();
            let materials = world.resource::<Assets<MetaballsMaterial>>();
            let mat = materials.get(&handle).expect("material exists");
            let data = &mat.data;
            let ball_count = data.v0.x as usize;
            let cluster_count = data.v0.y as usize;
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&(ball_count as u32).to_le_bytes());
            bytes.extend_from_slice(&(cluster_count as u32).to_le_bytes());
            // Params influencing encoding
            for f in [data.v0.z, data.v0.w, data.v1.x, data.v1.z] {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            // Balls (x,y,radius,cluster_slot)
            for i in 0..ball_count {
                let v = data.balls[i];
                for f in [v.x, v.y, v.z, v.w] {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
            }
            // Cluster colors (r,g,b) only (alpha constant 1.0)
            for i in 0..cluster_count {
                let c = data.cluster_colors[i];
                for f in [c.x, c.y, c.z] {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
            }
            bytes
        }

        // First update: clusters computed + uniform populated
        app.update();
        let snap1 = uniform_fingerprint(app.world());

        // Advance time & run additional frames (cluster persistence evolution should not reorder).
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.016));
        }
        app.update();
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.032));
        }
        app.update();

        let snap2 = uniform_fingerprint(app.world());

        assert_eq!(snap1, snap2, "metaballs uniform changed across frames without any topology or param mutations");
    }
}
