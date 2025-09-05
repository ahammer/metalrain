use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody};

use crate::core::config::config::{GameConfig, GravityWidgetConfig};

use super::layout::{LayoutFile, WallSegment};
// v2 grouped walls/timelines
use super::embedded_levels::{select_level_source, LevelSourceMode};
use super::registry::resolve_requested_level_id; // still used for CLI/env resolution (registry deprecated for selection list)
use super::wall_timeline::{WallGroupRoot, WallTimeline};
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
        // Solid wall visuals are spawned directly during load; gizmo lines no longer required.
        // Tests often run with only MinimalPlugins; ensure required asset storages exist.
        if app.world().get_resource::<Assets<Mesh>>().is_none() {
            app.init_resource::<Assets<Mesh>>();
        }
        if app
            .world()
            .get_resource::<Assets<ColorMaterial>>()
            .is_none()
        {
            app.init_resource::<Assets<ColorMaterial>>();
        }
        app.add_systems(Startup, load_level_data);
    }
}

pub fn load_level_data(
    mut commands: Commands,
    mut game_cfg: ResMut<GameConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Determine desired mode & features
    let live_requested = cfg!(feature = "live_levels")
        && !cfg!(any(target_arch = "wasm32", feature = "embedded_levels"));
    if cfg!(all(feature = "embedded_levels", feature = "live_levels")) {
        warn!(target="level", "LevelLoader: both 'embedded_levels' and 'live_levels' features active; live reload disabled in embedded mode");
    }

    // Select provider (embedded on wasm or embedded feature; else disk/disk+live)
    let (mode, source) = select_level_source(live_requested);

    // Resolve requested id (CLI/env) or use provider default
    let requested = resolve_requested_level_id();
    // Selection precedence: CLI/env override > game.ron (default_level_id) > registry/source default
    let cfg_default = game_cfg.default_level_id.trim();
    let mut chosen_id_string: String = requested
        .as_deref()
        .map(|s| s.to_string())
        .or_else(|| {
            if !cfg_default.is_empty() {
                Some(cfg_default.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| source.default_id().to_string());
    // Validate presence in source; if missing, warn and fall back to source default
    if !source.list_ids().iter().any(|id| *id == chosen_id_string) {
        warn!(
            target="level",
            "LevelLoader: configured default level '{}' not found in source ids {:?}; falling back to '{}'",
            chosen_id_string,
            source.list_ids(),
            source.default_id()
        );
        chosen_id_string = source.default_id().to_string();
    }
    let chosen_id = chosen_id_string.as_str();

    // Mode log (single authoritative line prior to any level file parsing except universal walls).
    info!(
        target = "level",
        "LevelLoader: mode={:?} requested='{:?}' selected level id='{}'",
        mode,
        requested,
        chosen_id
    );

    // Universal walls: embed on wasm / embedded feature to ensure zero runtime FS I/O; disk-load otherwise.
    let mut all_walls: Vec<WallSegment> = Vec::new();
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    {
        const UNIVERSAL_WALLS_RON: &str = include_str!("../../../assets/levels/basic_walls.ron");
        let lf: LayoutFile =
            ron::from_str(UNIVERSAL_WALLS_RON).expect("parse embedded universal walls failed");
        let segs = lf.to_wall_segments();
        debug!(
            target = "level",
            "LevelLoader: universal walls (embedded) loaded count={}",
            segs.len()
        );
        info!(
            target = "level",
            "LevelLoader: loaded {} universal wall segments",
            segs.len()
        );
        all_walls.extend(segs);
    }
    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    {
        use std::path::PathBuf;
        let crate_root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
        let base_levels: PathBuf = PathBuf::from(&crate_root).join("assets").join("levels");
        let universal_walls_path = base_levels.join("basic_walls.ron");
        match LayoutFile::load_from_file(&universal_walls_path) {
            Ok(lf) => {
                let segs = lf.to_wall_segments();
                debug!(
                    target = "level",
                    "LevelLoader: universal walls loaded count={}",
                    segs.len()
                );
                info!(
                    target = "level",
                    "LevelLoader: loaded {} universal wall segments",
                    segs.len()
                );
                all_walls.extend(segs);
            }
            Err(e) => {
                debug!(
                    target = "level",
                    "LevelLoader: universal walls load FAILED: {e}"
                );
                error!(
                    "LevelLoader: FAILED to load universal walls file {}: {e}",
                    universal_walls_path.display()
                );
                return;
            }
        }
    }

    // Acquire level layout/widgets contents depending on mode
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    let (layout_txt, widgets_txt) = match source.get_level(chosen_id) {
        Ok(p) => p,
        Err(e) => {
            panic!("LevelLoader: embedded level retrieval failed: {e}");
        }
    };

    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    let (layout_owned, widgets_owned) = match source.get_level_owned(chosen_id) {
        Ok(p) => p,
        Err(e) => {
            error!("LevelLoader: FAILED to load level '{}': {e}", chosen_id);
            return;
        }
    };

    // Parse layout
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    let layout_loaded: Option<LayoutFile> = {
        let lf: LayoutFile = ron::from_str(layout_txt).expect("parse embedded layout failed");
        let segs = lf.to_wall_segments();
        debug!(
            target = "level",
            "LevelLoader: layout loaded count={}",
            segs.len()
        );
        info!(
            target = "level",
            "LevelLoader: loaded {} level-specific wall segments",
            segs.len()
        );
        all_walls.extend(segs);
        Some(lf)
    };

    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    let layout_loaded: Option<LayoutFile> = match ron::from_str::<LayoutFile>(&layout_owned) {
        Ok(lf) => {
            if lf.version != 1 && lf.version != 2 {
                error!("LevelLoader: layout version unsupported");
                return;
            }
            let segs = lf.to_wall_segments();
            debug!(
                target = "level",
                "LevelLoader: layout loaded count={}",
                segs.len()
            );
            info!(
                target = "level",
                "LevelLoader: loaded {} level-specific wall segments",
                segs.len()
            );
            all_walls.extend(segs);
            Some(lf)
        }
        Err(e) => {
            debug!(target = "level", "LevelLoader: layout parse FAILED: {e}");
            error!("LevelLoader: FAILED to parse level layout: {e}");
            return;
        }
    };

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

    // ==================================================================================
    // Spawn static wall entities with:
    // - Cuboid collider matching data-driven thickness & length
    // - Rectangle mesh for visual (solid bar)
    // Layering: metaballs fullscreen quad at z=50, spawn widgets at z=82 -> choose z=70.
    // ==================================================================================
    const WALL_Z: f32 = 70.0; // Above metaballs (50.0), below widgets (82.0)
    let wall_color = Color::srgba(0.12, 0.12, 0.16, 0.95);
    for (i, w) in filtered.iter().enumerate() {
        let delta = w.to - w.from;
        let length = delta.length();
        if length <= 1e-4 {
            continue;
        }
        let thickness = w.thickness.max(2.0);
        let center = (w.from + w.to) * 0.5;
        let angle = delta.y.atan2(delta.x);
        let mesh = meshes.add(Mesh::from(Rectangle::new(length, thickness)));
        let material = materials.add(wall_color);
        commands.spawn((
            Name::new(format!("WallSeg{}", i)),
            WallVisual,
            // Use a fixed body + cuboid collider to match the visual thickness (segment was invisible & zero-width)
            RigidBody::Fixed,
            Collider::cuboid(length * 0.5, thickness * 0.5),
            Mesh2d::from(mesh),
            MeshMaterial2d(material),
            Transform {
                translation: Vec3::new(center.x, center.y, WALL_Z),
                rotation: Quat::from_rotation_z(angle),
                scale: Vec3::ONE,
            },
            GlobalTransform::default(),
            Visibility::Visible,
        ));
    }

    // Spawn group hierarchies (v2)
    if let Some(layout) = &layout_loaded {
        for g in &layout.groups {
            // Root entity with optional timeline
            let pivot: Vec2 = g.pivot.clone().into();
            let mut root_cmd = commands.spawn((
                Name::new(format!("WallGroup:{}", g.name)),
                WallGroupRoot {
                    name: g.name.clone(),
                    pivot,
                },
                // Kinematic body so physics colliders (children) move with animated transform
                RigidBody::KinematicPositionBased,
                Transform {
                    translation: Vec3::new(pivot.x, pivot.y, WALL_Z),
                    ..Default::default()
                },
                GlobalTransform::default(),
                Visibility::Visible,
            ));
            if let Some(tl) = &g.timeline {
                root_cmd.insert(WallTimeline::from_def(tl));
            }
            let root_entity = root_cmd.id();
            // Child walls positioned relative to pivot
            for (wi, w) in g.walls.iter().enumerate() {
                let seg = &w.segment;
                let from: Vec2 = seg.from.clone().into();
                let to: Vec2 = seg.to.clone().into();
                let delta = to - from;
                let length = delta.length();
                if length <= 1e-4 {
                    continue;
                }
                let thickness = seg.thickness.max(2.0);
                let center = (from + to) * 0.5 - pivot; // local offset
                let angle = delta.y.atan2(delta.x);
                let mesh = meshes.add(Mesh::from(Rectangle::new(length, thickness)));
                let material = materials.add(wall_color);
                commands.entity(root_entity).with_children(|c| {
                    c.spawn((
                        Name::new(format!("{}:Seg{}", g.name, wi)),
                        WallVisual,
                        Collider::cuboid(length * 0.5, thickness * 0.5),
                        Mesh2d::from(mesh),
                        MeshMaterial2d(material),
                        Transform {
                            translation: Vec3::new(center.x, center.y, 0.0),
                            rotation: Quat::from_rotation_z(angle),
                            scale: Vec3::ONE,
                        },
                        GlobalTransform::default(),
                        Visibility::Visible,
                    ));
                });
            }
        }
    }

    commands.insert_resource(LevelWalls(filtered));

    // Load / parse widgets
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    let widgets_file: WidgetsFile = {
        let wf: WidgetsFile = ron::from_str(widgets_txt).expect("parse embedded widgets failed");
        if wf.version != 1 {
            panic!(
                "WidgetsFile version {} unsupported (expected 1)",
                wf.version
            );
        }
        debug!(
            target = "level",
            "LevelLoader: widgets file loaded ({} entries)",
            wf.widgets.len()
        );
        wf
    };

    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    let widgets_file: WidgetsFile = match ron::from_str::<WidgetsFile>(&widgets_owned) {
        Ok(wf) => {
            if wf.version != 1 {
                error!(
                    "WidgetsFile version {} unsupported (expected 1)",
                    wf.version
                );
                return;
            }
            debug!(
                target = "level",
                "LevelLoader: widgets file loaded ({} entries)",
                wf.widgets.len()
            );
            wf
        }
        Err(e) => {
            debug!(target = "level", "LevelLoader: widgets parse FAILED: {e}");
            error!("LevelLoader: FAILED to parse widgets: {e}");
            return;
        }
    };
    let extracted = extract_widgets(&widgets_file);
    for w in &extracted.warnings {
        warn!("{w}");
    }
    info!(
        target = "level",
        "LevelLoader: extracted {} spawn points, {} attractors",
        extracted.spawn_points.len(),
        extracted.attractors.len()
    );

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
    commands.insert_resource(LevelSelection { id: chosen_id.to_string() });

    info!(
        target = "level",
        "LevelLoader: completed (walls={}, spawn_points={}, attractors={})",
        wall_count,
        game_cfg.spawn_widgets.widgets.len(),
        game_cfg.gravity_widgets.widgets.len()
    );

    // Live reload stub warning (only disk live mode)
    if matches!(mode, LevelSourceMode::DiskLive) {
        warn!(
            target = "level",
            "LevelLoader: live_levels feature active but watcher not implemented (TODO)"
        );
    }
}

// Legacy gizmo drawer retained for quick debugging was removed as it's unused by default.
