use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

use game_rendering::{
    BlendMode, CameraShakeCommand, CameraZoomCommand, CompositorSettings, GameCamera,
    GameRenderingPlugin, LayerBlendState, LayerToggleState, RenderLayer, RenderSurfaceSettings,
};
use game_assets::{GameAssetsPlugin, GameAssets};
use metaball_renderer::{
    MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin,
    MetaballShaderSourcePlugin,
};

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
struct BackgroundGlow;

#[derive(Component)]
struct EffectsPulse;

#[derive(Component)]
struct PerformanceOverlayText;

#[derive(Resource, Debug)]
struct PerformanceOverlayState {
    visible: bool,
    last_ui_update: f32,
}

impl Default for PerformanceOverlayState {
    fn default() -> Self {
        Self { visible: true, last_ui_update: 0.0 }
    }
}

#[derive(Resource, Debug, Default)]
struct FrameCounter {
    frame: u64,
}

fn main() {
    App::new()
        .insert_resource(RenderSurfaceSettings {
            base_resolution: UVec2::new(1280, 720),
            ..Default::default()
        })
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(MetaballShaderSourcePlugin)
        .add_plugins(DefaultPlugins)
    .add_plugins(GameAssetsPlugin::default())
    .add_plugins(GameRenderingPlugin)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-HALF_EXTENT, -HALF_EXTENT),
                    Vec2::new(HALF_EXTENT, HALF_EXTENT),
                ))
                .clustering_enabled(true)
                .with_presentation(true),
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .init_resource::<BurstForceState>()
        .init_resource::<WallPulseState>()
        .add_systems(Startup, (setup_scene, spawn_walls, spawn_balls))
        .init_resource::<PerformanceOverlayState>()
        .init_resource::<FrameCounter>()
        .add_systems(Startup, spawn_performance_overlay)
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
                animate_background_glow,
                animate_effect_overlay,
                update_performance_overlay,
                log_periodic_performance_snapshot,
            ),
        )
        .add_systems(PostStartup, configure_metaball_presentation)
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgba(0.05, 0.07, 0.13, 1.0),
            custom_size: Some(Vec2::new(1800.0, 1200.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -50.0),
        RenderLayers::layer(RenderLayer::Background.order()),
        Name::new("Background::Base"),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(0.45, 0.1, 0.75, 0.22),
            custom_size: Some(Vec2::new(1400.0, 1400.0)),
            ..Default::default()
        },
        Transform::from_xyz(0.0, 0.0, -40.0),
        RenderLayers::layer(RenderLayer::Background.order()),
        BackgroundGlow,
        Name::new("Background::Glow"),
    ));

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

    // Minimal UI layer placeholder (Sprint 3 cleanup)
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

fn spawn_performance_overlay(mut commands: Commands, assets: Res<GameAssets>) {
    let font = assets.fonts.ui_bold.clone();
    commands.spawn((
        Text2d::new("FPS: --\nFrame: 0\nFrame ms: --\nAvg1s: --  Avg5s: --\n(F1 to toggle)"),
        TextFont { font, font_size: 18.0, ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-600.0, 360.0, 500.0),
        RenderLayers::layer(RenderLayer::Ui.order()),
        Name::new("Ui::PerformanceOverlay"),
        PerformanceOverlayText,
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

fn animate_background_glow(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<BackgroundGlow>>,
) {
    let elapsed = time.elapsed_secs();
    for mut transform in &mut query {
        let scale = 1.0 + elapsed.sin() * 0.05;
        transform.scale = Vec3::splat(scale);
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

    // Zoom controls (- / +)
    if keys.just_pressed(KeyCode::Minus) {
        zoom_ev.write(CameraZoomCommand { delta_scale: -0.1 });
    }
    if keys.just_pressed(KeyCode::Equal) {
        zoom_ev.write(CameraZoomCommand { delta_scale: 0.1 });
    }

    // Camera shake trigger (Space)
    if keys.just_pressed(KeyCode::Space) {
        shake_ev.write(CameraShakeCommand { intensity: 12.0 });
    }

    // Exposure moved to bracket keys to free +/- for zoom
    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.exposure = (settings.exposure - 0.1).clamp(0.1, 3.0);
        info!(target: "compositor_demo", "Exposure set to {:.2}", settings.exposure);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.exposure = (settings.exposure + 0.1).clamp(0.1, 3.0);
        info!(target: "compositor_demo", "Exposure set to {:.2}", settings.exposure);
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
        info!(target: "compositor_demo", "Performance overlay {}", if overlay_state.visible { "shown" } else { "hidden" });
    }

    // Reset (R) restores camera defaults & exposure
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
}

fn update_performance_overlay(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut overlay_state: ResMut<PerformanceOverlayState>,
    mut text_query: Query<&mut Text2d, With<PerformanceOverlayText>>,
    mut frame_counter: ResMut<FrameCounter>,
) {
    frame_counter.frame += 1;
    if !overlay_state.visible { return; }
    let now = time.elapsed().as_secs_f32();
    if now - overlay_state.last_ui_update < 0.2 { return; }
    overlay_state.last_ui_update = now;

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_time_secs = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let avg_1s = fps; // placeholder
    let avg_5s = fps; // placeholder

    if let Some(mut text) = text_query.iter_mut().next() {
        text.0 = format!(
            "FPS: {fps:.1}\nFrame: {}\nFrame ms: {:.2}\nAvg1s: {avg_1s:.1}  Avg5s: {avg_5s:.1}\n(F1 to toggle)",
            frame_counter.frame,
            frame_time_secs * 1000.0
        );
    }
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
