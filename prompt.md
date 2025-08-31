## Widget Gravity System Prompt (Attract / Repulse Widgets)

### Status
Draft v1 (to replace legacy center radial gravity) – Prepared 2025-08-31

### Purpose
You WILL replace the existing center-based `RadialGravityPlugin` with a generalized 2D "Widget" system starting with a Gravity Widget that can attract or repulse balls. These widgets are interactive, toggleable icons rendered on top of the metaballs fullscreen quad (separate draw, NOT baked into the metaballs shader) and can optionally have physical Rapier colliders. You WILL favor Rapier-native mechanisms (`Sensor` colliders, collision events, `ExternalForce`, `ExternalImpulse`) so Rapier performs force integration instead of manual velocity edits.

### High-Level Goals (Mapped to User Requirements)
1. Render Layering: You WILL render gravity widgets in front of the metaball quad (metaballs use z=50.0; widgets MUST use a higher z, e.g. ≥ 60.0).
2. Physics Component: You WILL optionally attach `Collider` + (optionally) `RigidBody::Fixed` (or no collider when transparent) to each widget for interaction / obstruction.
3. Attraction / Repulsion: You WILL apply per-widget forces by accumulating into each ball's `ExternalForce` component (creating it if missing) each frame before the physics step (in `PrePhysicsSet`) with configurable strength, falloff, polarity (attract / repulse), radius (influence), and enable flag. Direct `Velocity` mutation is legacy fallback only.
4. Tap Toggle: You WILL allow tapping / clicking a widget to toggle its `enabled` state. You WILL reuse the existing pointer → world coordinate logic pattern (see `interaction/cluster_pop::primary_pointer_world_pos`).

### Current Codebase Audit (Key Findings)
* Gravity now: `RadialGravityPlugin` (file: `src/physics/gravity/radial_gravity.rs`) applies a center-directed delta proportional to `|GameConfig.gravity.y|` for each `Ball` each frame (PrePhysicsSet). Global Rapier gravity is set to zero (`rapier_physics.rs`).
* Metaballs quad: Spawned at `Transform::from_xyz(0.0, 0.0, 50.0)` in `setup_metaballs` (`metaballs.rs`). Widgets MUST use higher z for visibility.
* Ball data: Balls carry `BallRadius`, `Velocity`, `Transform`, and appear in cluster structures. Force application currently just edits `Velocity.linvel` directly.
* Input & picking: `cluster_pop` module includes robust pointer world conversion and release detection (mouse & touch). Reusable patterns exist.
* System ordering: `PrePhysicsSet` is used for mutation before Rapier integration runs; we MUST insert widget gravity system inside this set (before or after cluster pop picking; ensure deterministic order). The radial gravity currently sits there; we will remove it.

### External Research (Bevy 0.16.0) Summary
```
Bevy Version: 0.16.0 (in repo submodule)
Relevant Modules: transform, input, picking (optional), sprite / mesh2d for 2D rendering
Ordering: Z (Transform.z) controls draw order for 2D Mesh2d / sprites within same pass
```
* Z layering for 2D content is standard: higher `Transform.z` → rendered above (within same pipeline).
* Pointer world conversion uses camera `viewport_to_world_2d`; already implemented in codebase.
* Mesh2d + `ColorMaterial` is simplest for icon quads (no custom shader required initially).

### Design Overview
You WILL introduce a generic Widget framework plus a specific `GravityWidget` implementation.

Components / Resources:
* `Widget` (marker) – common tag for all widgets.
* `GravityWidget` – data component:
  - `strength: f32` (base magnitude, positive number; direction determined by mode)
  - `mode: GravityMode` (enum: `Attract`, `Repulse`)
  - `radius: f32` (influence radius; 0 or <0 treated as infinite or a configured cap)
  - `falloff: Falloff` enum (e.g. `None`, `InverseLinear`, `InverseSquare`, `SmoothEdge`)
  - `enabled: bool`
  - `physics_collider: bool` (if true spawn collider; else none)
  - `id: WidgetId` (stable identifier for UI / debug overlay; can be a `u32`)
* `GravityWidgets` (Resource) – indexed collection or map of active gravity widget entities for quick iteration (optional; can be deferred until perf need arises).
* (Optional) `WidgetIcon` marker for visual child entity (decouple logic & visuals).

Enums:
```rust
pub enum GravityMode { Attract, Repulse }
pub enum Falloff { None, InverseLinear, InverseSquare, SmoothEdge }
```

Force Law (per-ball, per-widget):
```
dir = (widget_pos - ball_pos) for Attract; negate for Repulse
dist2 = dir.length_squared().max(eps)
dist = sqrt(dist2)
if radius > 0 && dist > radius => skip (outside influence)
base = strength
scale_by_falloff:
  None -> f = base
  InverseLinear -> f = base / (1 + dist)
  InverseSquare -> f = base / (1 + dist2)
  SmoothEdge -> let t = (1 - dist/radius).clamp(0,1); f = base * (t*t*(3-2*t))  // smoothstep
accel = dir.normalize() * f
vel.linvel += accel * dt
```
You WILL clamp per-frame delta to prevent extreme impulses: e.g. `if accel.length()*dt > max_delta => scale`. Provide a constant `MAX_GRAVITY_DELTA: f32 = 2000.0` (tune later).

### Rendering & Visuals
* You WILL spawn each widget visual as a `Mesh2d` rectangle or circle (e.g. `Circle` shape if available; else `Rectangle::new(icon_size, icon_size)`).
* You WILL use a distinct z-layer constant: `const WIDGET_Z: f32 = 80.0;` (≥ 60, higher than metaballs 50.0).
* You WILL optionally color-code mode: Attract = bluish, Repulse = reddish. Disabled = desaturated / low alpha.
* You WILL update visual color each frame if `enabled` changes; use change detection for efficiency.
* Future: Replace with texture atlas or 9-slice; keep prompt generic.

### Interaction (Tap / Click Toggle)
* You WILL detect pointer release using same pattern as `cluster_pop` (`buttons.just_released(MouseButton::Left)` OR any touch released) inside a system in `Update` within `PrePhysicsSet` ordering.
* You WILL compute world position via a shared utility. You WILL refactor `primary_pointer_world_pos` from `cluster_pop` into a shared module (e.g. `interaction::pointer::world.rs`) to avoid duplication.
* You WILL iterate gravity widgets; test hit: distance ≤ hit_radius (can reuse `radius` OR introduce `hit_radius` field; start with `radius`). Only consider topmost (or first) widget; for simplicity pick nearest.
* On hit: toggle `enabled` and emit optional event `WidgetToggled { id, enabled }` for debug overlay / UI.

### Physics Integration (Rapier-Centric)
* You WILL remove `RadialGravityPlugin` from `GamePlugin` and register a new `GravityWidgetsPlugin`.
* Rapier global gravity remains zero vector (already configured) – do not change.
* For finite radius widgets you WILL spawn a sensor collider (`Collider::ball(radius) + Sensor`). If a physical collider is also requested, you MAY spawn a second child collider or reuse one collider and accept physical interaction.
* You WILL process sensor collision events (Started/Stopped) to maintain influenced ball sets rather than iterating all balls each frame.
* You WILL accumulate per-widget forces into a scratch map then write / update `ExternalForce` components before Rapier's integration step (systems in `PrePhysicsSet`).
* Infinite radius (radius <= 0) fallback: treat all balls as influenced (brute-force) or clamp to a large constant radius documented in code.
* Optional impulse burst on enable toggle: insert `ExternalImpulse` once when transitioning disabled→enabled if configured.
* Performance: Sensor-driven sets avoid O(N_widgets * N_balls) unless infinite-radius fallback triggers.

### Configuration Changes (GameConfig)
You WILL extend `GameConfig` with a new section (e.g. `gravity_widgets`) that lists widgets or provides a default single widget. Minimal addition:
```ron
gravityWidgets: [
  ( id: 0, strength: 600.0, mode: "Attract", radius: 0.0, falloff: "InverseLinear", enabled: true ),
]
```
Rust side (add new types in `core::config::config.rs`):
```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GravityWidgetConfig {
  pub id: u32,
  pub strength: f32,
  pub mode: String,      // parse to enum after load
  pub radius: f32,
  pub falloff: String,   // parse to enum after load
  pub enabled: bool,
  pub physics_collider: bool,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
#[serde(default)]
pub struct GravityWidgetsConfig { pub widgets: Vec<GravityWidgetConfig>; }
```
Integrate into `GameConfig` (add field, default). You WILL validate: warn if strength ≤ 0, radius < 0 (treat as unlimited), unknown mode/falloff strings.

### Migration Notes
* You WILL mark previous `gravity.y` magnitude as legacy; optionally map it to default first widget `strength` if no `gravityWidgets` provided.
* You WILL produce a warning in validation when old gravity is non-zero and no widgets defined: "gravity.y deprecated; using as strength of implicit widget id=0".
* Remove or disable `apply_radial_gravity` system once widgets confirmed.

### Systems to Implement
1. `spawn_configured_gravity_widgets` (Startup): spawn widgets + sensor colliders (and optional physical colliders) + visuals.
2. `toggle_widget_on_tap` (Update, PrePhysicsSet): pointer release toggles enabled; optionally inserts `ExternalImpulse`.
3. `collect_widget_influences` (Update, PrePhysicsSet): process Rapier collision events to update influence sets.
4. `accumulate_widget_forces` (Update, PrePhysicsSet): compute & sum forces for influenced pairs.
5. `apply_accumulated_widget_forces` (Update, PrePhysicsSet): write aggregated force vectors into `ExternalForce` components (create if missing, clamp magnitude).
6. `update_widget_visuals` (optional): adjust color/alpha after state changes.
7. `debug_overlay_widgets` (optional): display widget states & force metrics.

### Ordering
```
Update Schedule:
  PrePhysicsSet:
    - toggle_widget_on_tap
    - apply_gravity_widgets (after toggle)
    - existing cluster_pop systems (ensure no unintended order conflicts)
```
You WILL add explicit `.after(toggle_widget_on_tap)` where needed.

### Events
* `WidgetToggled { id: u32, enabled: bool }`
* Future: `WidgetAdded`, `WidgetRemoved`, `WidgetModeChanged` (defer until needed).

### Data Integrity / Edge Cases
You WILL handle:
* Zero strength or disabled widget: skip without cost.
* Infinite radius (radius ≤ 0): apply globally.
* Ball exactly at widget position: skip or use safe direction (return early if `dist2 < 1e-6`).
* NaN / non-finite values: ignore (defensive checks already used in cluster_pop patterns).
* Extremely large strength: clamp per-frame delta.

### Testing Strategy
You WILL add unit tests:
* Force direction test: Attract vs Repulse sign in `ExternalForce.force`.
* Falloff scalar correctness for each enum variant.
* Toggle test: simulate click; enabled → force > 0 then disabled → force = 0 next frame.
* Migration test: no widgets but legacy gravity produces implicit widget force approximating previous radial gravity (tolerance-based comparison after one integration step).
You WILL add an integration test verifying ball velocity delta after one frame equals expected `ExternalForce.force * dt / mass` (within tolerance) when only a single attract widget is active.

### Performance Considerations
Initial naive iteration (widgets × balls) acceptable (< few thousand balls). For scalability:
* Short-circuit when `strength == 0 || !enabled`.
* Precompute squared radius for each widget at spawn (store `radius2` in component for fast compare).
* Optional broad-phase (grid or quad tree) only if profiling flags it.

### Debug / Telemetry
You WILL extend existing overlay to list active widgets: `widgets: N | id:mode:state:strength@radius`.
You WILL log (info target="widgets") on toggle: `WidgetToggled id=.. enabled=..`.

### Implementation Checklist
1. Add config structs + integrate into `GameConfig` + validation.
2. Introduce widget module: `src/interaction/widgets/gravity.rs` (or `src/physics/gravity/widgets.rs`).
3. Define components & enums.
4. Implement spawn system (config-driven; fallback from legacy gravity).
5. Implement toggle system (refactor pointer world pos util into shared module).
6. Implement force application system.
7. Remove `RadialGravityPlugin` from `GamePlugin` (add new `GravityWidgetsPlugin`).
8. Add tests.
9. Update debug overlay.
10. Document migration in `CHANGELOG.md` (add entry: Removed center radial gravity; introduced gravity widgets).

### Example Code Snippets
<!-- <example> -->
Spawn (simplified):
```rust
commands.spawn((
  Widget,
  GravityWidget { strength: 600.0, mode: GravityMode::Attract, radius: 0.0, falloff: Falloff::InverseLinear, enabled: true, physics_collider: false, id: 0 },
  Transform::from_xyz(200.0, 0.0, WIDGET_Z),
  GlobalTransform::default(),
));
```
Force application (core loop extract – Rapier force accumulation):
```rust
// accumulate_widget_forces system
for (widget_e, w_tf, gw) in q_widgets.iter() {
    if !gw.enabled || gw.strength <= 0.0 { continue; }
    let wpos = w_tf.translation.truncate();
    // influenced_balls: iterator of (Entity, Vec2 position)
    for (ball_e, bpos) in influenced_balls(widget_e, &q_ball_pos, &influence_sets) {
        let mut dir = wpos - bpos; // attract base vector
        let dist2 = dir.length_squared();
        if dist2 < 1e-8 { continue; }
        if gw.mode == GravityMode::Repulse { dir = -dir; }
        if gw.radius > 0.0 && dist2 > gw.radius * gw.radius { continue; }
        let dist = dist2.sqrt();
        let base = gw.strength; // interpret as force magnitude (N)
        let scalar = match gw.falloff {
            Falloff::None => base,
            Falloff::InverseLinear => base / (1.0 + dist),
            Falloff::InverseSquare => base / (1.0 + dist2),
            Falloff::SmoothEdge => if gw.radius > 0.0 { let t = (1.0 - dist/gw.radius).clamp(0.0,1.0); base * (t*t*(3.0-2.0*t)) } else { base },
        };
        let fvec = dir.normalize() * scalar;
        *temp_forces.entry(ball_e).or_insert(Vec2::ZERO) += fvec;
    }
}
// apply_accumulated_widget_forces system
for (ball_e, f) in temp_forces.drain() {
    let clamped = f.clamp_length_max(MAX_WIDGET_FORCE);
    if let Ok(mut ef) = q_ext_force.get_mut(ball_e) { ef.force = clamped; }
    else { commands.entity(ball_e).insert(ExternalForce { force: clamped, torque: 0.0 }); }
}
```
Toggle hitting (simplified):
```rust
if released { if let Some(world_pos) = pointer_world_pos(...) {
  let mut nearest: Option<(Entity,f32)> = None;
  for (e, tf, gw) in q_widgets.iter() {
    let d2 = tf.translation.truncate().distance_squared(world_pos);
    if d2 <= gw.radius.max(hit_radius_default).powi(2) {
      if nearest.map(|(_,bd2)| d2 < bd2).unwrap_or(true) { nearest = Some((e,d2)); }
    }
  }
  if let Some((entity,_)) = nearest { commands.entity(entity).try_insert(ChangedEnabledMarker); /* toggle */ }
}}
```
<!-- </example> -->

### Extensibility Roadmap
You WILL architect so further widget types (e.g., Wind, Portal, Field Modifier, Spawner) can reuse the base marker and toggle logic. Consider a `WidgetKind` enum or dynamic registration later; keep first iteration minimal.

### Risks & Mitigations
* Performance degradation with many widgets: mitigate with early-outs & radius checks.
* Visual overlap confusion: allow slight scaling or halo outline to indicate selection; add highlight on pointer hover (future).
* Legacy config conflicts: provide clear validation warnings & migration path.

### Success Criteria
You WILL consider this migration successful when:
* Legacy radial gravity system is fully removed from runtime.
* A single gravity widget reproduces prior center gravity behavior (verified via average inward acceleration parity).
* Toggling widget immediately removes or restores `ExternalForce` contribution next frame.
* Widgets render above metaballs and visually reflect enabled state.
* All unit & integration tests (force direction, falloff, toggle, migration) pass.

### Validation Plan (Manual)
1. Spawn single Attract widget at origin strength matching old gravity magnitude; compare average radial inward velocity vs previous build (rough parity).
2. Toggle off: confirm balls drift without central pull.
3. Add second Repulse widget offset right; confirm local deflection near that widget only.
4. Stress: 8 widgets placed around perimeter; profile frame time (target negligible increase).

---
END OF PROMPT
