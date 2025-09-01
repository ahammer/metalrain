use bevy::prelude::*;

// =====================================================================================
// CPU Palette Construction (Phase A scaffold)
// =====================================================================================

/// CPU-side palette (sorted by stable cluster id). Each entry is RGBA (vec4<f32> on GPU).
#[derive(Debug, Default, Clone)]
pub struct ClusterPaletteCpu {
    pub colors: Vec<[f32; 4]>,
    pub ids: Vec<u64>, // parallel array of cluster ids for mapping / debugging
}

impl ClusterPaletteCpu {
    pub fn clear(&mut self) {
        self.colors.clear();
        self.ids.clear();
    }
}

// GPU palette storage (capacity doubling strategy)
#[derive(Resource, Debug, Clone)]
pub struct ClusterPaletteStorage {
    pub handle: Option<Handle<bevy::render::storage::ShaderStorageBuffer>>,
    pub capacity: u32,
    pub length: u32,
}
impl Default for ClusterPaletteStorage { fn default() -> Self { Self { handle: None, capacity: 0, length: 0 } } }

pub const PALETTE_INITIAL_CAPACITY: u32 = 512;
pub const PALETTE_MAX_CAPACITY: u32 = 16_384; // safety ceiling

pub fn ensure_palette_capacity(
    storage: &mut ClusterPaletteStorage,
    needed: u32,
    buffers: &mut Assets<bevy::render::storage::ShaderStorageBuffer>,
) {
    if needed == 0 { return; }
    if storage.capacity >= needed { storage.length = needed; return; }
    let mut new_cap = if storage.capacity == 0 { PALETTE_INITIAL_CAPACITY } else { storage.capacity }; 
    while new_cap < needed { new_cap = (new_cap * 2).min(PALETTE_MAX_CAPACITY); if new_cap == PALETTE_MAX_CAPACITY { break; } }
    if needed > new_cap { // truncated
        storage.length = new_cap;
    } else { storage.length = needed; }
    // allocate zeroed vec of new_cap entries
    let mut data: Vec<[f32;4]> = vec![[0.0;4]; new_cap as usize];
    // Fill first length later by caller
    let ssb = bevy::render::storage::ShaderStorageBuffer::from(data.as_slice());
    if let Some(h) = &storage.handle { if buffers.get(h).is_some() { if let Some(mut_ref) = buffers.get_mut(h) { *mut_ref = ssb; } else { let new_h = buffers.add(ssb); storage.handle = Some(new_h); } } else { let new_h = buffers.add(ssb); storage.handle = Some(new_h); } }
    else { let new_h = buffers.add(ssb); storage.handle = Some(new_h); }
    storage.capacity = new_cap;
}

/// Build a deterministic palette from current clusters.
/// Ordering strategy: sort ascending by `Cluster.id`.
pub fn build_cpu_palette(clusters: &crate::physics::clustering::cluster::Clusters) -> ClusterPaletteCpu {
    use crate::rendering::palette::palette::color_for_index; // existing color source
    let mut pairs: Vec<(u64, [f32;4])> = Vec::with_capacity(clusters.0.len());
    for cl in clusters.0.iter() {
        let c = color_for_index(cl.color_index).to_srgba();
        pairs.push((cl.id, [c.red, c.green, c.blue, 1.0]));
    }
    pairs.sort_unstable_by_key(|(id, _)| *id);
    let mut out = ClusterPaletteCpu::default();
    out.colors.reserve(pairs.len());
    out.ids.reserve(pairs.len());
    for (id, col) in pairs.into_iter() { out.ids.push(id); out.colors.push(col); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::clustering::cluster::{Cluster, Clusters};

    #[test]
    fn ordering_deterministic_by_id() {
        // Create synthetic clusters with out-of-order ids and different colors.
        let mut clusters = Clusters::default();
    clusters.0.push(Cluster { id: 10, color_index: 2, entities: vec![], min: Default::default(), max: Default::default(), centroid: Default::default(), total_area: 0.0 });
    clusters.0.push(Cluster { id: 2, color_index: 5, entities: vec![], min: Default::default(), max: Default::default(), centroid: Default::default(), total_area: 0.0 });
    clusters.0.push(Cluster { id: 5, color_index: 1, entities: vec![], min: Default::default(), max: Default::default(), centroid: Default::default(), total_area: 0.0 });

        let palette1 = build_cpu_palette(&clusters);

        // Shuffle insertion order (simulate next frame discovery order change)
        clusters.0.swap(0,2); // now ids order [5,2,10]
        let palette2 = build_cpu_palette(&clusters);

        assert_eq!(palette1.ids, palette2.ids, "Sorted ids must be stable across frames");
        // Basic sanity: ascending order
        assert!(palette1.ids.windows(2).all(|w| w[0] < w[1]));
        // Length matches cluster count
        assert_eq!(palette1.ids.len(), clusters.0.len());
        assert_eq!(palette1.colors.len(), palette1.ids.len());
    }
}
