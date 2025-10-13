use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

pub const WORKGROUP_SIZE: u32 = 8;

// Spatial grid constants
pub const MAX_BALLS: u32 = 4096;
pub const GRID_CELL_SIZE: f32 = 64.0; // Grid cell size in pixels

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BallGpu {
    pub center: [f32; 2],
    pub radius: f32,
    pub cluster_id: i32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(
    Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource,
)]
pub struct TimeUniform {
    pub time: f32,
    _pad: [f32; 3],
}

#[repr(C, align(16))]
#[derive(
    Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource,
)]
pub struct ParamsUniform {
    pub screen_size: [f32; 2],
    pub num_balls: u32,
    pub clustering_enabled: u32,
    pub grid_dimensions: [u32; 2],
    pub active_ball_count: u32,
    pub _pad: u32,
}

impl ParamsUniform {
    pub fn new(screen_size: [f32; 2], clustering_enabled: bool) -> Self {
        Self {
            screen_size,
            num_balls: 0,
            clustering_enabled: if clustering_enabled { 1 } else { 0 },
            grid_dimensions: [0, 0],
            active_ball_count: 0,
            _pad: 0,
        }
    }
}

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct FieldTexture(pub Handle<Image>);
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct AlbedoTexture(pub Handle<Image>);

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct NormalTexture(pub Handle<Image>);

/// Fixed-capacity ball buffer with free list management
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct BallBuffer {
    /// Fixed-size array of ball data (some slots may be inactive)
    pub balls: Vec<BallGpu>,
    /// Free list of available indices for reuse
    pub free_indices: Vec<u32>,
    /// Number of active balls (not counting free slots)
    pub active_count: u32,
}

impl Default for BallBuffer {
    fn default() -> Self {
        Self {
            balls: Vec::new(),
            free_indices: Vec::new(),
            active_count: 0,
        }
    }
}

impl BallBuffer {
    /// Allocate a slot for a new ball, reusing freed indices if available
    pub fn allocate_slot(&mut self) -> Option<u32> {
        if let Some(idx) = self.free_indices.pop() {
            self.active_count += 1;
            Some(idx)
        } else if self.balls.len() < MAX_BALLS as usize {
            let idx = self.balls.len() as u32;
            self.balls.push(BallGpu::default());
            self.active_count += 1;
            Some(idx)
        } else {
            None // Buffer full
        }
    }

    /// Free a slot for reuse
    pub fn free_slot(&mut self, idx: u32) {
        if (idx as usize) < self.balls.len() {
            self.free_indices.push(idx);
            self.active_count = self.active_count.saturating_sub(1);
        }
    }

    /// Update ball data at a specific index
    pub fn update_ball(&mut self, idx: u32, ball: BallGpu) {
        if let Some(slot) = self.balls.get_mut(idx as usize) {
            *slot = ball;
        }
    }
}

/// Grid cell entry: (cell_id, ball_index)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridCell {
    /// Offset into the ball_indices buffer
    pub offset: u32,
    /// Number of balls in this cell
    pub count: u32,
}

/// Spatial grid data computed on CPU each frame
#[derive(Resource, Clone, Debug, ExtractResource, Default)]
pub struct SpatialGrid {
    /// Grid dimensions (width, height)
    pub dimensions: UVec2,
    /// Flattened array of ball indices, sorted by grid cell
    pub ball_indices: Vec<u32>,
    /// Index table: offset and count for each grid cell
    pub cell_data: Vec<GridCell>,
}
