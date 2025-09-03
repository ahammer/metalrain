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
    pub id: u64, // stable cluster id
}
impl Cluster {
    fn new(color_index: usize, id: u64) -> Self {
        Self {
            color_index,
            entities: Vec::new(),
            min: Vec2::splat(f32::INFINITY),
            max: Vec2::splat(f32::NEG_INFINITY),
            centroid: Vec2::ZERO,
            total_area: 0.0,
            id,
        }
    }
}
#[derive(Resource, Default, Debug, Clone)]
pub struct Clusters(pub Vec<Cluster>);

/// Reverse index: ball Entity -> cluster index (into `Clusters.0`)
#[derive(Resource, Default, Debug, Clone)]
pub struct BallClusterIndex(pub std::collections::HashMap<Entity, usize>);

#[derive(Debug, Clone)]
struct PersistEntry {
    cluster_id: u64,
}
#[derive(Resource, Default, Debug, Clone)]
pub struct ClusterPersistence {
    map: std::collections::HashMap<Entity, PersistEntry>,
    pub next_cluster_id: u64,
}

/// Core clustering logic plugin.
pub struct ClusterCorePlugin;
impl Plugin for ClusterCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clusters>()
            .init_resource::<BallClusterIndex>()
            .init_resource::<ClusterPersistence>()
            .add_systems(Update, compute_clusters.in_set(PostPhysicsAdjustSet));
    }
}

/// Optional debug drawing for clusters.
pub struct ClusterDebugPlugin;
impl Plugin for ClusterDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_draw_clusters.after(compute_clusters));
    }
}

/// Umbrella plugin.
pub struct ClusterPlugin;
impl Plugin for ClusterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ClusterCorePlugin, ClusterDebugPlugin));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct Cell(i32, i32);

pub fn compute_clusters(
    mut clusters: ResMut<Clusters>,
    mut cluster_index: ResMut<BallClusterIndex>,
    mut persistence: ResMut<ClusterPersistence>,
    q: Query<ClusterQueryItem<'_>, With<Ball>>,
    cfg: Option<Res<crate::core::config::GameConfig>>,
) {
    clusters.0.clear();
    cluster_index.0.clear();

    let exclude_popping = cfg
        .as_ref()
        .map(|g| g.interactions.cluster_pop.exclude_from_new_clusters)
        .unwrap_or(true);

    // Collect included balls
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
        // Clear persistence of removed entities
        persistence.map.clear();
        return;
    }

    // Hysteresis thresholds (clamped logically)
    let enter = cfg
        .as_ref()
        .map(|c| c.clustering.distance_buffer_enter_cluster)
        .unwrap_or(1.2)
        .clamp(1.0, 3.0);
    let exit = cfg
        .as_ref()
        .map(|c| c.clustering.distance_buffer_exit_cluster)
        .unwrap_or(1.25)
        .clamp(enter, 3.0);


    // Union-find
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

    // Spatial hash
    let cell_size = (max_radius * 2.0 * exit).max(1.0); // use largest buffer for coverage
    let inv_cell = 1.0 / cell_size;
    use std::collections::HashMap;
    let mut grid: HashMap<Cell, Vec<usize>> = HashMap::new();
    for (i, p) in positions.iter().enumerate() {
        let cx = (p.x * inv_cell).floor() as i32;
        let cy = (p.y * inv_cell).floor() as i32;
        grid.entry(Cell(cx, cy)).or_default().push(i);
    }
    let neighbor_offsets = [-1, 0, 1];
    let keys: Vec<Cell> = grid.keys().cloned().collect();

    // Pre-fetch previous cluster ids for hysteresis
    let mut prev_cluster_ids: Vec<Option<u64>> = Vec::with_capacity(count);
    for e in entities.iter() {
        prev_cluster_ids.push(persistence.map.get(e).map(|p| p.cluster_id));
    }

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
                                let sum_r = ri + rj;
                                let enter_thresh = sum_r * enter;
                                let exit_thresh = sum_r * exit;
                                if dist2 <= enter_thresh * enter_thresh {
                                    union(&mut parent, &mut rank, i, j);
                                } else if dist2 <= exit_thresh * exit_thresh {
                                    // Only keep if previously same cluster (hysteresis)
                                    if let (Some(a), Some(b)) =
                                        (prev_cluster_ids[i], prev_cluster_ids[j])
                                    {
                                        if a == b {
                                            union(&mut parent, &mut rank, i, j);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Aggregate by root (collect indices)
    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..count {
        let root = find(&mut parent, i);
        comps.entry(root).or_default().push(i);
    }

    // Determine stable cluster IDs:
    // For each component, gather existing ids among members; reuse smallest, else allocate new.
    let mut new_clusters: Vec<Cluster> = Vec::with_capacity(comps.len());
    for indices in comps.values() {
        // Collect existing persistence ids
        let mut existing_ids: Vec<u64> = indices
            .iter()
            .filter_map(|&idx| prev_cluster_ids[idx])
            .collect();
        existing_ids.sort_unstable();
        existing_ids.dedup();
        let cluster_id = if let Some(id) = existing_ids.first() {
            *id
        } else {
            let id = persistence.next_cluster_id;
            persistence.next_cluster_id += 1;
            id
        };

        // If multiple previous ids merged, no need to rewrite anything yet; later we update entries.

        let color_index = colors[indices[0]];
        let mut cl = Cluster::new(color_index, cluster_id);
        for &idx in indices {
            cl.entities.push(entities[idx]);
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
        new_clusters.push(cl);
    }

    for cl in new_clusters.iter_mut() {
        if cl.total_area > 0.0 {
            cl.centroid /= cl.total_area;
        }
    }

    // Update persistence map to only current entities
    let current_set: std::collections::HashSet<Entity> =
        entities.iter().copied().collect();
    persistence.map.retain(|e, _| current_set.contains(e));

    // Write new persistence entries (merging any old cluster ids automatically reuses chosen id)
    for cl in new_clusters.iter() {
        for &e in cl.entities.iter() {
            persistence
                .map
                .insert(e, PersistEntry { cluster_id: cl.id });
        }
    }

    clusters.0 = new_clusters;

    // Rebuild reverse index
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

#[cfg(test)]
mod tests {
    use super::*;


    fn setup_app(enter: f32, exit: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(TransformPlugin);
        app.add_plugins(ClusterCorePlugin);
        let mut cfg = crate::core::config::GameConfig::default();
        cfg.clustering.distance_buffer_enter_cluster = enter;
        cfg.clustering.distance_buffer_exit_cluster = exit;
        app.insert_resource(cfg);
        app
    }

    fn spawn_ball(app: &mut App, pos: Vec2, radius: f32, color: usize) -> Entity {
        app.world_mut().spawn((
            Ball,
            BallRadius(radius),
            BallMaterialIndex(color),
            Transform::from_xyz(pos.x, pos.y, 0.0),
            GlobalTransform::default(),
        )).id()
    }

    #[test]
    fn within_enter_forms_cluster() {
        let mut app = setup_app(1.2, 1.25);
        spawn_ball(&mut app, Vec2::new(0.0, 0.0), 10.0, 0);
        spawn_ball(&mut app, Vec2::new(23.0, 0.0), 10.0, 0); // < 24 enter
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 1);
        assert_eq!(clusters.0[0].entities.len(), 2);
    }

    #[test]
    fn outside_enter_not_join_first_frame() {
        let mut app = setup_app(1.2, 1.25);
        spawn_ball(&mut app, Vec2::new(0.0, 0.0), 10.0, 0);
        spawn_ball(&mut app, Vec2::new(25.0, 0.0), 10.0, 0); // > 24 enter, > 25 exit? exit = (10+10)*1.25=25 => boundary
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
    }

    #[test]
    fn hysteresis_keeps_pair_until_exit() {
        let mut app = setup_app(1.2, 1.25);
        // Start within enter
    let _e1 = spawn_ball(&mut app, Vec2::new(0.0, 0.0), 10.0, 0);
        let e2 = spawn_ball(&mut app, Vec2::new(23.0, 0.0), 10.0, 0);
        app.update();
        {
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 1);
        }
        // Move second ball just outside enter (24.5) but inside exit (25)
        {
            let mut t = app.world_mut().get_mut::<Transform>(e2).unwrap();
            t.translation.x = 24.5;
        }
        app.update();
        {
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 1, "should persist due to exit hysteresis");
        }
        // Move beyond exit (25.5)
        {
            let mut t = app.world_mut().get_mut::<Transform>(e2).unwrap();
            t.translation.x = 25.5;
        }
        app.update();
        {
            let clusters = app.world().resource::<Clusters>();
            assert_eq!(clusters.0.len(), 2, "pair should split after exceeding exit");
        }
    }

    #[test]
    fn different_colors_do_not_merge() {
        let mut app = setup_app(1.2, 1.25);
        spawn_ball(&mut app, Vec2::new(0.0, 0.0), 10.0, 0);
        spawn_ball(&mut app, Vec2::new(20.0, 0.0), 10.0, 1);
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
    }
}
