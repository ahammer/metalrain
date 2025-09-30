use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::view::RenderLayers;
use bevy::text::TextFont;
use bevy_rapier2d::prelude::*;
use game_core::{Ball, Wall, Target, TargetState, Hazard, HazardType, Paddle, SpawnPoint, Selected, SpawnBallEvent, ActiveSpawnRotation, BallSpawnPolicy, BallSpawnPolicyMode, PaddlePlugin, SpawningPlugin};
use metaball_renderer::{MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin};
use rand::Rng;
use game_core::{BallBundle, GameColor, GameCorePlugin};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use widget_renderer::WidgetRendererPlugin;
use game_rendering::{GameRenderingPlugin, RenderLayer};

pub const DEMO_NAME: &str = "physics_playground";

const ARENA_WIDTH: f32 = 512.0;
const ARENA_HEIGHT: f32 = 512.0;
const TEX_SIZE: UVec2 = UVec2::new(512, 512);

#[derive(Resource, Default)]
struct WallPlacement(Option<Vec2>);

#[derive(Resource, Default, Debug, Clone)]
struct PhysicsStats {
    body_count: usize,
    last_text: String,
}

pub fn run_physics_playground() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(AssetPlugin { file_path: "../../assets".into(), ..default() }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(GameCorePlugin)
        .add_plugins(PaddlePlugin)
        .add_plugins(SpawningPlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(WidgetRendererPlugin)
        .init_resource::<WallPlacement>()
        .init_resource::<PhysicsStats>();

    #[cfg(not(feature = "no-compositor"))]
    {
        app.add_plugins(GameRenderingPlugin)
            .add_plugins(MetaballRendererPlugin::with(
                MetaballRenderSettings::default()
                    .with_texture_size(TEX_SIZE)
                    .with_world_bounds(Rect::from_corners(
                        Vec2::new(-ARENA_WIDTH * 0.8, -ARENA_HEIGHT * 0.8),
                        Vec2::new(ARENA_WIDTH * 0.8, ARENA_HEIGHT * 0.8),
                    ))
                    .clustering_enabled(true)
                    // Route presentation quad into dedicated metaballs layer for compositor
                    .with_presentation(true)
                    .with_presentation_layer(RenderLayer::Metaballs.order() as u8),
            ));
    }

    #[cfg(feature = "no-compositor")]
    {
        // Legacy mode (no compositor) spawns direct camera & metaballs presentation directly to screen
        app.add_systems(Startup, spawn_legacy_camera)
            .add_plugins(MetaballRendererPlugin::with(
                MetaballRenderSettings::default()
                    .with_texture_size(TEX_SIZE)
                    .with_world_bounds(Rect::from_corners(
                        Vec2::new(-ARENA_WIDTH * 0.8, -ARENA_HEIGHT * 0.8),
                        Vec2::new(ARENA_WIDTH * 0.8, ARENA_HEIGHT * 0.8),
                    ))
                    .clustering_enabled(true)
                    .with_presentation(true),
            ));
    }

    app.add_systems(Startup, (setup_walls, spawn_initial_balls, spawn_hud, spawn_initial_spawnpoints, spawn_initial_paddle))
        .add_systems(
            Update,
            (
                handle_spawn_input,
                handle_paddle_spawn_input,
                handle_spawnpoint_activation_input,
                handle_world_element_input,
                handle_control_input,
                stress_test_trigger,
                update_config_text,
                handle_target_hits,
                handle_hazard_collisions,
                apply_spawn_policy_toggle,
                #[cfg(not(feature = "no-compositor"))]
                update_hud,
                #[cfg(feature = "no-compositor")]
                update_config_text_legacy_hud,
            ),
        )
        .run();
}

fn setup_walls(mut commands: Commands) {
    // Four static colliders forming a bounding box.
    let thickness = 20.0;
    let half_w = ARENA_WIDTH / 2.0;
    let half_h = ARENA_HEIGHT / 2.0;
    let walls = [
        // Floor
        (Vec2::new(0.0, -half_h), Vec2::new(half_w, thickness / 2.0)),
        // Ceiling
        (Vec2::new(0.0, half_h), Vec2::new(half_w, thickness / 2.0)),
        // Left
        (Vec2::new(-half_w, 0.0), Vec2::new(thickness / 2.0, half_h)),
        // Right
        (Vec2::new(half_w, 0.0), Vec2::new(thickness / 2.0, half_h)),
    ];
    for (center, half_extents) in walls {
        let mut e = commands.spawn((
            Transform::from_translation(center.extend(0.0)),
            GlobalTransform::IDENTITY,
            RigidBody::Fixed,
            Collider::cuboid(half_extents.x, half_extents.y),
            Name::new("WallSegment"),
        ));
        #[cfg(not(feature = "no-compositor"))]
        {
            e.insert(RenderLayers::layer(RenderLayer::GameWorld.order()));
        }
    }
}

#[cfg(feature = "no-compositor")]
fn spawn_legacy_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Name::new("PhysicsPlaygroundLegacyCamera")));
}

fn handle_spawn_input(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    config: Res<PhysicsConfig>,
    mut spawn_writer: EventWriter<SpawnBallEvent>,
    spawn_points: Query<(Entity, &Transform, &SpawnPoint)>,
) {
    if !buttons.just_pressed(MouseButton::Left) { return; }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let window = match windows.single().ok() { Some(w) => w, None => return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let (camera, cam_transform) = if let Ok(c) = cameras.single() { c } else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };

    if shift {
        // Spawn via nearest active spawn point (or create one temporarily)
        let mut nearest: Option<(Entity, f32)> = None;
        for (e, tf, sp) in &spawn_points {
            if !sp.active { continue; }
            let d2 = tf.translation.truncate().distance_squared(world_pos);
            if nearest.map(|(_, nd2)| d2 < nd2).unwrap_or(true) { nearest = Some((e, d2)); }
        }
        if let Some((e, _)) = nearest {
            spawn_writer.write(SpawnBallEvent { spawn_entity: e, override_position: None });
            return; // physics system will add body next frame
        }
        // Fallback: direct spawn if no active spawn point exists yet
    }
    spawn_ball(world_pos, &mut commands, &config, 0);
}

fn handle_paddle_spawn_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    if !keys.just_pressed(KeyCode::KeyP) { return; }
    let window = match windows.single().ok() { Some(w) => w, None => return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let (camera, cam_transform) = if let Ok(c) = cameras.single() { c } else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };
    commands.spawn((
        Paddle::default(),
        Transform::from_translation(world_pos.extend(0.2)),
        GlobalTransform::IDENTITY,
        Selected, // mark most recent for clarity
    ));
}

fn handle_spawnpoint_activation_input(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut rotation: ResMut<ActiveSpawnRotation>,
    mut spawns: Query<(Entity, &Transform, &mut SpawnPoint)>,
) {
    // S to create spawn point at cursor
    if keys.just_pressed(KeyCode::KeyS) {
        let window = match windows.single().ok() { Some(w) => w, None => return };
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, cam_transform)) = cameras.single() {
                if let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
                    let e = commands.spawn((
                        SpawnPoint::default(),
                        Transform::from_translation(world_pos.extend(0.1)),
                        GlobalTransform::IDENTITY,
                    )).id();
                    rotation.indices.push(e); // ensure selectable immediately
                }
            }
        }
    }
    // Q/E cycle
    if keys.just_pressed(KeyCode::KeyQ) { rotation.retreat(); }
    if keys.just_pressed(KeyCode::KeyE) { rotation.advance(); }
    // Number keys 1..9 select index (no Shift modifier needed now that digits are not used for layer toggles)
    for (i, code) in [KeyCode::Digit1,KeyCode::Digit2,KeyCode::Digit3,KeyCode::Digit4,KeyCode::Digit5,KeyCode::Digit6,KeyCode::Digit7,KeyCode::Digit8,KeyCode::Digit9].iter().enumerate() {
        if keys.just_pressed(*code) { rotation.set_index(i); }
    }
    // X toggles currently selected spawn active flag
    if keys.just_pressed(KeyCode::KeyX) {
        if let Some(cur) = rotation.current_entity() {
            if let Ok((_e,_tf, mut sp)) = spawns.get_mut(cur) { sp.active = !sp.active; }
        }
    }
}

fn apply_spawn_policy_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut policy: ResMut<BallSpawnPolicy>,
) {
    if keys.just_pressed(KeyCode::KeyA) { // 'A' toggles auto spawn interval 0.8s
        policy.mode = match policy.mode { BallSpawnPolicyMode::Manual => BallSpawnPolicyMode::Auto(0.8), BallSpawnPolicyMode::Auto(_) => BallSpawnPolicyMode::Manual };
    }
}

fn handle_world_element_input(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut wall_placement: ResMut<WallPlacement>,
    mut clear_q: Query<Entity, Or<(With<Wall>, With<Target>, With<Hazard>)>>,
) {
    let window = match windows.single().ok() { Some(w) => w, None => return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let (camera, cam_transform) = if let Ok(c) = cameras.single() { c } else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };

    // Right click: two-phase wall placement
    if buttons.just_pressed(MouseButton::Right) {
        if let Some(start) = wall_placement.0.take() {
            // finalize
            let end = world_pos;
            let thickness = 10.0;
            let wall = Wall::new(start, end, thickness, Color::srgb(0.6,0.7,0.9));
            let length = wall.length();
            let center = wall.center();
            let direction = (end - start).normalize_or_zero();
            let angle = direction.y.atan2(direction.x);
            commands.spawn((
                wall,
                Transform::from_translation(center.extend(0.0))
                    .with_rotation(Quat::from_rotation_z(angle)),
                GlobalTransform::IDENTITY,
                RigidBody::Fixed,
                Collider::cuboid(length/2.0, thickness/2.0),
            ));
        } else {
            wall_placement.0 = Some(world_pos);
        }
    }

    // Middle click: target
    if buttons.just_pressed(MouseButton::Middle) {
        let target = Target::new(3, 20.0, Color::srgb(0.9,0.9,0.3));
        commands.spawn((
            target,
            Transform::from_translation(world_pos.extend(0.1)),
            GlobalTransform::IDENTITY,
            Sensor,
            Collider::ball(20.0),
            ActiveEvents::COLLISION_EVENTS,
        ));
    }

    // H key: hazard
    if keys.just_pressed(KeyCode::KeyH) {
        let size = Vec2::new(80.0, 40.0);
        let bounds = Rect::from_center_size(world_pos, size);
        let hazard = Hazard::new(bounds, HazardType::Pit);
        commands.spawn((
            hazard,
            Transform::from_translation(world_pos.extend(-0.2)),
            GlobalTransform::IDENTITY,
            Sensor,
            Collider::cuboid(size.x/2.0, size.y/2.0),
        ));
    }

    // C key: clear all (walls, targets, hazards)
    if keys.just_pressed(KeyCode::KeyC) {
        for e in &mut clear_q { commands.entity(e).despawn(); }
        wall_placement.0 = None; // cancel pending wall start
    }
}

fn handle_control_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<PhysicsConfig>,
    mut commands: Commands,
    balls: Query<Entity, With<Ball>>,
) {
    // Arrow keys adjust gravity components continuously while held.
    let mut changed = false;
    if keys.pressed(KeyCode::ArrowUp) {
        config.gravity.y += 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        config.gravity.y -= 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        config.gravity.x -= 10.0;
        changed = true;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        config.gravity.x += 10.0;
        changed = true;
    }
    if changed {
        config.gravity = config
            .gravity
            .clamp(Vec2::splat(-1000.0), Vec2::splat(1000.0));
    }

    // +/- adjust clustering strength
    if keys.just_pressed(KeyCode::Equal) {
        config.clustering_strength = (config.clustering_strength + 10.0).min(500.0);
    }
    if keys.just_pressed(KeyCode::Minus) {
        config.clustering_strength = (config.clustering_strength - 10.0).max(0.0);
    }
    // [ ] adjust clustering radius
    if keys.just_pressed(KeyCode::BracketRight) {
        config.clustering_radius = (config.clustering_radius + 10.0).min(400.0);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        config.clustering_radius = (config.clustering_radius - 10.0).max(10.0);
    }

    if keys.just_pressed(KeyCode::KeyG) {
        if config.gravity.length_squared() > 0.0 {
            config.gravity = Vec2::ZERO;
        } else {
            config.gravity = Vec2::new(0.0, -500.0);
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        for e in &balls {
            commands.entity(e).despawn();
        }
    }
}

fn stress_test_trigger(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    balls: Query<&Transform, With<Ball>>,
    config: Res<PhysicsConfig>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }
    let mut rng = rand::thread_rng();
    let target = 60usize;
    let current = balls.iter().len();
    if current < target {
        for i in current..target {
            let x = rng.gen_range(-ARENA_WIDTH * 0.45..ARENA_WIDTH * 0.45);
            let y = rng.gen_range(-ARENA_HEIGHT * 0.45..ARENA_HEIGHT * 0.45);
            spawn_ball(Vec2::new(x, y), &mut commands, &config, (i % 4) as i32);
        }
    }
}

fn spawn_ball(
    position: Vec2,
    commands: &mut Commands,
    config: &PhysicsConfig,
    cluster: i32,
) -> Entity {
    let mut rng = rand::thread_rng();
    let radius = rng.gen_range(8.0..16.0);
    let color = match rng.gen_range(0..3) {
        0 => GameColor::Red,
        1 => GameColor::Green,
        _ => GameColor::Blue,
    };
    let mut bundle = BallBundle::new(position, radius, color);

    // Initial random velocity.
    let initial_velocity = Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(0.0..300.0));
    bundle.ball.velocity = initial_velocity;

    let mut entity_commands = commands.spawn((
        bundle,
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity { linvel: initial_velocity, angvel: 0.0 },
        Restitution { coefficient: config.ball_restitution, combine_rule: CoefficientCombineRule::Average },
        Friction { coefficient: config.ball_friction, combine_rule: CoefficientCombineRule::Average },
        ExternalForce::default(),
        Damping { linear_damping: 0.0, angular_damping: 1.0 },
        ActiveEvents::COLLISION_EVENTS,
        MetaBall { radius_world: radius },
        MetaBallColor(LinearRgba::new(0.8, 0.2, 0.2, 1.0)),
        MetaBallCluster(cluster),
        Name::new("Ball"),
    ));
    #[cfg(not(feature = "no-compositor"))]
    {
        // Balls are visually represented primarily via metaballs layer; optionally also tag for debug overlays.
        entity_commands.insert(RenderLayers::layer(RenderLayer::Metaballs.order()));
    }
    let entity = entity_commands.id();
    entity
}

// (Velocity gizmos temporarily removed pending color API alignment for Bevy 0.16)

/// Keep metaball centers in sync with physics-driven transforms.

#[derive(Component)]
struct ConfigText;

fn update_config_text(mut query: Query<&mut Text, With<ConfigText>>, config: Res<PhysicsConfig>) {
    if let Some(mut text) = query.iter_mut().next() {
        text.0 = format!(
            "Gravity: ({:.0},{:.0})  Cluster: str {:.0} rad {:.0}  Speed: min {:.0} max {:.0}\nKeys: LMB ball  RMB wall(2-click)  MMB target  H hazard  C clear  R reset balls  Arrows grav  +/- strength  [ ] radius  G toggle grav  T stress",
            config.gravity.x, config.gravity.y,
            config.clustering_strength, config.clustering_radius,
            config.min_ball_speed, config.max_ball_speed
        );
    }
}

/// Draw velocity vectors & optional clustering radius visualization.
#[allow(dead_code)]
fn draw_debug_gizmos(
    mut gizmos: Gizmos,
    balls: Query<(&Transform, &Velocity)>,
    config: Res<PhysicsConfig>,
) {
    let scale = 0.25; // shorten arrows for readability
    for (tr, vel) in &balls {
        let p = tr.translation.truncate();
        let v = vel.linvel * scale;
        let color = Color::WHITE;
        gizmos.line_2d(p, p + v, color);
        // Draw faint circle for clustering radius (could be heavy; sample subset)
        if balls.iter().len() <= 40 {
            // avoid overdraw spam at high counts
            gizmos.circle_2d(
                p,
                config.clustering_radius,
                Color::linear_rgba(0.5, 0.5, 0.5, 0.2),
            );
        }
    }
}

// world_to_tex & sync system removed – mapping now handled internally during packing.

fn spawn_initial_balls(mut commands: Commands, config: Res<PhysicsConfig>) {
    let mut rng = rand::thread_rng();
    for i in 0..20 {
        // seed some balls so screen isn't empty
        let x = rng.gen_range(-ARENA_WIDTH * 0.45..ARENA_WIDTH * 0.45);
        let y = rng.gen_range(-ARENA_HEIGHT * 0.45..ARENA_HEIGHT * 0.45);
        spawn_ball(Vec2::new(x, y), &mut commands, &config, (i % 4) as i32);
    }
}

fn spawn_initial_spawnpoints(mut commands: Commands) {
    // Provide a pair of default spawn points for experimentation
    let offsets = [-120.0_f32, 120.0_f32];
    for x in offsets {
        let mut entity = commands.spawn((
            SpawnPoint::default(),
            Transform::from_translation(Vec3::new(x, 0.0, 0.1)),
            GlobalTransform::IDENTITY,
            Name::new("SpawnPoint"),
        ));
        #[cfg(not(feature = "no-compositor"))]
        {
            entity.insert(RenderLayers::layer(RenderLayer::GameWorld.order()));
        }
    }
}

fn spawn_initial_paddle(mut commands: Commands) {
    // Spawn a default paddle centered near bottom of arena for immediate interaction
    let y = -ARENA_HEIGHT * 0.35;
    let mut entity = commands.spawn((
        Paddle::default(),
        Transform::from_translation(Vec3::new(0.0, y, 0.2)),
        GlobalTransform::IDENTITY,
        Selected,
        Name::new("InitialPaddle"),
    ));
    #[cfg(not(feature = "no-compositor"))]
    {
        entity.insert(RenderLayers::layer(RenderLayer::GameWorld.order()));
    }
}

#[derive(Component)]
struct HudText;

fn spawn_hud(mut commands: Commands) {
    // HUD text (world-space 2d) - shown in Ui layer under compositor, or directly in legacy mode.
    let mut entity = commands.spawn((
        Text2d::new("Initializing HUD..."),
        TextFont { font_size: 14.0, ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-ARENA_WIDTH * 0.9, ARENA_HEIGHT * 0.9, 500.0),
        HudText,
        ConfigText,
        Name::new("HudText"),
    ));
    #[cfg(not(feature = "no-compositor"))]
    {
        entity.insert(RenderLayers::layer(RenderLayer::Ui.order()));
    }
}

#[cfg(feature = "no-compositor")]
fn update_config_text_legacy_hud(mut query: Query<&mut Text2d, With<HudText>>, config: Res<PhysicsConfig>) {
    if let Some(mut text) = query.iter_mut().next() {
        text.0 = format!(
            "(Legacy Mode) Gravity ({:.0},{:.0})  Cluster str {:.0} rad {:.0}",
            config.gravity.x, config.gravity.y, config.clustering_strength, config.clustering_radius
        );
    }
}

// NEW: collision-driven target hit handling
fn handle_target_hits(
    mut collisions: EventReader<CollisionEvent>,
    mut targets: Query<&mut Target>,
    balls: Query<(), With<Ball>>,
) {
    for ev in collisions.read() {
        if let CollisionEvent::Started(a, b, _) = ev {
            let (target_entity, other) = if targets.get(*a).is_ok() { (*a, *b) } else if targets.get(*b).is_ok() { (*b, *a) } else { continue };
            if balls.get(other).is_ok() {
                if let Ok(mut tgt) = targets.get_mut(target_entity) {
                    if tgt.health > 0 {
                        tgt.health = tgt.health.saturating_sub(1);
                        tgt.state = if tgt.health == 0 { TargetState::Destroying(0.0) } else { TargetState::Hit(0.0) };
                    }
                }
            }
        }
    }
}

// NEW: collision-driven hazard effect (Pit clears balls)
fn handle_hazard_collisions(
    mut collisions: EventReader<CollisionEvent>,
    hazards: Query<&Hazard>,
    balls: Query<(), With<Ball>>,
    mut commands: Commands,
) {
    for ev in collisions.read() {
        if let CollisionEvent::Started(a, b, _) = ev {
            // Determine which (if any) is a hazard and which is a ball
            let a_hazard = hazards.get(*a).ok();
            let b_hazard = hazards.get(*b).ok();
            if let Some(h) = a_hazard {
                if balls.get(*b).is_ok() {
                    if matches!(h.hazard_type, HazardType::Pit) {
                        commands.entity(*b).despawn();
                    }
                }
            } else if let Some(h) = b_hazard {
                if balls.get(*a).is_ok() {
                    if matches!(h.hazard_type, HazardType::Pit) {
                        commands.entity(*a).despawn();
                    }
                }
            }
        }
    }
}

// ---------------- Simplified HUD (all layers always enabled) ----------------

#[cfg(not(feature = "no-compositor"))]
fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    balls: Query<Entity, With<Ball>>,
    mut stats: ResMut<PhysicsStats>,
    mut text_q: Query<&mut Text2d, With<HudText>>,
) {
    let Some(mut text) = text_q.iter_mut().next() else { return; };
    let body_count = balls.iter().len();
    stats.body_count = body_count;
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let new_text = format!(
        "Bodies:{}  FPS:{:.1}\nControls: LMB ball  RMB wall(2-click)  MMB target  H hazard  C clear  S spawn point  1..9 select spawn  Q/E cycle spawn  X toggle spawn  P paddle  A auto-spawn  Arrows grav  +/- strength  [ ] radius  G toggle grav  R reset balls  T stress",
        body_count, fps
    );
    if new_text != stats.last_text { text.0 = new_text.clone(); stats.last_text = new_text; }
}
