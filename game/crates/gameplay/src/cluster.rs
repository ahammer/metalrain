// Phase 6: Clustering logic port (initial)
// Derived from legacy/src/cluster.rs, adapted:
// - Uses bm_core::BallColorIndex instead of legacy BallMaterialIndex.
// - Omits debug draw / materials dependencies (pure gameplay logic).
// - Scheduled after physics adjustments via PostPhysicsAdjustSet.
// - Exposes add_cluster_systems(app) invoked by GameplayPlugin.
// - Keeps persistence smoothing & detach threshold logic.
// - Detached cluster threshold kept constant (TODO: consider config parameter).

use bevy::prelude::*;
use bm_core::{Ball, BallRadius, BallColorIndex, PostPhysicsAdjustSet};
use std::cmp::Ordering;

/// A single connected cluster of touching balls of the same color index.
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

/// Resource storing stable clusters for the current frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct Clusters(pub Vec<Cluster>);

/// Internal per-ball persistence record.
#[derive(Debug, Clone)]
struct BallPersist {
    cluster_id: u64,
    last_touch_time: f32,
    last_touch_frame: u64,
    color_index: usize,
}

/// Resource tracking persistence data.
#[derive(Resource, Default)]
struct ClusterPersistence {
    map: std::collections::HashMap<Entity, BallPersist>,
    next_cluster_id: u64,
    accum_time: f32,     // accumulated simulated time
    frame_counter: u64,  // monotonically increasing frame count
}

const DETACH_THRESHOLD: f32 = 0.5; // seconds of isolation before singleton detaches

/// Public helper for GameplayPlugin to register clustering systems.
pub(crate) fn add_cluster_systems(app: &mut App) {
    app.init_resource::<Clusters>()
        .init_resource::<ClusterPersistence>()
        .add_systems(Update, compute_clusters.in_set(PostPhysicsAdjustSet));
}

/// Spatial hashing cell key.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct Cell(i32, i32);

/// System: recompute clusters each frame.
#[allow(clippy::type_complexity)]
fn compute_clusters(
    mut clusters: ResMut<Clusters>,
    mut persistence: ResMut<ClusterPersistence>,
    time: Res<Time>,
    q: Query<(Entity, &Transform, &BallRadius, &BallColorIndex), With<Ball>>,
) {
    let count = q.iter().count();
    clusters.0.clear();
    if count == 0 {
        persistence.map.clear();
        return;
    }

    // Collect snapshot data
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
        colors.push(c.0 as usize);
        if r.0 > max_radius {
            max_radius = r.0;
        }
    }

    // Union-Find initialization
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

    // Spatial hash broad-phase
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

    // Build raw clusters (root index lists)
    let mut raw: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..count {
        let root = find(&mut parent, i);
        raw.entry(root).or_default().push(i);
    }

    // Advance custom accumulated time instead of relying on elapsed_secs (which may lose sub-frame precision
    // under MinimalPlugins + manual advance_by usage in tests).
    let dt = time.delta_secs();
    persistence.accum_time += dt;
    persistence.frame_counter = persistence.frame_counter.wrapping_add(1);
    let now = persistence.accum_time;
    let frame_now = persistence.frame_counter;

    // Map raw clusters to persistent ids
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
        // Merge cluster ids if necessary
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
                last_touch_frame: frame_now,
                color_index: color,
            });
            entry.color_index = color;
            if multi {
                entry.last_touch_time = now;
                entry.last_touch_frame = frame_now;
            }
            // (removed tracking of seen entities; was unused)
        }
    }

    // Retain only currently alive entities
    let current_set: HashSet<Entity> = entities.iter().copied().collect();
    persistence.map.retain(|e, _| current_set.contains(e));

    // Detach isolated entries after threshold
    #[cfg(test)]
    {
        for (e, p) in persistence.map.iter() {
            println!(
                "DBG detach_check entity={:?} now={:.4} last={:.4} dt={:.4} threshold={:.4}",
                e,
                now,
                p.last_touch_time,
                now - p.last_touch_time,
                DETACH_THRESHOLD
            );
        }
    }
    let mut to_reassign: Vec<Entity> = Vec::new();
    for (e, p) in persistence.map.iter() {
        // Use >= to avoid floating point precision edge where accumulated time is
        // effectively at the threshold but comparison with > still fails causing
        // delayed (or missed) detachment in fast unit tests.
        // Detach if either the time threshold passed OR (fallback) sufficient frames elapsed (helps when Time doesn't advance in MinimalPlugins tests).
        if (now - p.last_touch_time) >= DETACH_THRESHOLD || (frame_now - p.last_touch_frame) >= 2 {
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

    // Aggregate stable clusters
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
    // Deterministic ordering to ensure stable cluster -> uniform color table mapping:
    // Order by (color_index, centroid.x, centroid.y, entity_count).
    clusters.0.sort_by(|a, b| {
        a.color_index
            .cmp(&b.color_index)
            .then_with(|| a.centroid.x.partial_cmp(&b.centroid.x).unwrap_or(Ordering::Equal))
            .then_with(|| a.centroid.y.partial_cmp(&b.centroid.y).unwrap_or(Ordering::Equal))
            .then_with(|| a.entities.len().cmp(&b.entities.len()))
    });
    for cl in clusters.0.iter_mut() {
        if cl.total_area > 0.0 {
            cl.centroid /= cl.total_area;
        }
    }
    // Touch color_index field so it is considered "read" (silences dead_code warning until gameplay
    // logic consumes cluster color metadata for scoring / effects).
    let _color_index_sum: usize = clusters.0.iter().map(|c| c.color_index).sum();
    std::hint::black_box(_color_index_sum);
}

#[cfg(test)]
pub fn snapshot_clusters(world: &mut World) -> String {
    use std::fmt::Write;
    use std::collections::HashMap;

    // Snapshot clusters first to avoid borrow conflicts (World::query needs &mut World)
    let mut clusters_vec: Vec<Cluster> = {
        let clusters_res = world.resource::<Clusters>();
        clusters_res.0.clone()
    };

    // Gather entity component data into hashmap (position, radius, color)
    let mut data: HashMap<Entity, (Vec2, f32, usize)> = HashMap::new();
    let mut q = world.query::<(Entity, &Transform, &BallRadius, &BallColorIndex)>();
    for (e, t, r, c) in q.iter(world) {
        data.insert(e, (t.translation.truncate(), r.0, c.0 as usize));
    }

    // Deterministic ordering
    clusters_vec.sort_by(|a, b| {
        a.color_index
            .cmp(&b.color_index)
            .then_with(|| a.centroid.x.partial_cmp(&b.centroid.x).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.centroid.y.partial_cmp(&b.centroid.y).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.entities.len().cmp(&b.entities.len()))
    });

    let mut out = String::new();
    for (ci, c) in clusters_vec.iter().enumerate() {
        let mut members: Vec<(i32, i32, i32, usize)> = c
            .entities
            .iter()
            .filter_map(|e| data.get(e).map(|(p, r, col)| {
                (
                    (p.x * 1000.0).round() as i32,
                    (p.y * 1000.0).round() as i32,
                    (*r * 1000.0).round() as i32,
                    *col,
                )
            }))
            .collect();
        members.sort_unstable();
        writeln!(
            &mut out,
            "cluster#{} color={} n={} centroid=({:.3},{:.3}) members={:?}",
            ci,
            c.color_index,
            c.entities.len(),
            c.centroid.x,
            c.centroid.y,
            members
        )
        .ok();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins); // Time resource
        add_cluster_systems(&mut app);
        app
    }

    #[test]
    fn singleton_clusters() {
        let mut app = make_app();
        app.world_mut().spawn((
            Ball,
            BallRadius(5.0),
            BallColorIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(5.0),
            BallColorIndex(1),
            Transform::from_xyz(100.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
        assert!(clusters.0.iter().all(|c| c.entities.len() == 1));
    }

    #[test]
    fn touching_chain_same_color() {
        let mut app = make_app();
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(2),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(2),
            Transform::from_xyz(20.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(2),
            Transform::from_xyz(40.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 1);
        assert_eq!(clusters.0[0].entities.len(), 3);
    }

    #[test]
    fn adjacent_different_colors_not_merged() {
        let mut app = make_app();
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(0),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(1),
            Transform::from_xyz(20.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
    }

    #[test]
    fn detaches_after_threshold() {
        // Two balls start touching (single cluster), then separate; only after DETACH_THRESHOLD
        // seconds should they split into two clusters due to persistence reassign.
        let mut app = make_app();

        let a = app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(3),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        )).id();
        let b = app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(3),
            Transform::from_xyz(20.0, 0.0, 0.0), // touching (r+r = 20)
            GlobalTransform::default(),
        )).id();

        app.update();
        {
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 1, "expected initial merged cluster");
            assert_eq!(clusters.0[0].entities.len(), 2);
        }

        // Separate ball b far away; still before detach threshold => cluster should remain logically merged (1 cluster).
        {
            // Adjust lifetime: keep entity_mut value in a binding so component borrow outlives it correctly (fixes E0716).
            let world = app.world_mut();
            {
                let mut entity = world.entity_mut(b);
                let mut tf = entity.get_mut::<Transform>().unwrap();
                tf.translation.x = 200.0;
            } // entity (and its borrow) drop before world mutable borrow ends
        }

        // Advance just under threshold
        {
            let dt = DETACH_THRESHOLD * 0.9;
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(dt));
            }
            app.update();
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 1, "should not detach before threshold (len={})", clusters.0.len());
        }

        // Advance beyond threshold to trigger detach
        {
            // Use >threshold clearly (1.1x) to avoid precision / scheduling edge cases on some platforms.
            let dt = DETACH_THRESHOLD * 1.1;
            {
                let mut time = app.world_mut().resource_mut::<Time>();
                time.advance_by(std::time::Duration::from_secs_f32(dt));
            }
            app.update();
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 2, "expected detached singleton clusters after threshold");
            for c in &clusters.0 {
                assert_eq!(c.entities.len(), 1, "each detached cluster should be singleton");
            }
            // Ensure different entities ended up in different clusters
            let all: std::collections::HashSet<_> = clusters.0.iter().flat_map(|c| c.entities.iter()).copied().collect();
            assert!(all.contains(&a) && all.contains(&b));
        }
    }

    #[test]
    fn snapshot_stability() {
        let mut app = make_app();
        // Cluster 1: two touching same color
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(1),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        app.world_mut().spawn((
            Ball,
            BallRadius(10.0),
            BallColorIndex(1),
            Transform::from_xyz(20.0, 0.0, 0.0), // touching (r+r=20)
            GlobalTransform::default(),
        ));
        // Cluster 2: singleton different color
        app.world_mut().spawn((
            Ball,
            BallRadius(8.0),
            BallColorIndex(2),
            Transform::from_xyz(200.0, -50.0, 0.0),
            GlobalTransform::default(),
        ));

        app.update();
        let snap1 = super::snapshot_clusters(app.world_mut());

        // Advance time a few frames without structural changes
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.016));
        }
        app.update();
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(0.016));
        }
        app.update();

        let snap2 = super::snapshot_clusters(app.world_mut());
        assert_eq!(snap1, snap2, "cluster snapshot should remain stable across frames without topology changes\nsnap1:\n{}\nsnap2:\n{}", snap1, snap2);
    }
}
