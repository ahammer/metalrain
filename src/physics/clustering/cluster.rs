use crate::core::components::{Ball, BallRadius};
use crate::core::system::system_order::PostPhysicsAdjustSet;
use crate::rendering::materials::materials::{BallDisplayMaterials, BallMaterialIndex};
use crate::rendering::palette::palette::color_for_index;
use bevy::prelude::*;
use crate::interaction::cluster_pop::PaddleLifecycle;

type ClusterQueryItem<'a> = (
    Entity,
    &'a Transform,
    &'a BallRadius,
    &'a BallMaterialIndex,
    Option<&'a PaddleLifecycle>,
);

#[derive(Debug, Clone)]
pub struct Cluster {
    pub color_index: usize,
    pub entities: Vec<Entity>,
    pub min: Vec2,
    pub max: Vec2,
    pub centroid: Vec2,
    pub total_area: f32,
}
impl Cluster {
    fn new(color_index: usize) -> Self {
        Self {
            color_index,
            entities: Vec::new(),
            min: Vec2::splat(f32::INFINITY),
            max: Vec2::splat(f32::NEG_INFINITY),
            centroid: Vec2::ZERO,
            total_area: 0.0,
        }
    }
}
#[derive(Resource, Default, Debug, Clone)]
pub struct Clusters(pub Vec<Cluster>);

/// Reverse index: ball Entity -> cluster index (into `Clusters.0`)
/// Rebuilt every frame immediately after clusters are computed.
#[derive(Resource, Default, Debug, Clone)]
pub struct BallClusterIndex(pub std::collections::HashMap<Entity, usize>);
#[derive(Debug, Clone)]
struct BallPersist {
    cluster_id: u64,
    last_touch_time: f32,
    color_index: usize,
}
#[derive(Resource, Default)]
struct ClusterPersistence {
    map: std::collections::HashMap<Entity, BallPersist>,
    next_cluster_id: u64,
}
const DETACH_THRESHOLD: f32 = 0.5;
/// Core clustering logic plugin: computes clusters and maintains reverse indices.
/// (Extracted so tests can depend on clustering logic without pulling in gizmo resources.)
pub struct ClusterCorePlugin;
impl Plugin for ClusterCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clusters>()
            .init_resource::<BallClusterIndex>()
            .init_resource::<ClusterPersistence>()
            .add_systems(Update, compute_clusters.in_set(PostPhysicsAdjustSet));
    }
}

/// Optional debug drawing for clusters; depends on gizmo infrastructure.
pub struct ClusterDebugPlugin;
impl Plugin for ClusterDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_draw_clusters.after(compute_clusters));
    }
}

/// Backwards-compatible umbrella plugin preserving previous behavior (core + debug drawing).
pub struct ClusterPlugin;
impl Plugin for ClusterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ClusterCorePlugin, ClusterDebugPlugin));
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct Cell(i32, i32);
fn compute_clusters(
    mut clusters: ResMut<Clusters>,
    mut cluster_index: ResMut<BallClusterIndex>,
    mut persistence: ResMut<ClusterPersistence>,
    time: Res<Time>,
    q: Query<ClusterQueryItem<'_>, With<Ball>>,
    cfg: Option<Res<crate::core::config::GameConfig>>,
) {
    clusters.0.clear();
    cluster_index.0.clear();

    let exclude_popping = cfg
        .as_ref()
        .map(|g| g.interactions.cluster_pop.exclude_from_new_clusters)
        .unwrap_or(true);

    // Collect only included (non-popping when excluded) balls
    let mut entities: Vec<Entity> = Vec::new();
    let mut positions: Vec<Vec2> = Vec::new();
    let mut radii: Vec<f32> = Vec::new();
    let mut colors: Vec<usize> = Vec::new();
    let mut max_radius = 0.0f32;

    for (e, t, r, c, popping) in q.iter() {
        if exclude_popping && popping.is_some() {
            continue;
        }
        entities.push(e);
        let p = t.translation.truncate();
        positions.push(p);
        radii.push(r.0);
        colors.push(c.0);
        if r.0 > max_radius {
            max_radius = r.0;
        }
    }

    let count = entities.len();
    if count == 0 {
        // No included balls this frame; clear persistence of removed entities
        persistence.map.clear();
        return;
    }

    // Union-find buffers sized to included count ONLY (bug fix: previously sized to total query count incl. excluded)
    let mut parent: Vec<usize> = (0..count).collect();
    let mut rank: Vec<u8> = vec![0; count];
    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            let root = find(parent, parent[i]);
            parent[i] = root;
            root
        } else {
            i
        }
    }
    fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
        let mut ra = find(parent, a);
        let mut rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            std::mem::swap(&mut ra, &mut rb);
        }
        parent[rb] = ra;
        if rank[ra] == rank[rb] {
            rank[ra] += 1;
        }
    }
    let cell_size = (max_radius * 2.0).max(1.0);
    let inv_cell = 1.0 / cell_size;
    use std::collections::{HashMap, HashSet};
    let mut grid: HashMap<Cell, Vec<usize>> = HashMap::new();
    for (i, p) in positions.iter().enumerate() {
        let cx = (p.x * inv_cell).floor() as i32;
        let cy = (p.y * inv_cell).floor() as i32;
        grid.entry(Cell(cx, cy)).or_default().push(i);
    }
    let neighbor_offsets = [-1, 0, 1];
    let keys: Vec<Cell> = grid.keys().cloned().collect();
    for cell in keys {
        if let Some(indices) = grid.get(&cell) {
            for &i in indices {
                let ci = colors[i];
                let pi = positions[i];
                let ri = radii[i];
                let cx = cell.0;
                let cy = cell.1;
                for dx in neighbor_offsets {
                    for dy in neighbor_offsets {
                        let ncell = Cell(cx + dx, cy + dy);
                        if let Some(list) = grid.get(&ncell) {
                            for &j in list {
                                if j <= i {
                                    continue;
                                }
                                if colors[j] != ci {
                                    continue;
                                }
                                let pj = positions[j];
                                let rj = radii[j];
                                let delta = pj - pi;
                                let dist2 = delta.length_squared();
                                let touch = ri + rj;
                                if dist2 <= touch * touch {
                                    union(&mut parent, &mut rank, i, j);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut raw: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..count {
        let root = find(&mut parent, i);
        raw.entry(root).or_default().push(i);
    }
    let now = time.elapsed_secs();
    let mut seen: HashSet<Entity> = HashSet::new();
    for indices in raw.values() {
        let multi = indices.len() > 1;
        let mut existing_ids: HashSet<u64> = HashSet::new();
        for &idx in indices {
            if let Some(p) = persistence.map.get(&entities[idx]) {
                existing_ids.insert(p.cluster_id);
            }
        }
        let target_id = if let Some(&id) = existing_ids.iter().next() {
            id
        } else {
            let id = persistence.next_cluster_id;
            persistence.next_cluster_id += 1;
            id
        };
        if existing_ids.len() > 1 {
            for (_e, p) in persistence.map.iter_mut() {
                if existing_ids.contains(&p.cluster_id) {
                    p.cluster_id = target_id;
                }
            }
        }
        for &idx in indices {
            let e = entities[idx];
            let color = colors[idx];
            let entry = persistence.map.entry(e).or_insert(BallPersist {
                cluster_id: target_id,
                last_touch_time: now,
                color_index: color,
            });
            entry.color_index = color;
            if multi {
                entry.last_touch_time = now;
            }
            seen.insert(e);
        }
    }
    let current_set: HashSet<Entity> = entities.iter().copied().collect();
    persistence.map.retain(|e, _| current_set.contains(e));
    let mut to_reassign: Vec<Entity> = Vec::new();
    for (e, p) in persistence.map.iter() {
        let isolated_long_enough = now - p.last_touch_time > DETACH_THRESHOLD;
        if isolated_long_enough {
            to_reassign.push(*e);
        }
    }
    for e in to_reassign {
        let new_id = persistence.next_cluster_id;
        persistence.next_cluster_id += 1;
        if let Some(p) = persistence.map.get_mut(&e) {
            p.cluster_id = new_id;
        }
    }
    let mut agg: HashMap<u64, Cluster> = HashMap::new();
    for (e, p) in persistence.map.iter() {
        if let Some(idx) = entities.iter().position(|ee| ee == e) {
            let cl = agg
                .entry(p.cluster_id)
                .or_insert_with(|| Cluster::new(p.color_index));
            cl.entities.push(*e);
            let pos = positions[idx];
            let r = radii[idx];
            let area = std::f32::consts::PI * r * r;
            cl.total_area += area;
            cl.centroid += pos * area;
            let min_p = pos - Vec2::splat(r);
            let max_p = pos + Vec2::splat(r);
            if min_p.x < cl.min.x {
                cl.min.x = min_p.x;
            }
            if min_p.y < cl.min.y {
                cl.min.y = min_p.y;
            }
            if max_p.x > cl.max.x {
                cl.max.x = max_p.x;
            }
            if max_p.y > cl.max.y {
                cl.max.y = max_p.y;
            }
        }
    }
    clusters.0 = agg.into_values().collect();
    for cl in clusters.0.iter_mut() {
        if cl.total_area > 0.0 {
            cl.centroid /= cl.total_area;
        }
    }

    // Rebuild reverse index (ball -> cluster index)
    for (idx, cl) in clusters.0.iter().enumerate() {
        for e in cl.entities.iter() {
            cluster_index.0.insert(*e, idx);
        }
    }
}
fn debug_draw_clusters(
    clusters: Res<Clusters>,
    _display: Option<Res<BallDisplayMaterials>>,
    mut gizmos: Gizmos,
    cfg: Option<Res<crate::core::config::GameConfig>>,
) {
    if let Some(cfg) = cfg {
        if !cfg.draw_cluster_bounds {
            return;
        }
    }
    for cl in clusters.0.iter() {
        let min = cl.min;
        let max = cl.max;
        let size = max - min;
        if !size.x.is_finite() {
            continue;
        }
        let center = min + size * 0.5;
        let color = color_for_index(cl.color_index);
        gizmos.rect_2d(Isometry2d::from_translation(center), size, color);
    }
}
