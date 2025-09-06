use bevy::prelude::*;
use bevy::sprite::MeshMaterial2d;
use bevy_rapier2d::prelude::{Collider, Damping, Friction, Restitution, RigidBody, Velocity};
use rand::Rng;

use crate::core::components::{Ball, BallRadius};
use crate::core::config::config::GameConfig;
use crate::core::level::loader::LevelWidgets;
use crate::core::level::widgets::TextColorMode;
use crate::core::system::system_order::PrePhysicsSet;
use crate::rendering::materials::materials::{BallDisplayMaterials, BallMaterialIndex};
use crate::rendering::palette::palette::BASE_COLORS;

/// Root entity marker for a TextSpawn widget instance.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TextSpawnRoot { pub id: u32 }

/// Per-ball metadata linking to target glyph sample point.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TextBall {
    pub word_index: u16,
    pub char_index: u16,
    pub target_local: Vec2,
    pub settled: bool,
}

/// Attraction (spring) parameters shared by children.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TextAttractionParams {
    pub strength: f32,
    pub damping: f32,
    pub snap_distance: f32,
}

/// Plugin responsible for instantiating text spawn widgets and applying attraction.
pub struct TextSpawnPlugin;

impl Plugin for TextSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TextAttractionParams>()
            .register_type::<TextBall>()
            .register_type::<TextSpawnRoot>()
            .add_systems(PostStartup, instantiate_text_spawns_spec) // use new spec-compliant version
            .add_systems(Update, apply_text_attraction.in_set(PrePhysicsSet));
    }
}

// ----------------------------- Rasterization Helper -----------------------------

/// Convert text into a deduplicated set of local sample points representing the filled glyph shapes.
/// Returns a vector of (word_index, charIndex_within_word, local_position).
pub fn rasterize_text_points(text: &str, font: &fontdue::Font, font_px: u32, cell: f32) -> Vec<(usize, usize, Vec2)> {
    let cell_px = cell.max(1.0);
    let scale = font_px as f32;
    // Split into words preserving ordering
    let words: Vec<&str> = text.split_whitespace().collect();
    // Track running pen x advance
    let mut pen_x: f32 = 0.0;
    let mut points: Vec<(usize, usize, Vec2)> = Vec::new();
    let mut bbox_min = Vec2::splat(f32::MAX);
    let mut bbox_max = Vec2::splat(f32::MIN);

    for (wi, w) in words.iter().enumerate() {
        for (ci, ch) in w.chars().enumerate() {
            let (metrics, bitmap) = font.rasterize(ch, scale);
            if metrics.width == 0 || metrics.height == 0 { continue; }
            let step_y = cell_px as usize;
            let step_x = cell_px as usize;
            for py in (0..metrics.height).step_by(step_y.max(1)) {
                for px in (0..metrics.width).step_by(step_x.max(1)) {
                    let idx = py * metrics.width + px;
                    if idx >= bitmap.len() { continue; }
                    let alpha = bitmap[idx] as f32 / 255.0;
                    if alpha < 0.5 { continue; }
                    let local_x = pen_x + px as f32 + metrics.xmin as f32;
                    let local_y = -(py as f32 + metrics.ymin as f32); // y-up
                    let p = Vec2::new(local_x, local_y);
                    bbox_min = bbox_min.min(p);
                    bbox_max = bbox_max.max(p);
                    points.push((wi, ci, p));
                }
            }
            pen_x += metrics.advance_width;
        }
        // Add space advance (approx) between words
        if wi + 1 < words.len() { pen_x += scale * 0.4; }
    }

    if points.is_empty() { return points; }

    // Recenter to midpoint
    let center = (bbox_min + bbox_max) * 0.5;
    for (_, _, p) in points.iter_mut() { *p -= center; }

    // Deduplicate by grid hashing (cell*0.4 threshold)
    let dedup_thresh = cell * 0.4;
    let dedup_sq = dedup_thresh * dedup_thresh;
    points.sort_by(|a,b| a.2.x.partial_cmp(&b.2.x).unwrap_or(std::cmp::Ordering::Equal));
    let mut deduped: Vec<(usize, usize, Vec2)> = Vec::with_capacity(points.len());
    for (wi, ci, p) in points.into_iter() {
        if let Some(last) = deduped.last() { if (last.2 - p).length_squared() < dedup_sq { continue; } }
        deduped.push((wi, ci, p));
    }

    deduped
}

// ----------------------------- Instantiation System -----------------------------

#[allow(dead_code)]
fn instantiate_text_spawns_spec(
    mut commands: Commands,
    level_widgets: Option<Res<LevelWidgets>>,
    cfg: Res<GameConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    display_palette: Option<Res<BallDisplayMaterials>>,
    q_ball_count: Query<Entity, With<Ball>>,
) {
    let Some(lw) = level_widgets else { return; };
    if lw.text_spawns.is_empty() { return; }
    // Attempt to load a suitable TTF from disk in priority order. User added AovelSansRounded.
    let font_search = ["assets/fonts/AovelSansRounded-rdDL.ttf", "assets/fonts/FiraSans-Bold.ttf"];
    let mut loaded_font: Option<fontdue::Font> = None;
    for path in font_search { if let Ok(bytes) = std::fs::read(path) {
        match fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()) {
            Ok(f) => { info!(target="text_spawn", "Loaded font '{path}' for TextSpawn glyph rasterization"); loaded_font = Some(f); break; },
            Err(e) => warn!(target="text_spawn", "Failed to parse font '{path}': {e}"),
        }
    } }
    let Some(font) = loaded_font.as_ref() else {
        warn!(target="text_spawn", "No font available for TextSpawn; skipping all text spawns (add a TTF at assets/fonts/AovelSansRounded-rdDL.ttf)");
        return;
    };

    let mut existing = q_ball_count.iter().len();
    let global_cap = cfg.spawn_widgets.global_max_balls;

    for spec in lw.text_spawns.iter() {
        if existing >= global_cap { break; }
    let mut points = rasterize_text_points(&spec.text, font, spec.font_px, spec.cell);
        if points.is_empty() { continue; }
        points.sort_by(|a,b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
        let remaining_cap = global_cap.saturating_sub(existing);
        let truncated = points.len() > remaining_cap;
        if truncated { points.truncate(remaining_cap); }
        // Bounds for jitter distribution
        let mut min_v = Vec2::splat(f32::MAX); let mut max_v = Vec2::splat(f32::MIN);
        for (_,_,p) in &points { min_v = min_v.min(*p); max_v = max_v.max(*p); }
        let aabb_size = max_v - min_v;
        let root_e = commands.spawn((
            TextSpawnRoot { id: spec.id },
            TextAttractionParams { strength: spec.attraction_strength, damping: spec.attraction_damping, snap_distance: spec.snap_distance },
            Transform::from_xyz(spec.pos.x, spec.pos.y, 0.0),
            GlobalTransform::default(), Visibility::Visible,
            Name::new(format!("TextSpawnRoot:{}", spec.id)),
        )).id();
        let mut rng = rand::thread_rng();
        for (wi, ci, target_local) in points.into_iter() {
            // Randomized initial placement around root.
            let jitter_disk = Vec2::new(rng.gen_range(-spec.jitter..spec.jitter), rng.gen_range(-spec.jitter..spec.jitter));
            let jitter_box = Vec2::new(rng.gen_range(-aabb_size.x*0.5..aabb_size.x*0.5), rng.gen_range(-aabb_size.y*0.5..aabb_size.y*0.5));
            let offset = jitter_disk + jitter_box;
            let radius = rng.gen_range(spec.radius_min..spec.radius_max);
            let speed = rng.gen_range(spec.speed_min..spec.speed_max);
            let dir = Vec2::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)).normalize_or_zero();
            let linvel = dir * speed;
            let variant_idx = match spec.color_mode {
                TextColorMode::RandomPerBall => {
                    if let Some(disp) = display_palette.as_ref() { let len = disp.0.len().max(1); if len>1 { rng.gen_range(0..len) } else {0} } else { let len = BASE_COLORS.len().max(1); if len>1 { rng.gen_range(0..len) } else {0} }
                }
                TextColorMode::WordSolid => {
                    if !spec.word_palette_indices.is_empty() { spec.word_palette_indices[wi % spec.word_palette_indices.len()] } else {0}
                }
                TextColorMode::Single => 0,
            };
            let bounce = &cfg.bounce;
            let mut e = commands.spawn((
                Ball,
                BallRadius(radius),
                BallMaterialIndex(variant_idx),
                TextBall { word_index: wi as u16, char_index: ci as u16, target_local, settled: false },
                RigidBody::Dynamic,
                Velocity { linvel, angvel: 0.0 },
                Damping { linear_damping: bounce.linear_damping, angular_damping: bounce.angular_damping },
                Restitution::coefficient(bounce.restitution),
                Friction::coefficient(bounce.friction),
                Collider::ball(radius),
                Transform::from_xyz(spec.pos.x + offset.x, spec.pos.y + offset.y, 0.0),
                GlobalTransform::default(), Visibility::Visible,
                Name::new(format!("TextBall:{}:{}:{}", spec.id, wi, ci)),
            ));
            if cfg.draw_circles { if let Some(disp) = display_palette.as_ref() { if variant_idx < disp.0.len() { let mesh_h = meshes.add(Mesh::from(Circle{radius})); e.insert((Mesh2d::from(mesh_h), MeshMaterial2d(disp.0[variant_idx].clone()))); } } }
            e.insert(ChildOf(root_e));
            existing += 1; if existing >= global_cap { break; }
        }
        info!(target="text_spawn", "TextSpawn id={} text=\"{}\" points={} truncated={}", spec.id, spec.text, existing, truncated);
        if existing >= global_cap { break; }
    }
}

// ----------------------------- Attraction System -----------------------------

fn apply_text_attraction(
    time: Res<Time>,
    mut q_balls: Query<(&ChildOf, &mut Velocity, &mut TextBall, &Transform)>,
    q_roots: Query<(&TextSpawnRoot, &TextAttractionParams, &Transform)>,
) {
    let dt = time.delta_secs();
    for (parent, mut vel, mut tb, tf) in q_balls.iter_mut() {
        let root_entity = parent.parent();
        if let Ok((_root, params, root_tf)) = q_roots.get(root_entity) {
            if tb.settled {
                vel.linvel *= 0.90;
                continue;
            }
            let world_target = root_tf.translation.truncate() + tb.target_local;
            let pos = tf.translation.truncate();
            let delta = world_target - pos;
            if delta.length() < params.snap_distance && vel.linvel.length() < params.snap_distance * 2.0 {
                tb.settled = true;
                vel.linvel *= 0.5;
                continue;
            }
            let accel = params.strength * delta - params.damping * vel.linvel;
            vel.linvel += accel * dt;
        }
    }
}
