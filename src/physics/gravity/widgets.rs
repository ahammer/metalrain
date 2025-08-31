use bevy::prelude::*;
use bevy::prelude::Mesh2d;
use bevy::sprite::MeshMaterial2d;
use bevy::sprite::ColorMaterial;
use bevy_rapier2d::prelude::*;

use crate::core::components::Ball;
use crate::core::config::config::{GameConfig, GravityWidgetConfig};
use crate::core::system::system_order::PrePhysicsSet;

// ============================= Gravity (Attractor/Repulse) Widget Tunables =============================
// NOTE: These constants are intentionally namespaced to the gravity widget family (future: SPAWNER, TELEPORT, etc.)
// so we avoid generic names that will collide or cause ambiguity once more widget kinds are added.
/// Z-depth for gravity widgets (metaballs quad sits at z=50.0)
pub const GRAVITY_WIDGET_Z: f32 = 80.0;
/// Per-frame acceleration clamp (acts as safety for extreme strengths)
pub const GRAVITY_WIDGET_MAX_ACCEL: f32 = 25_000.0;
/// Global acceleration multiplier applied to configured strength for the first-generation Attractor/Repulse widget.
/// (Treat strength as a base accel; multiplier helps match legacy radial gravity feel.)
pub const ATTRACTOR_BASE_ACCEL_MULT: f32 = 150.0;
/// Icon & collider radius for current gravity widget visuals (future widgets may diverge with their own constants).
pub const GRAVITY_WIDGET_ICON_RADIUS: f32 = 24.0;

// Marker for any widget root entity
#[derive(Component)]
pub struct Widget;

// Gravity widget modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum GravityMode { Attract, Repulse }
impl GravityMode { pub fn from_str(s: &str) -> Option<Self> { match s { "Attract"=>Some(Self::Attract), "Repulse"=>Some(Self::Repulse), _=>None } } }

// Falloff law
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum Falloff { None, InverseLinear, InverseSquare, SmoothEdge }
impl Falloff { pub fn from_str(s: &str) -> Option<Self> { match s { "None"=>Some(Self::None), "InverseLinear"=>Some(Self::InverseLinear), "InverseSquare"=>Some(Self::InverseSquare), "SmoothEdge"=>Some(Self::SmoothEdge), _=>None } } }

#[derive(Component, Debug, Clone, Reflect)]
pub struct GravityWidget {
    pub id: u32,
    pub strength: f32,
    pub mode: GravityMode,
    pub radius: f32,      // <=0 => infinite
    pub falloff: Falloff,
    pub enabled: bool,
    pub physics_collider: bool,
    pub radius2: f32,     // cached squared radius (0 if infinite)
}
impl GravityWidget {
    fn from_config(c: &GravityWidgetConfig) -> Self {
        let mode = GravityMode::from_str(&c.mode).unwrap_or(GravityMode::Attract);
        let falloff = Falloff::from_str(&c.falloff).unwrap_or(Falloff::InverseLinear);
        let r2 = if c.radius > 0.0 { c.radius * c.radius } else { 0.0 };
        Self { id: c.id, strength: c.strength.max(0.0), mode, radius: c.radius, falloff, enabled: c.enabled, physics_collider: c.physics_collider, radius2: r2 }
    }
}

// Event on toggle (debug / overlay consumption)
#[derive(Event, Debug, Clone)]
pub struct WidgetToggled { pub id: u32, pub enabled: bool }

// Resource scratch map for per-frame force accumulation
#[derive(Resource, Default)]
struct AccumulatedForces(pub Vec<(Entity, Vec2)>); // reused buffer each frame

pub struct GravityWidgetsPlugin;
impl Plugin for GravityWidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<GravityWidget>()
            .add_event::<WidgetToggled>()
            .init_resource::<AccumulatedForces>()
            .add_systems(Startup, spawn_configured_gravity_widgets)
            .add_systems(Update, (
                toggle_widget_on_tap.in_set(PrePhysicsSet),
                accumulate_widget_forces.after(toggle_widget_on_tap).in_set(PrePhysicsSet),
                apply_accumulated_widget_forces.after(accumulate_widget_forces).in_set(PrePhysicsSet),
                update_widget_visuals.after(apply_accumulated_widget_forces),
            ));
    }
}

// Spawn widgets from config (or synthesize from legacy gravity.y if present and no widgets defined)
fn spawn_configured_gravity_widgets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GameConfig>,
) {
    let mut widgets = cfg.gravity_widgets.widgets.clone();
    if widgets.is_empty() && cfg.gravity.y.abs() > 0.0 {
        // Legacy migration: implicit single attract widget at origin using |gravity.y|
        widgets.push(GravityWidgetConfig { id: 0, strength: cfg.gravity.y.abs(), mode: "Attract".into(), radius: 0.0, falloff: "InverseLinear".into(), enabled: true, physics_collider: false, _parsed_ok: true });
        info!(target: "widgets", "Spawned implicit gravity widget from legacy gravity.y = {}", cfg.gravity.y);
    }
    for (i, wc) in widgets.iter().enumerate() {
    let mut gw = GravityWidget::from_config(wc);
    // Apply global multiplier so initial feel matches / exceeds legacy radial gravity
    gw.strength *= ATTRACTOR_BASE_ACCEL_MULT;
        let color = match (gw.mode, gw.enabled) {
            (GravityMode::Attract, true) => Color::srgba(0.2, 0.4, 0.95, 0.85),
            (GravityMode::Attract, false) => Color::srgba(0.2, 0.4, 0.95, 0.25),
            (GravityMode::Repulse, true) => Color::srgba(0.95, 0.25, 0.3, 0.85),
            (GravityMode::Repulse, false) => Color::srgba(0.95, 0.25, 0.3, 0.25),
        };
    let mesh = meshes.add(Mesh::from(Circle { radius: GRAVITY_WIDGET_ICON_RADIUS }));
        let mat = materials.add(color);
        let x = (i as f32) * 120.0; // simple spread for multiple widgets; user can reposition later
        let mut entity = commands.spawn((
            Widget,
            gw,
            Mesh2d::from(mesh.clone()),
            MeshMaterial2d(mat.clone()),
            Transform::from_xyz(x, 0.0, GRAVITY_WIDGET_Z),
            GlobalTransform::default(),
            Visibility::Visible,
        ));
        // Always spawn a collider (non-sensor) matching the visual so clicks do not "pass through" metaball quad
    // Collider matches icon size (no oversized padding) for intuitive hit detection.
    entity.insert(Collider::ball(GRAVITY_WIDGET_ICON_RADIUS));
    }
}

// --- Interaction (tap to toggle) ---------------------------------------------------
fn primary_pointer_world_pos(
    window: &Window,
    touches: &Touches,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    if let Some(t) = touches.iter().next() { return camera_q.iter().next()?.0.viewport_to_world_2d(camera_q.iter().next()?.1, t.position()).ok(); }
    let cursor = window.cursor_position()?;
    let (camera, cam_tf) = camera_q.iter().next()?;
    camera.viewport_to_world_2d(cam_tf, cursor).ok()
}

fn toggle_widget_on_tap(
    buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    windows_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut q_widgets: Query<(Entity, &Transform, &mut GravityWidget, &mut MeshMaterial2d<ColorMaterial>)>,
    mut ew: EventWriter<WidgetToggled>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let released = buttons.just_released(MouseButton::Left) || touches.iter_just_released().next().is_some();
    if !released { return; }
    let Ok(window) = windows_q.single() else { return; };
    let Some(world_pos) = primary_pointer_world_pos(window, &touches, &camera_q) else { return; };
    // Find nearest hit within radius (or default pick radius)
    let mut best: Option<(Entity, f32)> = None;
    for (e, tf, _gw, _mat) in q_widgets.iter_mut() {
        let pos = tf.translation.truncate();
        let d2 = pos.distance_squared(world_pos);
    let pick_r = GRAVITY_WIDGET_ICON_RADIUS * 1.15; // slight slack for pointer/touch accuracy
        if d2 <= pick_r * pick_r {
            if best.map(|(_,bd2)| d2 < bd2).unwrap_or(true) { best = Some((e,d2)); }
        }
    }
    if let Some((entity,_)) = best { if let Ok((_e,_tf, mut gw, mat_handle)) = q_widgets.get_mut(entity) {
    gw.enabled = !gw.enabled; // toggle
    debug!(target: "widgets", "toggle_widget_on_tap: entity={:?} id={} new_enabled={}", entity, gw.id, gw.enabled);
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let base = match gw.mode { GravityMode::Attract => (0.2,0.4,0.95), GravityMode::Repulse => (0.95,0.25,0.3) };
            let alpha = if gw.enabled { 0.85 } else { 0.25 }; mat.color = Color::srgba(base.0, base.1, base.2, alpha);
        }
        ew.write(WidgetToggled { id: gw.id, enabled: gw.enabled });
        info!(target: "widgets", "WidgetToggled id={} enabled={}", gw.id, gw.enabled);
    }}
}

// --- Force accumulation ------------------------------------------------------------
fn accumulate_widget_forces(
    time: Res<Time>,
    mut acc: ResMut<AccumulatedForces>,
    q_widgets: Query<(&Transform, &GravityWidget)>,
    mut q_balls: Query<(Entity, &Transform), With<Ball>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }
    acc.0.clear();
    // Naive iteration (widgets x balls). Optimization via influence sets deferred.
    for (w_tf, gw) in q_widgets.iter() {
        if !gw.enabled || gw.strength <= 0.0 { continue; }
        let wpos = w_tf.translation.truncate();
        for (ball_e, b_tf) in q_balls.iter_mut() {
            let bpos = b_tf.translation.truncate();
            let mut dir = wpos - bpos; // attract
            let dist2 = dir.length_squared();
            if dist2 < 1e-8 { continue; }
            if gw.mode == GravityMode::Repulse { dir = -dir; }
            if gw.radius2 > 0.0 && dist2 > gw.radius2 { continue; }
            let dist = dist2.sqrt();
            let base = gw.strength;
            let scalar = match gw.falloff {
                Falloff::None => base,
                Falloff::InverseLinear => base / (1.0 + dist),
                Falloff::InverseSquare => base / (1.0 + dist2),
                Falloff::SmoothEdge => if gw.radius > 0.0 { let t = (1.0 - dist / gw.radius).clamp(0.0,1.0); base * (t*t*(3.0-2.0*t)) } else { base },
            };
            let fvec = dir.normalize() * scalar; // treat as force directly (Rapier integrates)
            acc.0.push((ball_e, fvec));
        }
    }
}

fn apply_accumulated_widget_forces(
    time: Res<Time>,
    mut acc: ResMut<AccumulatedForces>,
    mut q_vel: Query<&mut Velocity>,
    mut commands: Commands,
    mut q_force: Query<&mut ExternalForce>,
) {
    if acc.0.is_empty() { return; }
    let dt = time.delta_secs();
    if dt <= 0.0 { acc.0.clear(); return; }
    use std::collections::HashMap;
    let mut summed: HashMap<Entity, Vec2> = HashMap::with_capacity(acc.0.len()/2 + 1);
    for (e, f) in acc.0.drain(..) { *summed.entry(e).or_insert(Vec2::ZERO) += f; }
    for (e, mut accel) in summed.into_iter() {
    if accel.length() > GRAVITY_WIDGET_MAX_ACCEL { accel = accel.normalize() * GRAVITY_WIDGET_MAX_ACCEL; }
        if let Ok(mut vel) = q_vel.get_mut(e) { vel.linvel += accel * dt; }
        if let Ok(mut ef) = q_force.get_mut(e) { ef.force = accel; } else { commands.entity(e).insert(ExternalForce { force: accel, torque: 0.0 }); }
    }
}

// Update visuals (currently handled in toggle; placeholder for future animations / enabled change detection)
fn update_widget_visuals() { /* no-op; reserved */ }

// --- Tests -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn attract_vs_repulse_direction() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<AccumulatedForces>();
        app.insert_resource(Time::<()>::default());
        // Spawn two widgets at origin: one attract, one repulse
        app.world_mut().spawn((Transform::from_xyz(0.0,0.0,0.0), GravityWidget { id:0, strength: 100.0, mode: GravityMode::Attract, radius:0.0, falloff: Falloff::None, enabled:true, physics_collider:false, radius2:0.0 }));
        app.world_mut().spawn((Transform::from_xyz(0.0,0.0,0.0), GravityWidget { id:1, strength: 100.0, mode: GravityMode::Repulse, radius:0.0, falloff: Falloff::None, enabled:true, physics_collider:false, radius2:0.0 }));
        // Single ball at x=100
        app.world_mut().spawn((Ball, Transform::from_xyz(100.0,0.0,0.0), GlobalTransform::default()));
        // Accumulate forces
        let _ = app.world_mut().run_system_once(accumulate_widget_forces);
        let acc = app.world().resource::<AccumulatedForces>();
        assert!(acc.0.len() == 2, "expected 2 force entries (one per widget) got {}", acc.0.len());
        let mut attract_force = None; let mut repulse_force = None;
        for (_e, f) in acc.0.iter() { if f.x > 0.0 { attract_force = Some(f.x); } else if f.x < 0.0 { repulse_force = Some(f.x); } }
        assert!(attract_force.is_some() && repulse_force.is_some(), "forces not found: {:?}", acc.0);
    }
}
