/// Spatial grid acceleration for metaball rendering
///
/// This module provides CPU-side spatial partitioning to reduce
/// the per-pixel cost of metaball field computation from O(N) to O(k)
/// where k is the local ball density.
use crate::internal::{BallGpu, GridCell, SpatialGrid, GRID_CELL_SIZE};
use bevy::prelude::*;

/// Build a spatial grid from ball data
pub fn build_spatial_grid(balls: &[BallGpu], screen_size: Vec2) -> SpatialGrid {
    // Calculate grid dimensions
    let grid_width = ((screen_size.x / GRID_CELL_SIZE).ceil() as u32).max(1);
    let grid_height = ((screen_size.y / GRID_CELL_SIZE).ceil() as u32).max(1);
    let dimensions = UVec2::new(grid_width, grid_height);
    let total_cells = (grid_width * grid_height) as usize;

    // Initialize cell data
    let mut cell_data = vec![GridCell::default(); total_cells];

    // First pass: count balls per cell
    let mut cell_counts = vec![0u32; total_cells];
    let mut ball_cell_assignments = Vec::with_capacity(balls.len());

    for (ball_idx, ball) in balls.iter().enumerate() {
        let cells = get_influenced_cells(ball, dimensions, screen_size);
        for cell_id in cells {
            cell_counts[cell_id as usize] += 1;
            ball_cell_assignments.push((cell_id, ball_idx as u32));
        }
    }

    // Second pass: compute offsets (prefix sum)
    let mut offset = 0u32;
    for (cell_id, count) in cell_counts.iter().enumerate() {
        cell_data[cell_id].offset = offset;
        cell_data[cell_id].count = *count;
        offset += count;
    }

    // Third pass: populate ball indices array
    let total_entries = offset as usize;
    let mut ball_indices = vec![0u32; total_entries];
    let mut write_offsets = cell_data.iter().map(|c| c.offset).collect::<Vec<_>>();

    for (cell_id, ball_idx) in ball_cell_assignments {
        let write_pos = write_offsets[cell_id as usize];
        ball_indices[write_pos as usize] = ball_idx;
        write_offsets[cell_id as usize] += 1;
    }

    SpatialGrid {
        dimensions,
        ball_indices,
        cell_data,
    }
}

/// Calculate which grid cells a ball influences based on its radius
fn get_influenced_cells(ball: &BallGpu, grid_dimensions: UVec2, screen_size: Vec2) -> Vec<u32> {
    let mut cells = Vec::new();

    // Ball's influence extends to about 2-3 radii (metaball falloff)
    let influence_radius = ball.radius * 3.0;

    let min_x = (ball.center[0] - influence_radius).max(0.0);
    let max_x = (ball.center[0] + influence_radius).min(screen_size.x);
    let min_y = (ball.center[1] - influence_radius).max(0.0);
    let max_y = (ball.center[1] + influence_radius).min(screen_size.y);

    let cell_min_x = (min_x / GRID_CELL_SIZE) as u32;
    let cell_max_x = ((max_x / GRID_CELL_SIZE) as u32).min(grid_dimensions.x - 1);
    let cell_min_y = (min_y / GRID_CELL_SIZE) as u32;
    let cell_max_y = ((max_y / GRID_CELL_SIZE) as u32).min(grid_dimensions.y - 1);

    for y in cell_min_y..=cell_max_y {
        for x in cell_min_x..=cell_max_x {
            let cell_id = y * grid_dimensions.x + x;
            cells.push(cell_id);
        }
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_construction() {
        let balls = vec![
            BallGpu {
                center: [100.0, 100.0],
                radius: 20.0,
                cluster_id: 0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
            BallGpu {
                center: [200.0, 200.0],
                radius: 20.0,
                cluster_id: 0,
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let grid = build_spatial_grid(&balls, Vec2::new(512.0, 512.0));

        // Basic sanity checks
        assert!(grid.dimensions.x > 0);
        assert!(grid.dimensions.y > 0);
        assert_eq!(
            grid.cell_data.len(),
            (grid.dimensions.x * grid.dimensions.y) as usize
        );

        // Should have entries for both balls
        let total_entries: u32 = grid.cell_data.iter().map(|c| c.count).sum();
        assert!(
            total_entries >= 2,
            "Expected at least 2 entries, got {}",
            total_entries
        );
    }

    #[test]
    fn test_influenced_cells() {
        let ball = BallGpu {
            center: [100.0, 100.0],
            radius: 10.0,
            cluster_id: 0,
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let grid_dims = UVec2::new(8, 8); // 512x512 / 64 = 8x8 grid
        let cells = get_influenced_cells(&ball, grid_dims, Vec2::new(512.0, 512.0));

        assert!(!cells.is_empty(), "Ball should influence at least one cell");

        // All cell IDs should be valid
        for cell_id in cells {
            assert!(cell_id < grid_dims.x * grid_dims.y);
        }
    }
}
