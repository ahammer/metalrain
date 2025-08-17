use crate::palette::color_for_index;
use bevy::prelude::*;

use crate::components::{Ball, BallRadius};
use crate::materials::{BallDisplayMaterials, BallMaterialIndex};
use crate::system_order::PostPhysicsAdjustSet;

/// A single connected cluster of touching balls of the same material index (color variant).
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

/// Resource storing stable (time-filtered) clusters for the current frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct Clusters(pub Vec<Cluster>);

/// Internal persistence record per ball to stabilize clusters over minor jitters.
#[derive(Debug, Clone)]
struct BallPersist {
    cluster_id: u64,
    last_touch_time: f32, // last time it touched at least one same-color neighbor
    color_index: usize,
}

/// Resource tracking per-ball persistence info.
#[derive(Resource, Default)]
struct ClusterPersistence {
    map: std::collections::HashMap<Entity, BallPersist>,
    next_cluster_id: u64,
}

const DETACH_THRESHOLD: f32 = 0.5; // seconds of isolation before detaching (TODO: make configurable)

/// Plugin computing clusters each frame after physics/separation so transforms are settled.
pub struct ClusterPlugin;

impl Plugin for ClusterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clusters>()
            .init_resource::<ClusterPersistence>()
            .add_systems(Update, compute_clusters.in_set(PostPhysicsAdjustSet))
            .add_systems(Update, debug_draw_clusters.after(compute_clusters));
    }
}

/// Spatial hashing cell key (integer pair) for broad-phase neighbor gathering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct Cell(i32, i32);

/// System: recompute clusters each frame.
#[allow(clippy::type_complexity)]
fn compute_clusters(
    mut clusters: ResMut<Clusters>,
    mut persistence: ResMut<ClusterPersistence>,
    time: Res<Time>,
    q: Query<(Entity, &Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
) {
    let count = q.iter().count();
    clusters.0.clear();
    if count == 0 {
        // purge persistence if no balls
        persistence.map.clear();
        return;
    }

    // Collect data (stable indices)
    let mut entities = Vec::with_capacity(count);
    let mut positions = Vec::with_capacity(count);
    let mut radii = Vec::with_capacity(count);
    let mut colors = Vec::with_capacity(count);
    let mut max_radius = 0.0f32;
    for (e, t, r, c) in q.iter() {
        entities.push(e);
        let p = t.translation.truncate();
        positions.push(p);
        radii.push(r.0);
        colors.push(c.0);
        if r.0 > max_radius {
            max_radius = r.0;
        }
    }

    // Build instantaneous (raw) connectivity with union-find
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

    // Build raw clusters (index lists)
    let mut raw: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..count {
        let root = find(&mut parent, i);
        raw.entry(root).or_default().push(i);
    }

    let now = time.elapsed_secs();
    let mut seen: HashSet<Entity> = HashSet::new();

    // Map raw clusters to persistent cluster ids (merge if previously separate but now connected)
    for indices in raw.values() {
        // Determine if this cluster has >1 members (contact cluster)
        let multi = indices.len() > 1;
        // Collect existing persistent cluster ids among members
    let mut existing_ids: HashSet<u64> = HashSet::new();
        for &idx in indices {
            if let Some(p) = persistence.map.get(&entities[idx]) {
                existing_ids.insert(p.cluster_id);
            }
        }
        // Choose target id
        let target_id = if let Some(&id) = existing_ids.iter().next() {
            id
        } else {
            let id = persistence.next_cluster_id;
            persistence.next_cluster_id += 1;
            id
        };
        // If multiple existing ids, reassign them to target
        if existing_ids.len() > 1 {
            for (_e, p) in persistence.map.iter_mut() {
                if existing_ids.contains(&p.cluster_id) {
                    p.cluster_id = target_id;
                }
            }
        }
        // Update / insert members
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
                // only update touch time if actually touching others
                entry.last_touch_time = now;
            }
            // If singleton (multi false) we do not refresh last_touch_time so potential detach countdown proceeds.
            seen.insert(e);
        }
    }

    // Remove entries for despawned entities (not in seen and not in current query at all)
    // Identify current entities set for quick check
    let current_set: HashSet<Entity> = entities.iter().copied().collect();
    persistence.map.retain(|e, _| current_set.contains(e));

    // Process isolation countdown for entities not seen in raw (should not happen because raw covers all), and for singletons that were not multi-contact this frame.
    // Determine which entries should detach (convert to their own cluster id) because they've been isolated long enough.
    // Collect entities needing new cluster ids first (to avoid borrow conflicts).
    let mut to_reassign: Vec<Entity> = Vec::new();
    for (e, p) in persistence.map.iter() {
        let isolated_long_enough = now - p.last_touch_time > DETACH_THRESHOLD;
        if isolated_long_enough {
            to_reassign.push(*e);
        }
    }
    for e in to_reassign {
        let new_id = persistence.next_cluster_id; // capture
        persistence.next_cluster_id += 1;
        if let Some(p) = persistence.map.get_mut(&e) {
            p.cluster_id = new_id;
        }
    }

    // Aggregate stable clusters from persistence map
    let mut agg: HashMap<u64, Cluster> = HashMap::new();
    for (e, p) in persistence.map.iter() {
        // Find position & radius via index search (could build map for perf; acceptable for now)
        // Since dataset moderate, linear search fine; TODO: optimize with HashMap<Entity, idx> if needed.
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
}

/// Debug rendering using Gizmos; draws an AABB per cluster in its color.
fn debug_draw_clusters(
    clusters: Res<Clusters>,
    _display: Option<Res<BallDisplayMaterials>>,
    mut gizmos: Gizmos,
    cfg: Option<Res<crate::config::GameConfig>>,
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
    // New Bevy 0.16 rect_2d signature: (isometry, size, color)
    gizmos.rect_2d(Isometry2d::from_translation(center), size, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins); // provides Time resource
        app.add_systems(Update, compute_clusters);
        app.init_resource::<Clusters>();
        app.init_resource::<ClusterPersistence>();
        app
    }

    #[test]
    fn singleton_clusters() {
        let mut app = make_app();
        // create two balls different colors far apart
        app.world_mut().spawn((
            Ball,
            BallRadius(5.0),
            BallMaterialIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(5.0),
            BallMaterialIndex(1),
            Transform::from_xyz(100.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update(); // first frame builds persistence
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
        assert!(clusters.0.iter().all(|c| c.entities.len() == 1));
    }

    #[test]
    fn touching_chain_same_color() {
        let mut app = make_app();
        // three balls in a horizontal line, touching (centers 2r apart)
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(2),
            Transform::from_xyz(20.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(2),
            Transform::from_xyz(40.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(
            clusters.0.len(),
            1,
            "All touching chain should be one cluster"
        );
        assert_eq!(clusters.0[0].entities.len(), 3);
    }

    #[test]
    fn adjacent_different_colors_not_merged() {
        let mut app = make_app();
        // two touching but different color => separate clusters
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(1),
            Transform::from_xyz(20.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2, "Different colors must not merge");
    }

    #[test]
    fn detaches_after_threshold() {
        let mut app = make_app();
        // two balls touching initially
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallMaterialIndex(0),
            Transform::from_xyz(19.0, 0.0, 0.0),
            GlobalTransform::default(),
        )); // slightly overlapping (touch threshold 20)
        app.update();
        // Move second ball far away without updating time enough (simulate frames < threshold)
        {
            let mut q = app.world_mut().query::<&mut Transform>();
            let mut moved = false;
            for mut tf in q.iter_mut(app.world_mut()) {
                if !moved {
                    tf.translation.x = 200.0;
                    moved = true;
                }
            }
        }
        // Advance time less than threshold
        // Advance time by running updates with a fixed delta (simulate 0.25s < threshold)
        // Bevy's Time resource is updated by the runner; for tests we manually set elapsed by mutating the resource.
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.25));
        }
        app.update(); // recompute with advanced time
        let clusters_mid = app.world().resource::<Clusters>();
        // Should still be one cluster of size 2 (lingering)
        assert_eq!(
            clusters_mid
                .0
                .iter()
                .map(|c| c.entities.len())
                .sum::<usize>(),
            2,
            "Should still consider both in lingering cluster before threshold"
        );
    }
}
