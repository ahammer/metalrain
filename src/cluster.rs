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

/// Resource storing all clusters for the current frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct Clusters(pub Vec<Cluster>);

/// Plugin computing clusters each frame after physics/separation so transforms are settled.
pub struct ClusterPlugin;

impl Plugin for ClusterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Clusters>()
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
    q: Query<(Entity, &Transform, &BallRadius, &BallMaterialIndex), With<Ball>>,
) {
    let count = q.iter().count();
    clusters.0.clear();
    if count == 0 { return; }

    // Collect data into temporary arrays (stable index mapping 0..n)
    let mut entities: Vec<Entity> = Vec::with_capacity(count);
    let mut positions: Vec<Vec2> = Vec::with_capacity(count);
    let mut radii: Vec<f32> = Vec::with_capacity(count);
    let mut colors: Vec<usize> = Vec::with_capacity(count);
    let mut max_radius = 0.0f32;
    for (e, t, r, c) in q.iter() {
        entities.push(e);
        let p = t.translation.truncate();
        positions.push(p);
        radii.push(r.0);
        colors.push(c.0);
        if r.0 > max_radius { max_radius = r.0; }
    }

    // Union-Find (Disjoint Set)
    let mut parent: Vec<usize> = (0..count).collect();
    let mut rank: Vec<u8> = vec![0; count];
    fn find(parent: &mut [usize], i: usize) -> usize { if parent[i] != i { let root = find(parent, parent[i]); parent[i] = root; root } else { i } }
    fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
        let mut ra = find(parent, a); let mut rb = find(parent, b);
        if ra == rb { return; }
        if rank[ra] < rank[rb] { std::mem::swap(&mut ra, &mut rb); }
        parent[rb] = ra;
        if rank[ra] == rank[rb] { rank[ra] += 1; }
    }

    // Spatial hash: cell size = 2 * max_radius (guarantee touching balls reside in same or adjacent cells)
    let cell_size = (max_radius * 2.0).max(1.0); // avoid zero
    let inv_cell = 1.0 / cell_size;
    use bevy::utils::HashMap;
    let mut grid: HashMap<Cell, Vec<usize>> = HashMap::default();
    for (i, p) in positions.iter().enumerate() {
        let cx = (p.x * inv_cell).floor() as i32;
        let cy = (p.y * inv_cell).floor() as i32;
        grid.entry(Cell(cx, cy)).or_default().push(i);
    }

    // Neighbor search (consider 9 neighboring cells)
    let neighbor_offsets = [-1, 0, 1];
    // To iterate deterministically over grid cells, collect keys
    let keys: Vec<Cell> = grid.keys().cloned().collect();
    for cell in keys {
        if let Some(indices) = grid.get(&cell) {
            for &i in indices {
                let ci = colors[i];
                let pi = positions[i];
                let ri = radii[i];
                let cx = cell.0; let cy = cell.1;
                for dx in neighbor_offsets { for dy in neighbor_offsets {
                    let ncell = Cell(cx + dx, cy + dy);
                    if let Some(list) = grid.get(&ncell) {
                        for &j in list {
                            if j <= i { continue; } // avoid double & self
                            if colors[j] != ci { continue; }
                            let pj = positions[j];
                            let rj = radii[j];
                            let delta = pj - pi;
                            let dist2 = delta.length_squared();
                            let touch = ri + rj; // slop=1.0 (could add config)
                            if dist2 <= touch * touch { union(&mut parent, &mut rank, i, j); }
                        }
                    }
                }}
            }
        }
    }

    // Gather clusters by root
    let mut map: HashMap<usize, usize> = HashMap::default(); // root -> cluster index
    for i in 0..count {
        let root = find(&mut parent, i);
        let color = colors[i];
        let entry = map.entry(root).or_insert_with(|| { clusters.0.push(Cluster::new(color)); clusters.0.len() - 1 });
        let cl = &mut clusters.0[*entry];
        cl.entities.push(entities[i]);
        let p = positions[i];
        let r = radii[i];
        let area = std::f32::consts::PI * r * r;
        cl.total_area += area;
        cl.centroid += p * area; // area-weighted centroid
        let min_p = p - Vec2::splat(r);
        let max_p = p + Vec2::splat(r);
        if min_p.x < cl.min.x { cl.min.x = min_p.x; }
        if min_p.y < cl.min.y { cl.min.y = min_p.y; }
        if max_p.x > cl.max.x { cl.max.x = max_p.x; }
        if max_p.y > cl.max.y { cl.max.y = max_p.y; }
    }

    // finalize centroid (divide by total weighted area) & handle single-element infinite init
    for cl in clusters.0.iter_mut() {
        if cl.total_area > 0.0 { cl.centroid /= cl.total_area; }
        if !cl.min.x.is_finite() { cl.min = Vec2::ZERO; cl.max = Vec2::ZERO; }
    }
}

/// Debug rendering using Gizmos; draws an AABB per cluster in its color.
fn debug_draw_clusters(
    clusters: Res<Clusters>,
    _display: Option<Res<BallDisplayMaterials>>,
    mut gizmos: Gizmos,
) {
    for cl in clusters.0.iter() {
        let min = cl.min;
        let max = cl.max;
        let size = max - min;
        if !size.x.is_finite() { continue; }
        let center = min + size * 0.5;
        // Map color index to stable palette (duplicated from materials for debug only)
        let color = match cl.color_index % 6 {
            0 => Color::srgb(0.90, 0.20, 0.25),
            1 => Color::srgb(0.20, 0.55, 0.90),
            2 => Color::srgb(0.95, 0.75, 0.15),
            3 => Color::srgb(0.20, 0.80, 0.45),
            4 => Color::srgb(0.65, 0.45, 0.95),
            _ => Color::srgb(0.95, 0.50, 0.15),
        };
        gizmos.rect_2d(center, 0.0, size, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        let mut app = App::new();
        app.add_systems(Update, compute_clusters);
        app.init_resource::<Clusters>();
        app
    }

    #[test]
    fn singleton_clusters() {
        let mut app = make_app();
        // create two balls different colors far apart
        app.world_mut().spawn((Ball, BallRadius(5.0), BallMaterialIndex(0), Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default()));
        app.world_mut().spawn((Ball, BallRadius(5.0), BallMaterialIndex(1), Transform::from_xyz(100.0, 0.0, 0.0), GlobalTransform::default()));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2);
        assert!(clusters.0.iter().all(|c| c.entities.len() == 1));
    }

    #[test]
    fn touching_chain_same_color() {
        let mut app = make_app();
        // three balls in a horizontal line, touching (centers 2r apart)
        app.world_mut().spawn((Ball, BallRadius(10.0), BallMaterialIndex(2), Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default()));
        app.world_mut().spawn((Ball, BallRadius(10.0), BallMaterialIndex(2), Transform::from_xyz(20.0, 0.0, 0.0), GlobalTransform::default()));
        app.world_mut().spawn((Ball, BallRadius(10.0), BallMaterialIndex(2), Transform::from_xyz(40.0, 0.0, 0.0), GlobalTransform::default()));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 1, "All touching chain should be one cluster");
        assert_eq!(clusters.0[0].entities.len(), 3);
    }

    #[test]
    fn adjacent_different_colors_not_merged() {
        let mut app = make_app();
        // two touching but different color => separate clusters
        app.world_mut().spawn((Ball, BallRadius(10.0), BallMaterialIndex(0), Transform::from_xyz(0.0, 0.0, 0.0), GlobalTransform::default()));
        app.world_mut().spawn((Ball, BallRadius(10.0), BallMaterialIndex(1), Transform::from_xyz(20.0, 0.0, 0.0), GlobalTransform::default()));
        app.update();
        let clusters = app.world().resource::<Clusters>();
        assert_eq!(clusters.0.len(), 2, "Different colors must not merge");
    }
}
