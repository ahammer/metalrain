use bevy::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody};

use crate::core::config::config::{GameConfig, GravityWidgetConfig};

use super::layout::{LayoutFile, WallSegment};
// v2 grouped walls/timelines
use super::wall_timeline::{WallGroupRoot, WallTimeline};
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
        // Solid wall visuals are spawned directly during load; gizmo lines no longer required.
        app.add_systems(Startup, load_level_data);
    }
}

pub fn load_level_data(
    mut commands: Commands,
    mut game_cfg: ResMut<GameConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
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
    let layout_loaded = match LayoutFile::load_from_file(&layout_path) {
        Ok(lf) => {
            let segs = lf.to_wall_segments();
            debug!(target="level", "LevelLoader: layout loaded count={}", segs.len());
            info!(target="level", "LevelLoader: loaded {} level-specific wall segments", segs.len());
            all_walls.extend(segs);
            Some(lf)
        }
        Err(e) => {
            debug!(target="level", "LevelLoader: layout load FAILED: {e}");
            error!("LevelLoader: FAILED to load level layout '{}': {e}", layout_path.display());
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
        if length <= 1e-4 { continue; }
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
            Transform { translation: Vec3::new(center.x, center.y, WALL_Z), rotation: Quat::from_rotation_z(angle), scale: Vec3::ONE },
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
                WallGroupRoot { name: g.name.clone(), pivot },
                // Kinematic body so physics colliders (children) move with animated transform
                RigidBody::KinematicPositionBased,
                Transform { translation: Vec3::new(pivot.x, pivot.y, WALL_Z), ..Default::default() },
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
                if length <= 1e-4 { continue; }
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
                        Transform { translation: Vec3::new(center.x, center.y, 0.0), rotation: Quat::from_rotation_z(angle), scale: Vec3::ONE },
                        GlobalTransform::default(),
                        Visibility::Visible,
                    ));
                });
            }
        }
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

// Legacy gizmo drawer retained for quick debugging (unused by default). Enable manually if needed.
#[allow(dead_code)]
pub fn draw_wall_gizmos(walls: Res<LevelWalls>, mut gizmos: Gizmos) {
    for w in &walls.0 {
        gizmos.line_2d(w.from, w.to, Color::srgba(0.85, 0.75, 0.10, 0.90));
    }
}
