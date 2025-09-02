use bevy::prelude::*;
use bevy::prelude::{OnEnter, NextState};
use bevy_rapier2d::prelude::Collider;

use crate::core::config::config::{GameConfig, GravityWidgetConfig};

use super::layout::{LayoutFile, WallSegment};
use super::registry::{resolve_requested_level_id, LevelRegistry};
use super::widgets::{extract_widgets, AttractorSpec, SpawnPointSpec, WidgetsFile};

#[derive(Component)]
struct WallVisual;

/// Marker component applied to ALL entities whose lifetime is bound to the
/// currently loaded level (so we can bulk-despawn when switching levels).
#[derive(Component, Debug)]
pub struct LevelEntity;

/// Resource inserted by the menu (or tests) to request a level load.
#[derive(Resource, Debug, Clone)]
pub struct PendingLevel { pub id: String }

/// Resource: final chosen level id
#[derive(Debug, Resource, Clone)]
pub struct LevelSelection {
    pub id: String,
}

/// Resource: all wall segments (universal + level-specific)
#[derive(Debug, Resource, Clone, Default)]
pub struct LevelWalls(pub Vec<WallSegment>);

/// Resource: loaded widget specs separated by kind
#[derive(Debug, Resource, Clone, Default)]
pub struct LevelWidgets {
    pub spawn_points: Vec<SpawnPointSpec>,
    pub attractors: Vec<AttractorSpec>,
}

/// Plugin performing data-driven level loading & integration into GameConfig
pub struct LevelLoaderPlugin;

impl Plugin for LevelLoaderPlugin {
    fn build(&self, app: &mut App) {
        use crate::app::state::AppState;
        app.add_systems(Startup, (load_level_registry, fallback_auto_load_if_no_state).chain());
        app.add_systems(OnEnter(AppState::Loading), (cleanup_level, process_loading));
    }
}

/// STARTUP: Load registry only.
pub fn load_level_registry(mut commands: Commands) {
    // Build absolute paths rooted at crate manifest dir to avoid cwd variance in tests.
    use std::path::PathBuf;
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let base_levels: PathBuf = PathBuf::from(&crate_root).join("assets").join("levels");
    let registry_path = base_levels.join("levels.ron");
    match LevelRegistry::load_from_file(&registry_path) {
        Ok(r) => {
            debug!(target="level", "LevelLoader: registry loaded from {}", registry_path.display());
            commands.insert_resource(r);
        },
        Err(e) => {
            println!("[LevelLoader DEBUG] registry load FAILED: {e}");
            error!("LevelLoader: FAILED to load registry: {e}");
            // Registry missing; menu will show warning.
        }
    };
}

/// Fallback path for environments without states/menu: automatically load default level on Startup.
fn fallback_auto_load_if_no_state(
    mut commands: Commands,
    mut game_cfg: ResMut<GameConfig>,
    registry: Option<Res<LevelRegistry>>,
    maybe_state: Option<Res<State<crate::app::state::AppState>>>,
    existing_selection: Option<Res<LevelSelection>>,
    pending: Option<Res<PendingLevel>>,
) {
    if existing_selection.is_some() || pending.is_some() { return; }
    if maybe_state.is_some() { return; } // States present => menu flow wanted
    let Some(reg) = registry else { return; };
    if let Err(e) = internal_perform_level_load(&mut commands, &mut game_cfg, &reg, None) {
        error!("LevelLoader: fallback autoload failed: {e}");
    }
}

// (Legacy immediate load path removed; loading now always state-driven.)

/// OnEnter(AppState::Loading): despawn prior level.
pub fn cleanup_level(
    mut commands: Commands,
    q_level: Query<Entity, With<LevelEntity>>,
) {
    for e in q_level.iter() {
        commands.entity(e).despawn(); // recursive by default in 0.16
    }
}

/// OnEnter(AppState::Loading): perform loading using PendingLevel request, then transition.
pub fn process_loading(
    mut commands: Commands,
    mut game_cfg: ResMut<GameConfig>,
    registry: Res<LevelRegistry>,
    pending: Option<Res<PendingLevel>>,
    mut next: ResMut<NextState<crate::app::state::AppState>>,
) {
    use crate::app::state::AppState;
    let Some(pending) = pending else {
        error!("LevelLoader: PendingLevel missing in Loading state");
        next.set(AppState::MainMenu);
        return;
    };
    let requested = Some(pending.id.as_str());
    match internal_perform_level_load(&mut commands, &mut game_cfg, &registry, requested) {
        Ok(sel_id) => {
            info!(target="level", "LevelLoader: transitioned to Gameplay after loading '{}'.", sel_id);
            next.set(AppState::Gameplay);
        }
        Err(e) => {
            error!("LevelLoader: load failed: {e}");
            next.set(AppState::MainMenu);
        }
    }
}

/// Shared core logic performing the actual file IO & entity spawning.
fn internal_perform_level_load(
    commands: &mut Commands,
    game_cfg: &mut ResMut<GameConfig>,
    registry: &LevelRegistry,
    requested: Option<&str>,
) -> Result<String, String> {
    use std::path::PathBuf;
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let base_levels: PathBuf = PathBuf::from(&crate_root).join("assets").join("levels");
    let universal_walls_path = base_levels.join("basic_walls.ron");

    // Resolve requested level id (cli/env) then select
    let requested_cli_env = requested.map(|s| s.to_string()).or_else(resolve_requested_level_id);
    let level_entry = match registry.select_level(requested_cli_env.as_deref()) {
        Ok(e) => e,
        Err(e) => {
            return Err(format!("select level: {e}"));
        }
    };
    info!(target: "level", "LevelLoader: selected level id='{}' (layout='{}', widgets='{}')", level_entry.id, level_entry.layout, level_entry.widgets);

    // Load universal walls
    let mut all_walls: Vec<WallSegment> = Vec::new();
    match LayoutFile::load_from_file(&universal_walls_path) {
        Ok(lf) => {
            let segs = lf.to_wall_segments();
            debug!(target="level", "LevelLoader: universal walls loaded count={}", segs.len());
            info!(target="level", "LevelLoader: loaded {} universal wall segments", segs.len());
            all_walls.extend(segs);
        }
        Err(e) => {
            debug!(target="level", "LevelLoader: universal walls load FAILED: {e}");
            return Err(format!("universal walls load: {e}"));
        }
    }

    // Load level layout
    let layout_path = base_levels.join(&level_entry.layout);
    match LayoutFile::load_from_file(&layout_path) {
        Ok(lf) => {
            let segs = lf.to_wall_segments();
            debug!(target="level", "LevelLoader: layout loaded count={}", segs.len());
            info!(target="level", "LevelLoader: loaded {} level-specific wall segments", segs.len());
            all_walls.extend(segs);
        }
        Err(e) => {
            debug!(target="level", "LevelLoader: layout load FAILED: {e}");
            return Err(format!("level layout load: {e}"));
        }
    }

    // Validate walls (skip zero-length)
    let mut filtered = Vec::with_capacity(all_walls.len());
    for w in all_walls.into_iter() {
        if (w.from - w.to).length_squared() < 1e-6 {
            warn!("LevelLoader: wall segment endpoints identical; skipped.");
            continue;
        }
        filtered.push(w);
    }
    let wall_count = filtered.len();

    // Spawn static wall colliders (segment; thickness kept for future expansion)
    for (i, w) in filtered.iter().enumerate() {
        commands.spawn((
            LevelEntity,
            Name::new(format!("WallSeg{}", i)),
            Collider::segment(w.from, w.to),
            Transform::IDENTITY,
            GlobalTransform::default(),
            Visibility::Hidden,
        ));
    }

    commands.insert_resource(LevelWalls(filtered.clone()));

    // Load widgets
    let widgets_path = base_levels.join(&level_entry.widgets);
    let widgets_file = match WidgetsFile::load_from_file(&widgets_path) {
        Ok(w) => {
            debug!(target="level", "LevelLoader: widgets file loaded ({} entries)", w.widgets.len());
            w
        },
        Err(e) => {
            debug!(target="level", "LevelLoader: widgets load FAILED: {e}");
            return Err(format!("widgets load: {e}"));
        }
    };
    let extracted = extract_widgets(&widgets_file);
    for w in &extracted.warnings {
        warn!("{w}");
    }
    info!(target="level", "LevelLoader: extracted {} spawn points, {} attractors",
        extracted.spawn_points.len(), extracted.attractors.len());

    // Integrate spawn points into GameConfig.spawn_widgets (overriding any existing list)
    if !extracted.spawn_points.is_empty() {
        if !game_cfg.spawn_widgets.widgets.is_empty() {
            warn!("LevelLoader: ignoring GameConfig.spawn_widgets.widgets (data-driven widgets present).");
        }
        game_cfg.spawn_widgets.widgets = extracted
            .spawn_points
            .iter()
            .map(|sp| sp.to_config())
            .collect();
    }

    // Integrate attractors into GameConfig.gravity_widgets (override only if any present)
    if !extracted.attractors.is_empty() {
        if !game_cfg.gravity_widgets.widgets.is_empty() {
            // Overwrite; treat file as authoritative
            game_cfg.gravity_widgets.widgets.clear();
        }
        if game_cfg.gravity.y.abs() > 0.0 {
            warn!("LevelLoader: gravity.y legacy value ignored (attractors defined).");
        }
        game_cfg.gravity_widgets.widgets = extracted
            .attractors
            .iter()
            .map(|a| a.to_config())
            .collect::<Vec<GravityWidgetConfig>>();
    }

    // Insert LevelWidgets resource with full positional info
    commands.insert_resource(LevelWidgets {
        spawn_points: extracted.spawn_points.clone(),
        attractors: extracted.attractors.clone(),
    });

    // Insert selection resource
    commands.insert_resource(LevelSelection { id: level_entry.id.clone() });

    info!(target="level", "LevelLoader: completed (walls={}, spawn_points={}, attractors={})",
        wall_count,
        game_cfg.spawn_widgets.widgets.len(),
        game_cfg.gravity_widgets.widgets.len());
    Ok(level_entry.id)
}
