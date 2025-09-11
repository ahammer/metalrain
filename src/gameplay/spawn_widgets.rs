use bevy::prelude::Mesh2d;
use bevy::prelude::*;
use bevy::sprite::{ColorMaterial, MeshMaterial2d};
use bevy_rapier2d::prelude::{Collider, Damping, Friction, Restitution, RigidBody, Velocity};
use rand::Rng;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::config::{GameConfig, SpawnWidgetConfig};
use crate::core::level::loader::LevelWidgets;
use crate::rendering::materials::materials::{
    BallDisplayMaterials, BallMaterialIndex, BallPhysicsMaterials,
};
use crate::rendering::metaballs::MetaballsUpdateSet; // for system ordering
use crate::rendering::sdf_atlas::SdfAtlas; // for random glyph assignment if loaded
use crate::rendering::palette::palette::BASE_COLORS; // for variant index length when not drawing circles

// Visual constants for spawn widgets (distinct from gravity widgets)
const SPAWN_WIDGET_Z: f32 = 82.0;
const SPAWN_WIDGET_ICON_RADIUS: f32 = 20.0;

#[derive(Component)]
pub struct SpawnWidget {
    pub id: u32,
    pub enabled: bool,
    pub cfg: SpawnWidgetConfig,
    pub timer: f32,
}

pub struct SpawnWidgetsPlugin;
impl Plugin for SpawnWidgetsPlugin {
    fn build(&self, app: &mut App) {
        // Run spawn widget instantiation after LevelLoader (which runs in Startup)
        // so that GameConfig.spawn_widgets.widgets is populated.
        app.add_systems(PostStartup, spawn_spawn_widgets)
            // Ensure spawns for this frame exist before metaballs / clustering update runs.
            .add_systems(
                Update,
                (toggle_spawn_widget_on_tap, run_spawn_widgets).before(MetaballsUpdateSet),
            );
    }
}

fn spawn_spawn_widgets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GameConfig>,
    level_widgets: Option<Res<LevelWidgets>>,
) {
    let widgets = cfg.spawn_widgets.widgets.clone();
    if widgets.is_empty() {
        warn!("SpawnWidgets: no spawn widgets present after LevelLoader; no balls will spawn.");
        return;
    }
    // Map id -> position from LevelWidgets (if present)
    for sw in widgets.into_iter() {
        let mut pos = Vec2::ZERO;
        if let Some(lw) = level_widgets.as_ref() {
            if let Some(sp) = lw.spawn_points.iter().find(|p| p.id == sw.id) {
                pos = sp.pos;
            }
        }
        spawn_single_spawn_widget(&mut commands, &mut meshes, &mut materials, sw, pos);
    }
}

fn spawn_single_spawn_widget(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    sw_cfg: SpawnWidgetConfig,
    pos: Vec2,
) {
    let mesh = meshes.add(Mesh::from(Circle {
        radius: SPAWN_WIDGET_ICON_RADIUS,
    }));
    let color = Color::srgba(0.25, 0.85, 0.35, 0.85);
    let mat = materials.add(color);
    // Prime timer so first spawn happens immediately on first Update frame.
    let interval = sw_cfg.spawn_interval;
    commands.spawn((
        SpawnWidget {
            id: sw_cfg.id,
            enabled: sw_cfg.enabled,
            cfg: sw_cfg,
            timer: interval,
        },
        Mesh2d::from(mesh),
        MeshMaterial2d(mat),
        Transform::from_xyz(pos.x, pos.y, SPAWN_WIDGET_Z),
        GlobalTransform::default(),
        Visibility::Visible,
    ));
}

fn toggle_spawn_widget_on_tap(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q_widgets: Query<(
        Entity,
        &Transform,
        &mut SpawnWidget,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let released =
        buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released {
        return;
    }
    let Ok(window) = windows_q.single() else {
        return;
    };
    let cursor = if let Some(t) = touches.iter().next() {
        t.position()
    } else {
        match window.cursor_position() {
            Some(c) => c,
            None => return,
        }
    };
    let (camera, cam_tf) = match camera_q.iter().next() {
        Some(c) => c,
        None => return,
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };
    let mut best: Option<(Entity, f32)> = None;
    for (e, tf, _sw, _mat) in q_widgets.iter_mut() {
        let pos = tf.translation.truncate();
        let d2 = pos.distance_squared(world_pos);
        let pick_r = SPAWN_WIDGET_ICON_RADIUS * 1.2;
        if d2 <= pick_r * pick_r && best.map(|(_, bd2)| d2 < bd2).unwrap_or(true) {
            best = Some((e, d2));
        }
    }
    if let Some((entity, _)) = best {
        if let Ok((_e, _tf, mut sw, mat_handle)) = q_widgets.get_mut(entity) {
            sw.enabled = !sw.enabled;
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                let base = (0.25, 0.85, 0.35);
                let alpha = if sw.enabled { 0.85 } else { 0.25 };
                mat.color = Color::srgba(base.0, base.1, base.2, alpha);
            }
        }
    }
}

fn run_spawn_widgets(
    time: Res<Time>,
    mut commands: Commands,
    mut q_widgets: Query<(&Transform, &mut SpawnWidget)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    cfg: Res<GameConfig>,
    display_palette: Option<Res<BallDisplayMaterials>>,
    physics_palette: Option<Res<BallPhysicsMaterials>>,
    q_ball_count: Query<Entity, With<Ball>>,
    mut spawn_ord: Option<ResMut<crate::rendering::sdf_atlas::BallSpawnOrdinal>>,
    sdf_atlas: Option<Res<SdfAtlas>>,
) {
    let total = q_ball_count.iter().len();
    if total >= cfg.spawn_widgets.global_max_balls {
        return;
    }
    let mut rng = rand::thread_rng();
    for (tf, mut sw) in q_widgets.iter_mut() {
        if !sw.enabled {
            continue;
        }
        sw.timer += time.delta_secs();
        if sw.timer < sw.cfg.spawn_interval {
            continue;
        }
        sw.timer = 0.0;
        let remaining_capacity = cfg
            .spawn_widgets
            .global_max_balls
            .saturating_sub(q_ball_count.iter().len());
        if remaining_capacity == 0 {
            break;
        }
        let batch = sw.cfg.batch.min(remaining_capacity);
        let base_pos = tf.translation.truncate();
        for _ in 0..batch {
            let ord = if let Some(ref mut so) = spawn_ord { let cur = so.0; so.0 += 1; cur } else { 0 };
            spawn_single_ball(
                &mut commands,
                &mut materials,
                &mut meshes,
                &sw.cfg,
                base_pos,
                &mut rng,
                &display_palette,
                &physics_palette,
                &cfg, // for bounce / physics params
                ord,
                sdf_atlas.as_ref().map(|a| &**a),
            );
        }
    }
}

fn spawn_single_ball(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    swc: &SpawnWidgetConfig,
    base_pos: Vec2,
    rng: &mut rand::rngs::ThreadRng,
    display_palette: &Option<Res<BallDisplayMaterials>>,
    _physics_palette: &Option<Res<BallPhysicsMaterials>>,
    game_cfg: &GameConfig,
    ordinal: u64,
    sdf_atlas: Option<&SdfAtlas>,
) {
    // Random radius & position in disk
    let r_ball = rng.gen_range(swc.ball_radius_min..swc.ball_radius_max);
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let radius = rng.gen_range(0.0..swc.area_radius);
    let offset = Vec2::new(angle.cos(), angle.sin()) * radius;
    let speed = rng.gen_range(swc.speed_min..swc.speed_max);
    let vel_dir = Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)).normalize_or_zero();
    let linvel = vel_dir * speed;
    // Variant (color) index selection (independent of visual rendering)
    let variant_idx = if let Some(disp) = display_palette.as_ref() {
        let len = disp.0.len().max(1);
        if len > 1 {
            rng.gen_range(0..len)
        } else {
            0
        }
    } else {
        // Fall back to base palette length
        let len = BASE_COLORS.len().max(1);
        if len > 1 {
            rng.gen_range(0..len)
        } else {
            0
        }
    };
    // Optional visual (circle mesh) only if draw_circles enabled
    let want_visual = game_cfg.draw_circles;
    let (maybe_mesh, maybe_material_handle) = if want_visual {
        let mesh_handle = meshes.add(Mesh::from(Circle { radius: r_ball }));
        // Choose material handle if palette exists; else random color
        let mat_handle = if let Some(disp) = display_palette.as_ref() {
            if variant_idx < disp.0.len() {
                disp.0[variant_idx].clone()
            } else {
                materials.add(Color::srgba(rng.gen(), rng.gen(), rng.gen(), 1.0))
            }
        } else {
            materials.add(Color::srgba(rng.gen(), rng.gen(), rng.gen(), 1.0))
        };
        (Some(mesh_handle), Some(mat_handle))
    } else {
        (None, None)
    };
    let world_pos = base_pos + offset;
    // Physics material properties
    let bounce = &game_cfg.bounce;
    // Random glyph (shape) index if atlas loaded & enabled.
    // Prefer curated subset (preferred_shapes) if non-empty for more controlled aesthetics.
    let shape_index: u16 = if let Some(atlas) = sdf_atlas {
        if atlas.enabled && atlas.shape_count > 0 {
            if !atlas.preferred_shapes.is_empty() {
                let idx = rng.gen_range(0..atlas.preferred_shapes.len());
                atlas.preferred_shapes[idx]
            } else {
                rng.gen_range(1..=atlas.shape_count as u32) as u16
            }
        } else { 0 }
    } else { 0 };
    let mut entity = commands.spawn((
        Ball,
        BallRadius(r_ball),
        crate::core::components::BallOrdinal(ordinal),
        BallMaterialIndex(variant_idx),
        crate::rendering::materials::materials::BallShapeIndex(shape_index),
        RigidBody::Dynamic,
        Velocity {
            linvel,
            angvel: 0.0,
        },
        Damping {
            linear_damping: bounce.linear_damping,
            angular_damping: bounce.angular_damping,
        },
        Restitution::coefficient(bounce.restitution),
        Friction::coefficient(bounce.friction),
        Collider::ball(r_ball),
        Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
        GlobalTransform::default(),
        Visibility::Visible,
    ));
    if let (Some(mesh_handle), Some(mat_handle)) = (maybe_mesh, maybe_material_handle) {
        entity.insert((Mesh2d::from(mesh_handle), MeshMaterial2d(mat_handle)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    #[test]
    fn basic_spawn_progress() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(GameConfig::default());
        app.init_resource::<Assets<ColorMaterial>>();
        app.init_resource::<Assets<Mesh>>();
        app.insert_resource(BallDisplayMaterials(vec![]));
        app.insert_resource(BallPhysicsMaterials(vec![]));
        let _ = app.world_mut().run_system_once(
            |mut commands: Commands,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<ColorMaterial>>| {
                let cfg = SpawnWidgetConfig::default();
                spawn_single_spawn_widget(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    cfg,
                    Vec2::ZERO,
                );
            },
        );
        app.add_systems(Update, run_spawn_widgets);
        app.insert_resource(Time::<()>::default());
        for _ in 0..10 {
            app.update();
        }
        // Count Ball components via a one-off system to avoid borrow issues.
        let ball_count = app
            .world_mut()
            .run_system_once(|q: Query<&Ball>| q.iter().count())
            .unwrap();
        assert!(
            ball_count > 0,
            "expected at least one Ball to be spawned, got {}",
            ball_count
        );
    }
}
