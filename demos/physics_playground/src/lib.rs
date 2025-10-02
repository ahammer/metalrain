use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::*;
use rand::Rng;

use game_core::{ArenaConfig, BallBundle, GameColor, GameCorePlugin};
use game_physics::{GamePhysicsPlugin, PhysicsConfig};
use game_rendering::{GameRenderingPlugin, RenderLayer};
use metaball_renderer::{
    MetaBall, MetaBallCluster, MetaBallColor, MetaballRenderSettings, MetaballRendererPlugin,
};
use event_core::*;
use event_core::EventFlowSet;
use widget_renderer::WidgetRendererPlugin;

pub const DEMO_NAME: &str = "physics_playground";

const ARENA_WIDTH: f32 = 512.0;
const ARENA_HEIGHT: f32 = 512.0;
const TEX_SIZE: UVec2 = UVec2::new(512, 512);

// Resources for playground state
#[derive(Resource, Default)]
pub struct PlaygroundState {
    pub cursor_world_pos: Option<Vec2>,
    pub selected_entity: Option<Entity>,
    pub ball_cluster_counter: i32,
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct InputDiagnostics { pub dropped_clicks: u32 }

#[derive(Resource, Clone)]
pub struct BallSpawnPreset {
    pub color: GameColor,
    pub radius_range: (f32, f32),
    pub speed_range: (f32, f32),
}

impl Default for BallSpawnPreset {
    fn default() -> Self {
        Self {
            color: GameColor::Red,
            radius_range: (10.0, 18.0),
            speed_range: (80.0, 180.0),
        }
    }
}

// Marker components
#[derive(Component)]
struct InstructionsText;

#[derive(Component)]
struct PlacementPreview;

#[derive(Component)]
struct DynamicEntity;

pub fn run_physics_playground() {
    let mut km = KeyMappingMiddleware::empty();
    km.map(KeyCode::Space, KeyMappingOutput::Game(GameEvent::SpawnBallAtCursor { position: Vec2::ZERO }))
      .map(KeyCode::KeyC, KeyMappingOutput::Game(GameEvent::ClearArena))
      .map(KeyCode::KeyR, KeyMappingOutput::Game(GameEvent::ResetLevel))
      .map(KeyCode::KeyP, KeyMappingOutput::Game(GameEvent::TogglePhysics))
      .map(KeyCode::Digit1, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::SpawnBall }))
      .map(KeyCode::Digit2, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceWall }))
      .map(KeyCode::Digit3, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceTarget }))
      .map(KeyCode::Digit4, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceHazard }))
      .map(KeyCode::Digit5, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlacePaddle }))
      .map(KeyCode::Digit6, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::PlaceSpawnPoint }))
      .map(KeyCode::Digit7, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::Select }))
      .map(KeyCode::Digit8, KeyMappingOutput::Game(GameEvent::ChangeTool { mode: PlaygroundMode::Delete }));

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "../../assets".into(),
            ..default()
        }))
        .add_plugins(GameCorePlugin)
        .add_plugins(GamePhysicsPlugin)
        .add_plugins(GameRenderingPlugin)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_texture_size(TEX_SIZE)
                .with_world_bounds(Rect::from_corners(
                    Vec2::new(-ARENA_WIDTH * 0.5, -ARENA_HEIGHT * 0.5),
                    Vec2::new(ARENA_WIDTH * 0.5, ARENA_HEIGHT * 0.5),
                ))
                .clustering_enabled(true)
                .with_presentation(true)
                .with_presentation_layer(RenderLayer::Metaballs.order() as u8),
        ))
        .add_plugins(WidgetRendererPlugin)
        .add_plugins(EventCorePlugin::default())
    .init_resource::<PlaygroundState>()
    .init_resource::<InputDiagnostics>()
        .init_resource::<BallSpawnPreset>()
        .insert_resource(PlaygroundMode::default())
        .register_middleware(km)
        .register_middleware(DebounceMiddleware::new(2))
        .register_handler(BallSpawnHandler)
        .register_handler(WidgetPlacementHandler)
        .register_handler(SelectionHandler)
        .register_handler(DeletionHandler)
        .register_handler(ClearArenaHandler)
        .register_handler(ToolChangeHandler)
        .register_handler(PhysicsToggleHandler)
        .add_systems(Startup, (setup_board, setup_camera, spawn_instructions_overlay, spawn_test_ball))
        // Order-sensitive: ensure cursor position updates before click handling (and preview) every frame.
        .add_systems(Update, (
            // InputCollect set
            track_mouse_position.in_set(EventFlowSet::InputCollect),
            handle_key_input.in_set(EventFlowSet::InputCollect),
            // InputProcess set
            handle_mouse_clicks.in_set(EventFlowSet::InputProcess),
            // UIUpdate set
            (preview_widget_placement, update_instructions_overlay, highlight_selected_entity)
                .in_set(EventFlowSet::UIUpdate),
        ))
        .run();
}

fn setup_board(mut commands: Commands) {
    commands.insert_resource(ArenaConfig {
        width: ARENA_WIDTH,
        height: ARENA_HEIGHT,
        background: GameColor::White,
    });

    commands.spawn((
        Name::new("PlayfieldBackground"),
        Sprite::from_color(
            Color::srgb(0.07, 0.08, 0.1),
            Vec2::new(ARENA_WIDTH + 40.0, ARENA_HEIGHT + 40.0),
        ),
        Transform::from_xyz(0.0, 0.0, -5.0),
        GlobalTransform::IDENTITY,
        RenderLayers::layer(RenderLayer::Background.order()),
    ));

    let thickness = 20.0;
    let wall_color = Color::srgb(0.2, 0.25, 0.35);
    let half_w = ARENA_WIDTH * 0.5;
    let half_h = ARENA_HEIGHT * 0.5;

    // Horizontal boundaries
    for (name, y) in [("Bottom", -half_h), ("Top", half_h)] {
        commands.spawn((
            Name::new(format!("Wall::{name}")),
            RigidBody::Fixed,
            Collider::cuboid(half_w, thickness * 0.5),
            Sprite::from_color(wall_color, Vec2::new(ARENA_WIDTH, thickness)),
            Transform::from_xyz(0.0, y, 0.0),
            GlobalTransform::IDENTITY,
            RenderLayers::layer(RenderLayer::GameWorld.order()),
        ));
    }

    // Vertical boundaries
    for (name, x) in [("Left", -half_w), ("Right", half_w)] {
        commands.spawn((
            Name::new(format!("Wall::{name}")),
            RigidBody::Fixed,
            Collider::cuboid(thickness * 0.5, half_h),
            Sprite::from_color(wall_color, Vec2::new(thickness, ARENA_HEIGHT)),
            Transform::from_xyz(x, 0.0, 0.0),
            GlobalTransform::IDENTITY,
            RenderLayers::layer(RenderLayer::GameWorld.order()),
        ));
    }
}



fn spawn_ball(
    position: Vec2,
    color: GameColor,
    cluster: i32,
    commands: &mut Commands,
    config: &PhysicsConfig,
    rng: &mut impl Rng,
) {
    let radius = rng.gen_range(10.0..18.0);
    let mut bundle = BallBundle::new(position, radius, color);
    bundle.transform.translation.z = 0.05;

    let speed = rng.gen_range(80.0..180.0);
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
    bundle.ball.velocity = velocity;

    let material_color = match color {
        GameColor::Red => Color::srgb(0.92, 0.25, 0.25),
        GameColor::Green => Color::srgb(0.2, 0.85, 0.55),
        GameColor::Blue => Color::srgb(0.3, 0.45, 0.95),
        GameColor::Yellow => Color::srgb(0.95, 0.8, 0.3),
        GameColor::White => Color::srgb(0.9, 0.9, 0.95),
    };
    let linear_color = material_color.to_linear();

    commands.spawn((
        bundle,
        RigidBody::Dynamic,
        Collider::ball(radius),
        Velocity {
            linvel: velocity,
            angvel: 0.0,
        },
        Restitution {
            coefficient: config.ball_restitution,
            combine_rule: CoefficientCombineRule::Average,
        },
        Friction {
            coefficient: config.ball_friction,
            combine_rule: CoefficientCombineRule::Average,
        },
        ExternalForce::default(),
        Damping {
            linear_damping: 0.0,
            angular_damping: 1.0,
        },
        ActiveEvents::COLLISION_EVENTS,
        MetaBall {
            radius_world: radius,
        },
        MetaBallColor(linear_color),
        MetaBallCluster(cluster),
        Name::new("Ball"),
        RenderLayers::layer(RenderLayer::Metaballs.order()),
        DynamicEntity,
    ));
}

// Camera setup
fn setup_camera(mut commands: Commands) {
    // Spawn the 2D camera marker; required components (Camera, Projection, etc.) are added via #[require] on Camera2d.
    commands.spawn(Camera2d);
}

// Test ball to ensure metaballs renderer has something to render
fn spawn_test_ball(mut commands: Commands, config: Res<PhysicsConfig>) {
    let mut rng = rand::thread_rng();
    spawn_ball(Vec2::new(0.0, 0.0), GameColor::Blue, 0, &mut commands, &config, &mut rng);
}

// Event Handlers
struct BallSpawnHandler;
impl EventHandler for BallSpawnHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        if let GameEvent::SpawnBallAtCursor { position } = ev {
            let mut state = world.resource_mut::<PlaygroundState>();
            let cursor_pos = state.cursor_world_pos.unwrap_or(*position);
            state.ball_cluster_counter += 1;
            let cluster = state.ball_cluster_counter;
            drop(state);

            let preset = world.resource::<BallSpawnPreset>().clone();
            let config = world.resource::<PhysicsConfig>().clone();
            let mut rng = rand::thread_rng();

            let mut commands = world.commands();
            spawn_ball(cursor_pos, preset.color, cluster, &mut commands, &config, &mut rng);

            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "BallSpawnHandler" }
}

struct WidgetPlacementHandler;
impl EventHandler for WidgetPlacementHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        if let GameEvent::PlaceWidget { widget_type, position } = ev {
            let mut commands = world.commands();
                match widget_type {
                    WidgetType::Wall { start, end, thickness } => {
                        let center = (*start + *end) / 2.0;
                        let diff = *end - *start;
                        let length = diff.length();
                        let angle = diff.y.atan2(diff.x);

                        commands.spawn((
                            Name::new("PlacedWall"),
                            RigidBody::Fixed,
                            Collider::cuboid(length / 2.0, thickness / 2.0),
                            Sprite::from_color(
                                Color::srgb(0.2, 0.25, 0.35),
                                Vec2::new(length, *thickness),
                            ),
                            Transform::from_xyz(center.x, center.y, 0.0)
                                .with_rotation(Quat::from_rotation_z(angle)),
                            GlobalTransform::IDENTITY,
                            RenderLayers::layer(RenderLayer::GameWorld.order()),
                            DynamicEntity,
                        ));
                    }
                    WidgetType::Target { health: _, radius } => {
                        commands.spawn((
                            Name::new("PlacedTarget"),
                            RigidBody::Fixed,
                            Collider::ball(*radius),
                            Sprite::from_color(
                                Color::srgb(0.8, 0.3, 0.3),
                                Vec2::splat(radius * 2.0),
                            ),
                            Transform::from_xyz(position.x, position.y, 0.0),
                            GlobalTransform::IDENTITY,
                            RenderLayers::layer(RenderLayer::GameWorld.order()),
                            DynamicEntity,
                        ));
                    }
                    WidgetType::Hazard { bounds } => {
                        let size = bounds.size();
                        let center = bounds.center();
                        commands.spawn((
                            Name::new("PlacedHazard"),
                            RigidBody::Fixed,
                            Collider::cuboid(size.x / 2.0, size.y / 2.0),
                            Sprite::from_color(
                                Color::srgb(0.9, 0.4, 0.1),
                                size,
                            ),
                            Transform::from_xyz(center.x, center.y, 0.0),
                            GlobalTransform::IDENTITY,
                            RenderLayers::layer(RenderLayer::GameWorld.order()),
                            DynamicEntity,
                        ));
                    }
                    WidgetType::Paddle => {
                        let size = Vec2::new(80.0, 15.0);
                        commands.spawn((
                            Name::new("PlacedPaddle"),
                            RigidBody::Fixed,
                            Collider::cuboid(size.x / 2.0, size.y / 2.0),
                            Sprite::from_color(
                                Color::srgb(0.3, 0.7, 0.9),
                                size,
                            ),
                            Transform::from_xyz(position.x, position.y, 0.0),
                            GlobalTransform::IDENTITY,
                            RenderLayers::layer(RenderLayer::GameWorld.order()),
                            DynamicEntity,
                        ));
                    }
                    WidgetType::SpawnPoint => {
                        commands.spawn((
                            Name::new("PlacedSpawnPoint"),
                            Sprite::from_color(
                                Color::srgb(0.5, 0.9, 0.5),
                                Vec2::splat(25.0),
                            ),
                            Transform::from_xyz(position.x, position.y, 0.0),
                            GlobalTransform::IDENTITY,
                            RenderLayers::layer(RenderLayer::GameWorld.order()),
                            DynamicEntity,
                        ));
                    }
                }
            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "WidgetPlacementHandler" }
}

struct SelectionHandler;
impl EventHandler for SelectionHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        match ev {
            GameEvent::SelectEntity { entity } => {
                let mut state = world.resource_mut::<PlaygroundState>();
                state.selected_entity = *entity;
                EventResult::Handled
            }
            GameEvent::MoveEntity { entity, position } => {
                if let Some(mut transform) = world.get_mut::<Transform>(*entity) {
                    transform.translation.x = position.x;
                    transform.translation.y = position.y;
                }
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }

    fn name(&self) -> &'static str { "SelectionHandler" }
}

struct DeletionHandler;
impl EventHandler for DeletionHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        if let GameEvent::DeleteEntity { entity } = ev {
            world.despawn(*entity);
            let mut state = world.resource_mut::<PlaygroundState>();
            if state.selected_entity == Some(*entity) {
                state.selected_entity = None;
            }
            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "DeletionHandler" }
}

struct ClearArenaHandler;
impl EventHandler for ClearArenaHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        if matches!(ev, GameEvent::ClearArena) {
            let entities: Vec<Entity> = world
                .query_filtered::<Entity, With<DynamicEntity>>()
                .iter(world)
                .collect();

            for entity in entities {
                world.despawn(entity);
            }

            let mut state = world.resource_mut::<PlaygroundState>();
            state.selected_entity = None;
            state.ball_cluster_counter = 0;

            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "ClearArenaHandler" }
}

struct ToolChangeHandler;
impl EventHandler for ToolChangeHandler {
    fn handle(&mut self, ev: &GameEvent, world: &mut World) -> EventResult {
        if let GameEvent::ChangeTool { mode } = ev {
            world.insert_resource(*mode);
            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "ToolChangeHandler" }
}

struct PhysicsToggleHandler;
impl EventHandler for PhysicsToggleHandler {
    fn handle(&mut self, ev: &GameEvent, _world: &mut World) -> EventResult {
        if matches!(ev, GameEvent::TogglePhysics) {
            // TODO: Fix physics toggle - RapierConfiguration is not a Resource in current version
            // Need to investigate proper way to pause physics
            warn!("Physics toggle not yet implemented");
            EventResult::Handled
        } else {
            EventResult::Ignored
        }
    }

    fn name(&self) -> &'static str { "PhysicsToggleHandler" }
}

// Input Systems
fn track_mouse_position(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut state: ResMut<PlaygroundState>,
) {
    let Ok(window) = windows.single() else {
        warn!("No window in track_mouse_position");
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        warn!("No camera in track_mouse_position");
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        match camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            Ok(world_pos) => {
                state.cursor_world_pos = Some(world_pos);
            }
            Err(e) => {
                warn!("Failed to convert cursor position: {:?}", e);
            }
        }
    } else {
        // Cursor not in window
        state.cursor_world_pos = None;
    }
}

fn handle_mouse_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    mut queue: ResMut<EventQueue>,
    frame: Res<FrameCounter>,
    mut diag: ResMut<InputDiagnostics>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        info!("Left click detected!");
    }

    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(pos) = state.cursor_world_pos else {
        diag.dropped_clicks += 1;
        warn!("Click detected but no cursor position available (dropped: {})", diag.dropped_clicks);
        return;
    };

    info!("Placing in mode {:?} at {:?}", *mode, pos);

    match *mode {
        PlaygroundMode::SpawnBall => {
            queue.enqueue(EventEnvelope::new(EventPayload::Game(GameEvent::SpawnBallAtCursor { position: pos }), EventSourceTag::Input, frame.0), frame.0);
        }
        PlaygroundMode::PlaceWall => {
            // Simple wall placement - fixed size horizontal wall
            let thickness = 20.0;
            let length = 80.0;
            let start = pos - Vec2::new(length / 2.0, 0.0);
            let end = pos + Vec2::new(length / 2.0, 0.0);
            queue.enqueue(
                EventEnvelope::new(
                    EventPayload::Game(GameEvent::PlaceWidget {
                        widget_type: WidgetType::Wall { start, end, thickness },
                        position: pos,
                    }),
                    EventSourceTag::Input,
                    frame.0
                ),
                frame.0,
            );
        }
        PlaygroundMode::PlaceTarget => {
            queue.enqueue(
                EventEnvelope::new(
                    EventPayload::Game(GameEvent::PlaceWidget {
                        widget_type: WidgetType::Target { health: 3, radius: 20.0 },
                        position: pos,
                    }),
                    EventSourceTag::Input,
                    frame.0
                ),
                frame.0,
            );
        }
        PlaygroundMode::PlaceHazard => {
            let size = Vec2::new(60.0, 60.0);
            let bounds = Rect::from_center_size(pos, size);
            queue.enqueue(
                EventEnvelope::new(
                    EventPayload::Game(GameEvent::PlaceWidget {
                        widget_type: WidgetType::Hazard { bounds },
                        position: pos,
                    }),
                    EventSourceTag::Input,
                    frame.0
                ),
                frame.0,
            );
        }
        PlaygroundMode::PlacePaddle => {
            queue.enqueue(
                EventEnvelope::new(
                    EventPayload::Game(GameEvent::PlaceWidget {
                        widget_type: WidgetType::Paddle,
                        position: pos,
                    }),
                    EventSourceTag::Input,
                    frame.0
                ),
                frame.0,
            );
        }
        PlaygroundMode::PlaceSpawnPoint => {
            queue.enqueue(
                EventEnvelope::new(
                    EventPayload::Game(GameEvent::PlaceWidget {
                        widget_type: WidgetType::SpawnPoint,
                        position: pos,
                    }),
                    EventSourceTag::Input,
                    frame.0
                ),
                frame.0,
            );
        }
        PlaygroundMode::Select => {
            // TODO: Implement entity selection via raycast
        }
        PlaygroundMode::Delete => {
            // TODO: Implement entity deletion via raycast
        }
    }
}

fn handle_key_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<EventQueue>,
    frame: Res<FrameCounter>,
) {
    // Inject key events into the event queue
    for key in keys.get_just_pressed() {
        queue.enqueue(
            EventEnvelope::new(
                EventPayload::Input(InputEvent::KeyDown(*key)),
                EventSourceTag::Input,
                frame.0
            ),
            frame.0,
        );
    }
}

// UI Systems
fn spawn_instructions_overlay(mut commands: Commands) {
    let half_w = ARENA_WIDTH * 0.5;
    let half_h = ARENA_HEIGHT * 0.5;

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Transform::from_xyz(-half_w + 20.0, half_h - 20.0, 10.0),
        InstructionsText,
    ));
}

fn update_instructions_overlay(
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    diag: Res<InputDiagnostics>,
    mut query: Query<&mut Text2d, With<InstructionsText>>,
) {
    let Ok(mut text) = query.single_mut() else { return };

    let mode_name = match *mode {
        PlaygroundMode::SpawnBall => "Spawn Ball",
        PlaygroundMode::PlaceWall => "Place Wall",
        PlaygroundMode::PlaceTarget => "Place Target",
        PlaygroundMode::PlaceHazard => "Place Hazard",
        PlaygroundMode::PlacePaddle => "Place Paddle",
        PlaygroundMode::PlaceSpawnPoint => "Place Spawn Point",
        PlaygroundMode::Select => "Select Entity",
        PlaygroundMode::Delete => "Delete Entity",
    };

    let cursor_info = if let Some(pos) = state.cursor_world_pos {
        format!("Cursor: ({:.0}, {:.0})", pos.x, pos.y)
    } else {
        "Cursor: --".to_string()
    };

    text.0 = format!(
        "PHYSICS PLAYGROUND\n\
Mode: [{}]\n\
{}\n\n\
TOOLS:\n\
1: Spawn Ball    5: Place Paddle\n\
2: Place Wall    6: Place Spawn Point\n\
3: Place Target  7: Select Entity\n\
4: Place Hazard  8: Delete Entity\n\n\
ACTIONS:\n\
Space: Spawn Ball\n\
Left Click: Use Current Tool\n\
C: Clear Arena\n\
R: Reset Level\n\
P: Toggle Physics\n\n\
DIAG: Dropped Clicks: {}",
        mode_name,
        cursor_info,
        diag.dropped_clicks,
    );
}

fn preview_widget_placement(
    mode: Res<PlaygroundMode>,
    state: Res<PlaygroundState>,
    mut commands: Commands,
    preview_q: Query<Entity, With<PlacementPreview>>,
) {
    // Despawn old preview
    for entity in &preview_q {
        commands.entity(entity).despawn();
    }

    let Some(pos) = state.cursor_world_pos else { return };

    let color = Color::srgba(1.0, 1.0, 1.0, 0.3);

    match *mode {
        PlaygroundMode::PlaceWall => {
            let size = Vec2::new(80.0, 20.0);
            commands.spawn((
                Sprite::from_color(color, size),
                Transform::from_xyz(pos.x, pos.y, 1.0),
                GlobalTransform::IDENTITY,
                RenderLayers::layer(RenderLayer::GameWorld.order()),
                PlacementPreview,
            ));
        }
        PlaygroundMode::PlaceTarget => {
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(40.0)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
                GlobalTransform::IDENTITY,
                RenderLayers::layer(RenderLayer::GameWorld.order()),
                PlacementPreview,
            ));
        }
        PlaygroundMode::PlaceHazard => {
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(60.0)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
                GlobalTransform::IDENTITY,
                RenderLayers::layer(RenderLayer::GameWorld.order()),
                PlacementPreview,
            ));
        }
        PlaygroundMode::PlacePaddle => {
            commands.spawn((
                Sprite::from_color(color, Vec2::new(80.0, 15.0)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
                GlobalTransform::IDENTITY,
                RenderLayers::layer(RenderLayer::GameWorld.order()),
                PlacementPreview,
            ));
        }
        PlaygroundMode::PlaceSpawnPoint => {
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(25.0)),
                Transform::from_xyz(pos.x, pos.y, 1.0),
                GlobalTransform::IDENTITY,
                RenderLayers::layer(RenderLayer::GameWorld.order()),
                PlacementPreview,
            ));
        }
        _ => {}
    }
}

fn highlight_selected_entity(
    state: Res<PlaygroundState>,
    mut gizmos: Gizmos,
    transforms: Query<&Transform>,
) {
    if let Some(entity) = state.selected_entity {
        if let Ok(transform) = transforms.get(entity) {
            let pos = transform.translation.truncate();
            gizmos.circle_2d(pos, 30.0, Color::srgb(1.0, 1.0, 0.0));
        }
    }
}
