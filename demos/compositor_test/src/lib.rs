use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use std::collections::VecDeque;

use game_rendering::{
    BlendMode, CameraShakeCommand, CameraZoomCommand, CompositorSettings, GameCamera,
    GameRenderingPlugin, LayerBlendState, LayerToggleState, RenderLayer, RenderSurfaceSettings,
};
use game_assets::{GameAssets, GameAssetsPlugin};
use background_renderer::{BackgroundRendererPlugin, BackgroundConfig, BackgroundMode};
use metaball_renderer::{
    MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin,
};

pub const DEMO_NAME: &str = "compositor_test";

const HALF_EXTENT: f32 = 256.0;
const TEX_SIZE: UVec2 = UVec2::new(512, 512);
const WALL_THICKNESS: f32 = 10.0;
const NUM_BALLS: usize = 400;
const GRAVITY_SCALE: f32 = 0.0;

const BURST_INTERVAL_SECONDS: f32 = 3.0;
const BURST_ACTIVE_SECONDS: f32 = 0.6;
const BURST_RADIUS: f32 = 110.0;
const BURST_STRENGTH: f32 = 1400.0;

const WALL_PULSE_INTERVAL_SECONDS: f32 = 10.0;
const WALL_PULSE_ACTIVE_SECONDS: f32 = 0.8;
const WALL_PULSE_DISTANCE: f32 = 120.0;
const WALL_PULSE_STRENGTH: f32 = 2200.0;

#[derive(Resource, Debug)]
struct BurstForceState {
    interval_timer: Timer,
    active_timer: Option<Timer>,
    center: Vec2,
}

impl Default for BurstForceState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(BURST_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
            center: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Debug)]
struct WallPulseState {
    interval_timer: Timer,
    active_timer: Option<Timer>,
}

impl Default for WallPulseState {
    fn default() -> Self {
        Self {
            interval_timer: Timer::from_seconds(WALL_PULSE_INTERVAL_SECONDS, TimerMode::Repeating),
            active_timer: None,
        }
    }
}

#[derive(Component)]
struct EffectsPulse;

#[derive(Component)]
struct HudText;

#[derive(Resource, Debug)]
struct PerformanceOverlayState {
    visible: bool,
}

impl Default for PerformanceOverlayState { fn default() -> Self { Self { visible: true } } }

#[derive(Resource, Debug, Default)]
struct PerformanceStats {
    frames: u64,
    last_sample_time: f32,
    recent: VecDeque<(f32, f32)>, // (timestamp, dt)
}

#[derive(Resource, Debug, Default, Clone)]
struct LayerHudCache {
    last_enabled: [bool; 5],
    last_blends: [BlendMode; 5],
    last_exposure: f32,
    last_boundary_debug: bool,
    last_camera_scale: f32,
    last_text: String,
}

#[derive(Resource, Debug, Default)]
struct FrameCounter {
    frame: u64,
}

pub fn run_compositor_test() {
    App::new()
        .insert_resource(RenderSurfaceSettings {
            base_resolution: UVec2::new(1280, 720),
            ..Default::default()
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::DefaultPlugins.set(bevy::asset::AssetPlugin {
            file_path: "../../assets".into(),
            ..Default::default()
        }))
        .add_plugins(GameAssetsPlugin::default())
    .add_plugins(GameRenderingPlugin)
    .add_plugins(BackgroundRendererPlugin)
    .insert_resource(BackgroundConfig::default())
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-HALF_EXTENT, -HALF_EXTENT),
                    Vec2::new(HALF_EXTENT, HALF_EXTENT),
                ))
                .clustering_enabled(true)
                .with_presentation(true)
                .with_presentation_layer(RenderLayer::Metaballs.order() as u8),
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .add_systems(Startup, (setup_scene, spawn_walls, spawn_balls))
    .init_resource::<PerformanceOverlayState>()
    .init_resource::<PerformanceStats>()
    .init_resource::<LayerHudCache>()
    .init_resource::<FrameCounter>()
    .add_systems(Startup, spawn_hud)
        .add_systems(
            PreUpdate,
            (update_burst_force_state, apply_burst_forces).chain(),
        )
        .add_systems(
            Update,
            (
                update_wall_pulse_state,
                apply_wall_pulse_forces,
                handle_compositor_inputs,
                animate_effect_overlay,
                accumulate_performance_stats,
                update_hud,
                log_periodic_performance_snapshot,
            ),
        )
        .add_systems(PostStartup, configure_metaball_presentation)
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.06, 0.12, 0.20, 0.65),
            custom_size: Some(Vec2::splat(HALF_EXTENT * 2.0 + 32.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        RenderLayers::layer(RenderLayer::GameWorld.order()),
        Name::new("GameWorld::PlayfieldBackdrop"),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(0.2, 0.6, 1.0, 0.18),
            custom_size: Some(Vec2::new(620.0, 620.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, 30.0),
        RenderLayers::layer(RenderLayer::Effects.order()),
        EffectsPulse,
        Name::new("Effects::PulseOverlay"),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.06),
            custom_size: Some(Vec2::new(300.0, 80.0)),
            ..Default::default()
        },
        Transform::from_xyz(-480.0, 320.0, 200.0),
        RenderLayers::layer(RenderLayer::Ui.order()),
        Name::new("Ui::Placeholder"),
    ));
}

fn spawn_hud(mut commands: Commands, assets: Res<GameAssets>) {
    let font = assets.fonts.ui_bold.clone();
    commands.spawn((
        Text2d::new("HUD initializing..."),
        TextFont { font, font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-600.0, 360.0, 500.0),
        RenderLayers::layer(RenderLayer::Ui.order()),
        Name::new("Ui::Hud"),
        HudText,
    ));
}

fn configure_metaball_presentation(
    mut commands: Commands,
    mut done: Local<bool>,
    query: Query<(Entity, &Name), Without<RenderLayers>>,
) {
    if *done {
        return;
    }

    for (entity, name) in &query {
        if name.as_str() == "MetaballPresentationQuad" {
            commands
                .entity(entity)
                .insert(RenderLayers::layer(RenderLayer::Metaballs.order()));
            *done = true;
            info!(target: "compositor_demo", "Metaball presentation routed to Metaballs layer");
            break;
        }
    }
}

fn animate_effect_overlay(time: Res<Time>, mut query: Query<&mut Sprite, With<EffectsPulse>>) {
    let elapsed = time.elapsed_secs();
    for mut sprite in &mut query {
        let wave = (elapsed * 1.2).sin() * 0.5 + 0.5;
        sprite.color = sprite.color.with_alpha(0.12 + wave * 0.18);
    }
}

fn handle_compositor_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mut layer_state: ResMut<LayerToggleState>,
    mut blend_state: ResMut<LayerBlendState>,
    mut settings: ResMut<CompositorSettings>,
    mut shake_ev: EventWriter<CameraShakeCommand>,
    mut zoom_ev: EventWriter<CameraZoomCommand>,
    mut game_cam_q: Query<&mut GameCamera>,
    mut overlay_state: ResMut<PerformanceOverlayState>,
    mut bg_cfg: ResMut<BackgroundConfig>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        toggle_layer(&mut layer_state, RenderLayer::Background);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        toggle_layer(&mut layer_state, RenderLayer::GameWorld);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        toggle_layer(&mut layer_state, RenderLayer::Metaballs);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        toggle_layer(&mut layer_state, RenderLayer::Effects);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        toggle_layer(&mut layer_state, RenderLayer::Ui);
    }

    if keys.just_pressed(KeyCode::KeyQ) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Normal);
        info!(target: "compositor_demo", "Metaballs blend mode: Normal");
    }
    if keys.just_pressed(KeyCode::KeyW) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Additive);
        info!(target: "compositor_demo", "Metaballs blend mode: Additive");
    }
    if keys.just_pressed(KeyCode::KeyE) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Multiply);
        info!(target: "compositor_demo", "Metaballs blend mode: Multiply");
    }

    if keys.just_pressed(KeyCode::Minus) {
        zoom_ev.write(CameraZoomCommand { delta_scale: -0.1 });
    }
    if keys.just_pressed(KeyCode::Equal) {
        zoom_ev.write(CameraZoomCommand { delta_scale: 0.1 });
    }

    if keys.just_pressed(KeyCode::Space) {
        shake_ev.write(CameraShakeCommand { intensity: 12.0 });
    }

    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.exposure = (settings.exposure - 0.1).clamp(0.1, 3.0);
        info!(
            target: "compositor_demo",
            "Exposure set to {:.2}",
            settings.exposure
        );
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.exposure = (settings.exposure + 0.1).clamp(0.1, 3.0);
        info!(
            target: "compositor_demo",
            "Exposure set to {:.2}",
            settings.exposure
        );
    }
    if keys.just_pressed(KeyCode::F2) {
        settings.debug_layer_boundaries = !settings.debug_layer_boundaries;
        info!(
            target: "compositor_demo",
            "Layer boundary debug {}",
            if settings.debug_layer_boundaries {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    if keys.just_pressed(KeyCode::F1) {
        overlay_state.visible = !overlay_state.visible;
        info!(target: "compositor_demo", "HUD {}", if overlay_state.visible { "shown" } else { "hidden" });
    }

    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(mut cam) = game_cam_q.iter_mut().next() {
            cam.viewport_scale = 1.0;
            cam.target_viewport_scale = 1.0;
            cam.shake_intensity = 0.0;
            cam.shake_offset = Vec2::ZERO;
        }
        settings.exposure = 1.0;
        info!(target: "compositor_demo", "Camera & exposure reset");
    }

    if keys.just_pressed(KeyCode::KeyB) {
        bg_cfg.mode = bg_cfg.mode.next();
        info!(target: "compositor_demo", "Background mode -> {:?}", bg_cfg.mode);
    }
    if keys.pressed(KeyCode::KeyA) { bg_cfg.angle += 0.9 * 0.016; }
    if keys.pressed(KeyCode::KeyD) { bg_cfg.angle -= 0.9 * 0.016; }
    if matches!(bg_cfg.mode, BackgroundMode::RadialGradient) {
        let mut changed = false;
        if keys.pressed(KeyCode::ArrowLeft) { bg_cfg.radial_center.x -= 0.25 * 0.016; changed = true; }
        if keys.pressed(KeyCode::ArrowRight){ bg_cfg.radial_center.x += 0.25 * 0.016; changed = true; }
        if keys.pressed(KeyCode::ArrowUp)   { bg_cfg.radial_center.y += 0.25 * 0.016; changed = true; }
        if keys.pressed(KeyCode::ArrowDown) { bg_cfg.radial_center.y -= 0.25 * 0.016; changed = true; }
        if changed { bg_cfg.radial_center = bg_cfg.radial_center.clamp(Vec2::ZERO, Vec2::splat(1.0)); }
    }
}

fn accumulate_performance_stats(
    time: Res<Time>,
    mut stats: ResMut<PerformanceStats>,
    mut frame_counter: ResMut<FrameCounter>,
) {
    frame_counter.frame += 1;
    stats.frames = frame_counter.frame;
    let now = time.elapsed().as_secs_f32();
    let dt = time.delta().as_secs_f32();
    stats.last_sample_time = now;
    stats.recent.push_back((now, dt));
    while let Some((t, _)) = stats.recent.front() { if now - *t > 6.0 { stats.recent.pop_front(); } else { break; } }
}

fn compute_fps_windows(stats: &PerformanceStats) -> (f32, f32) {
    let now = stats.last_sample_time;
    let mut count_1s = 0u32; let mut time_1s = 0.0;
    let mut count_5s = 0u32; let mut time_5s = 0.0;
    for (t, dt) in stats.recent.iter().rev() {
        let age = now - *t;
        if age <= 1.0 { count_1s += 1; time_1s += *dt; }
        if age <= 5.0 { count_5s += 1; time_5s += *dt; } else { break; }
    }
    let fps_1s = if time_1s > 0.0 { count_1s as f32 / time_1s } else { 0.0 };
    let fps_5s = if time_5s > 0.0 { count_5s as f32 / time_5s } else { 0.0 };
    (fps_1s, fps_5s)
}

fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    layer_toggles: Res<LayerToggleState>,
    blend_state: Res<LayerBlendState>,
    settings: Res<CompositorSettings>,
    overlay_state: Res<PerformanceOverlayState>,
    mut cache: ResMut<LayerHudCache>,
    stats: Res<PerformanceStats>,
    cam_q: Query<&GameCamera>,
    mut text_q: Query<&mut Text2d, With<HudText>>,
    entities_layers: Query<&RenderLayers>,
) {
    let mut text = if let Some(t) = text_q.iter_mut().next() { t } else { return };
    if !overlay_state.visible { text.0 = "(HUD hidden - F1)".to_string(); return; }

    let mut enabled = [true;5];
    for cfg in &layer_toggles.configs { enabled[cfg.layer.order()] = cfg.enabled; }
    let mut blends = [BlendMode::Normal;5];
    for layer in RenderLayer::ALL { blends[layer.order()] = blend_state.blend_for(layer); }
    let camera_scale = cam_q.iter().next().map(|c| c.viewport_scale).unwrap_or(1.0);
    let (fps_1s, fps_5s) = compute_fps_windows(&stats);
    let fps_instant = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(fps_1s as f64) as f32;
    let frame_time_ms = (diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) * 1000.0) as f32;
    let mut counts = [0usize;5];
    for rl in &entities_layers {
        for (i, _) in RenderLayer::ALL.iter().enumerate() {
            if rl.intersects(&RenderLayers::layer(i)) { counts[i] += 1; }
        }
    }
    let mut needs_rebuild = false;
    if enabled != cache.last_enabled { needs_rebuild = true; cache.last_enabled = enabled; }
    if blends != cache.last_blends { needs_rebuild = true; cache.last_blends = blends; }
    if (settings.exposure - cache.last_exposure).abs() > f32::EPSILON { needs_rebuild = true; cache.last_exposure = settings.exposure; }
    if settings.debug_layer_boundaries != cache.last_boundary_debug { needs_rebuild = true; cache.last_boundary_debug = settings.debug_layer_boundaries; }
    if (camera_scale - cache.last_camera_scale).abs() > 1e-4 { needs_rebuild = true; cache.last_camera_scale = camera_scale; }
    if !needs_rebuild { return; }
    let layer_lines: String = RenderLayer::ALL.iter().enumerate().map(|(i,l)| {
        let en = if enabled[i] { "ON " } else { "OFF" };
        let blend = match blends[i] { BlendMode::Normal=>"N", BlendMode::Additive=>"A", BlendMode::Multiply=>"M" };
        format!("[{}] {:10} {:3} | Ent:{:4} | B:{}\n", i+1, l.label(), en, counts[i], blend)
    }).collect();
    let exposure = settings.exposure;
    let boundaries = if settings.debug_layer_boundaries { "ON" } else { "OFF" };
    let hud = format!(
        "Layers:\n{layer_lines}FPS: {fps_instant:.1} (1s:{fps_1s:.1} 5s:{fps_5s:.1})\nFrame: {}  {:.2} ms\nExposure: {exposure:.2}  Boundaries:{boundaries}\nZoom: {camera_scale:.2}x  (F1 HUD 1-5 Layers Q/W/E Blend +/- Zoom [ ] Exposure F2 Bounds R Reset)",
        stats.frames, frame_time_ms
    );
    cache.last_text = hud.clone();
    text.0 = hud;
}

fn log_periodic_performance_snapshot(
    diagnostics: Res<DiagnosticsStore>,
    frame_counter: Res<FrameCounter>,
) {
    if frame_counter.frame == 0 || frame_counter.frame % 600 != 0 { return; }
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) * 1000.0;
    info!(target: "perf_snapshot", "Frame {} | FPS {:.2} | Frame {:.2} ms", frame_counter.frame, fps, frame_time_ms);
}

fn toggle_layer(state: &mut LayerToggleState, layer: RenderLayer) {
    if let Some(config) = state.config_mut(layer) {
        config.enabled = !config.enabled;
        info!(
            target: "compositor_demo",
            "{} layer {}",
            layer,
            if config.enabled { "enabled" } else { "disabled" }
        );
    }
}

fn spawn_walls(mut commands: Commands) {
    let hx = HALF_EXTENT;
    let hy = HALF_EXTENT;
    let t = WALL_THICKNESS;

    let horizontal_size = Vec2::new((hx + t) * 2.0, t * 2.0);
    let vertical_size = Vec2::new(t * 2.0, (hy + t) * 2.0);

    commands
        .spawn((
            Name::new("WallBottom"),
            RigidBody::Fixed,
            Collider::cuboid(hx + t, t),
            Transform::from_translation(Vec3::new(0.0, -hy - t, 0.0)),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite {
                    color: Color::srgba(0.25, 0.45, 0.7, 0.45),
                    custom_size: Some(horizontal_size),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(RenderLayer::GameWorld.order()),
            ));
        });

    commands
        .spawn((
            Name::new("WallTop"),
            RigidBody::Fixed,
            Collider::cuboid(hx + t, t),
            Transform::from_translation(Vec3::new(0.0, hy + t, 0.0)),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite {
                    color: Color::srgba(0.25, 0.45, 0.7, 0.45),
                    custom_size: Some(horizontal_size),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(RenderLayer::GameWorld.order()),
            ));
        });

    commands
        .spawn((
            Name::new("WallLeft"),
            RigidBody::Fixed,
            Collider::cuboid(t, hy + t),
            Transform::from_translation(Vec3::new(-hx - t, 0.0, 0.0)),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite {
                    color: Color::srgba(0.25, 0.45, 0.7, 0.45),
                    custom_size: Some(vertical_size),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(RenderLayer::GameWorld.order()),
            ));
        });

    commands
        .spawn((
            Name::new("WallRight"),
            RigidBody::Fixed,
            Collider::cuboid(t, hy + t),
            Transform::from_translation(Vec3::new(hx + t, 0.0, 0.0)),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite {
                    color: Color::srgba(0.25, 0.45, 0.7, 0.45),
                    custom_size: Some(vertical_size),
                    ..Default::default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLayers::layer(RenderLayer::GameWorld.order()),
            ));
        });
}

fn spawn_balls(mut commands: Commands) {
    let mut rng = StdRng::from_entropy();
    let palette = [
        LinearRgba::new(1.0, 0.3, 0.3, 1.0),
        LinearRgba::new(0.3, 1.0, 0.4, 1.0),
        LinearRgba::new(0.3, 0.4, 1.0, 1.0),
        LinearRgba::new(1.0, 0.9, 0.3, 1.0),
    ];

    for i in 0..NUM_BALLS {
        let radius = rng.gen_range(3.0..12.0);
        let x = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let y = rng.gen_range(-HALF_EXTENT + radius..HALF_EXTENT - radius);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(10.0..120.0);
        let vel = Vec2::from_angle(angle) * speed;
        let cluster = (i % palette.len()) as i32;
        let base_color = palette[cluster as usize];
        let sprite_color =
            Color::linear_rgba(base_color.red, base_color.green, base_color.blue, 0.2);

        let mut entity = commands.spawn((
            Sprite {
                color: sprite_color,
                custom_size: Some(Vec2::splat(radius * 2.0)),
                ..Default::default()
            },
            Transform::from_translation(Vec3::new(x, y, 0.0)),
            RenderLayers::layer(RenderLayer::GameWorld.order()),
            Name::new(format!("Ball#{i}")),
        ));

        entity.insert((
            RigidBody::Dynamic,
            Collider::ball(radius),
            Restitution::coefficient(0.8),
            Damping {
                linear_damping: 0.2,
                angular_damping: 0.8,
            },
            Velocity {
                linvel: vel,
                angvel: rng.gen_range(-5.0..5.0),
            },
            GravityScale(GRAVITY_SCALE),
            Ccd::disabled(),
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
        ));

        entity.insert((
            MetaBall {
                radius_world: radius,
            },
            MetaBallColor(base_color),
            MetaBallCluster(cluster),
            ExternalForce::default(),
        ));
    }

    info!("Spawned {NUM_BALLS} balls in compositor demo");
}

fn update_burst_force_state(time: Res<Time>, mut state: ResMut<BurstForceState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() {
            state.active_timer = None;
        }
    }
    if state.interval_timer.just_finished() {
        let mut rng = thread_rng();
        let margin = BURST_RADIUS * 0.5;
        let x = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        let y = rng.gen_range(-HALF_EXTENT + margin..HALF_EXTENT - margin);
        state.center = Vec2::new(x, y);
        state.active_timer = Some(Timer::from_seconds(BURST_ACTIVE_SECONDS, TimerMode::Once));
        info!("Burst force activated at ({x:.1},{y:.1})");
    }
}

fn apply_burst_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<BurstForceState>,
) {
    let Some(active) = state.active_timer.as_ref() else {
        return;
    };
    if active.finished() {
        return;
    }
    let center = state.center;
    let r2 = BURST_RADIUS * BURST_RADIUS;
    for (tr, mut force) in &mut q {
        let pos = tr.translation.truncate();
        let to_ball = pos - center;
        let dist2 = to_ball.length_squared();
        if dist2 > r2 || dist2 < 1.0 {
            continue;
        }
        let dist = dist2.sqrt();
        let falloff = 1.0 - (dist / BURST_RADIUS);
        let dir = to_ball / dist;
        force.force += dir * BURST_STRENGTH * falloff;
    }
}

fn update_wall_pulse_state(time: Res<Time>, mut state: ResMut<WallPulseState>) {
    state.interval_timer.tick(time.delta());
    if let Some(active) = state.active_timer.as_mut() {
        active.tick(time.delta());
        if active.finished() {
            state.active_timer = None;
        }
    }
    if state.interval_timer.just_finished() {
        state.active_timer = Some(Timer::from_seconds(
            WALL_PULSE_ACTIVE_SECONDS,
            TimerMode::Once,
        ));
        info!("Wall repulsion pulse active");
    }
}

fn apply_wall_pulse_forces(
    mut q: Query<(&Transform, &mut ExternalForce), With<RigidBody>>,
    state: Res<WallPulseState>,
) {
    let Some(active) = state.active_timer.as_ref() else {
        return;
    };
    if active.finished() {
        return;
    }
    let max_dist = WALL_PULSE_DISTANCE;
    for (tr, mut force) in &mut q {
        let p = tr.translation.truncate();
        let mut accum = Vec2::ZERO;

        let left_d = (p.x - (-HALF_EXTENT)).max(0.0);
        if left_d < max_dist {
            let f = 1.0 - left_d / max_dist;
            accum.x += f;
        }
        let right_d = (HALF_EXTENT - p.x).max(0.0);
        if right_d < max_dist {
            let f = 1.0 - right_d / max_dist;
            accum.x -= f;
        }
        let bottom_d = (p.y - (-HALF_EXTENT)).max(0.0);
        if bottom_d < max_dist {
            let f = 1.0 - bottom_d / max_dist;
            accum.y += f;
        }
        let top_d = (HALF_EXTENT - p.y).max(0.0);
        if top_d < max_dist {
            let f = 1.0 - top_d / max_dist;
            accum.y -= f;
        }
        if accum.length_squared() > 0.0001 {
            let magnitude = accum.length();
            let dir = accum / magnitude;
            force.force += dir * WALL_PULSE_STRENGTH * magnitude;
        }
    }
}
