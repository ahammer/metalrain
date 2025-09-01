use bevy::prelude::*;
use bevy_rapier2d::prelude::Collider;

use crate::core::config::config::{GameConfig, GravityWidgetConfig};

use super::layout::{LayoutFile, WallSegment};
use super::registry::{resolve_requested_level_id, LevelRegistry};
use super::widgets::{extract_widgets, AttractorSpec, SpawnPointSpec, WidgetsFile};

#[derive(Component)]
struct WallVisual;

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
        app.add_systems(Startup, load_level_data)
            .add_systems(Update, draw_wall_gizmos);
    }
}

pub fn load_level_data(
    mut commands: Commands,
    mut game_cfg: ResMut<GameConfig>,
) {
    // Build absolute paths rooted at crate manifest dir to avoid cwd variance in tests.
    use std::path::PathBuf;
    let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let base_levels: PathBuf = PathBuf::from(&crate_root).join("assets").join("levels");
    let registry_path = base_levels.join("levels.ron");
    let universal_walls_path = base_levels.join("basic_walls.ron");

    // Load registry
    let registry = match LevelRegistry::load_from_file(&registry_path) {
        Ok(r) => {
            debug!(target="level", "LevelLoader: registry loaded from {}", registry_path.display());
            r
        },
        Err(e) => {
            println!("[LevelLoader DEBUG] registry load FAILED: {e}");
            error!("LevelLoader: FAILED to load registry: {e}");
            return;
        }
    };

    // Resolve requested level id (cli/env) then select
    let requested = resolve_requested_level_id();
    let level_entry = match registry.select_level(requested.as_deref()) {
        Ok(e) => e,
        Err(e) => {
            error!("LevelLoader: FAILED to select level: {e}");
            return;
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
            error!("LevelLoader: FAILED to load universal walls file {}: {e}", universal_walls_path.display());
            return;
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
            error!("LevelLoader: FAILED to load level layout '{}': {e}", layout_path.display());
            return;
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
            Name::new(format!("WallSeg{}", i)),
            Collider::segment(w.from, w.to),
            Transform::IDENTITY,
            GlobalTransform::default(),
            Visibility::Hidden,
        ));
    }

    commands.insert_resource(LevelWalls(filtered));

    // Load widgets
    let widgets_path = base_levels.join(&level_entry.widgets);
    let widgets_file = match WidgetsFile::load_from_file(&widgets_path) {
        Ok(w) => {
            debug!(target="level", "LevelLoader: widgets file loaded ({} entries)", w.widgets.len());
            w
        },
        Err(e) => {
            debug!(target="level", "LevelLoader: widgets load FAILED: {e}");
            error!("LevelLoader: FAILED to load level widgets '{}': {e}", widgets_path.display());
            return;
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
        spawn_points: extracted.spawn_points,
        attractors: extracted.attractors,
    });

    // Insert selection resource
    commands.insert_resource(LevelSelection { id: level_entry.id.clone() });

    info!(target="level", "LevelLoader: completed (walls={}, spawn_points={}, attractors={})",
        wall_count,
        game_cfg.spawn_widgets.widgets.len(),
        game_cfg.gravity_widgets.widgets.len());
}

pub fn draw_wall_gizmos(
    walls: Res<LevelWalls>,
    mut gizmos: Gizmos,
) {
    for w in &walls.0 {
        gizmos.line_2d(w.from, w.to, Color::srgba(0.85, 0.75, 0.10, 0.90));
    }
}
