use bevy::prelude::*;
use event_core::*;
use event_core::EventFlowSet;

pub const DEMO_NAME: &str = "input_demo";

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct CursorState {
    pub world: Option<Vec2>,
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct InputStats {
    pub clicks_total: u32,
    pub clicks_dropped: u32,
    pub keys_total: u32,
    pub last_key: Option<KeyCode>,
}

#[derive(Component)]
struct OverlayText;

pub fn run_input_demo() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(AssetPlugin { file_path: "../../assets".into(), ..default() }))
        .add_plugins(EventCorePlugin::default())
        .init_resource::<CursorState>()
        .init_resource::<InputStats>()
        .add_systems(Startup, (spawn_camera, spawn_overlay))
        .add_systems(Update, (
            track_cursor.in_set(EventFlowSet::InputCollect),
            collect_keys.in_set(EventFlowSet::InputCollect),
            process_clicks.in_set(EventFlowSet::InputProcess),
            update_overlay.in_set(EventFlowSet::UIUpdate),
        ))
        .run();
}

fn spawn_camera(mut commands: Commands) { commands.spawn(Camera2d); }

fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        Text2d::new(""),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Transform::from_xyz(-300.0, 260.0, 0.0),
        OverlayText,
    ));
}

fn track_cursor(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut cursor: ResMut<CursorState>,
) {
    let Ok(window) = windows.single() else { return; };
    let Ok((camera, cam_transform)) = camera_q.single() else { return; };
    if let Some(pos) = window.cursor_position() {
        if let Ok(world) = camera.viewport_to_world_2d(cam_transform, pos) { cursor.world = Some(world); }
    } else { cursor.world = None; }
}

fn process_clicks(
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorState>,
    mut stats: ResMut<InputStats>,
    mut gizmos: Gizmos,
) {
    if buttons.just_pressed(MouseButton::Left) {
        stats.clicks_total += 1;
        if let Some(p) = cursor.world {
            gizmos.circle_2d(p, 6.0, Color::srgb(0.2, 0.8, 1.0));
        } else {
            stats.clicks_dropped += 1;
        }
    }
}

fn collect_keys(keys: Res<ButtonInput<KeyCode>>, mut stats: ResMut<InputStats>) {
    for k in keys.get_just_pressed() { stats.keys_total += 1; stats.last_key = Some(*k); }
}

fn update_overlay(
    cursor: Res<CursorState>,
    stats: Res<InputStats>,
    mut q: Query<&mut Text2d, With<OverlayText>>,
    time: Res<Time>,
) {
    let Ok(mut text) = q.single_mut() else { return; };
    let cursor_line = if let Some(p) = cursor.world { format!("Cursor: ({:.0},{:.0})", p.x, p.y) } else { "Cursor: --".to_string() };
    let last_key = stats.last_key.map(|k| format!("{:?}", k)).unwrap_or_else(|| "None".into());
    text.0 = format!(
        "INPUT DEMO\n\
 Flow Sets: [Collect -> Process -> UI]\n\
 Time: {:.2}s\n\
 {}\n\
 Clicks: total={} dropped={} ({}%)\n\
 Keys: total={} last={}\n\
 Left Click: register click\n Move mouse: update cursor\n Press keys: update stats",
    time.elapsed_secs(),
        cursor_line,
        stats.clicks_total,
        stats.clicks_dropped,
    if stats.clicks_total == 0 { 0 } else { stats.clicks_dropped * 100 / stats.clicks_total },
        stats.keys_total,
        last_key,
    );
}
