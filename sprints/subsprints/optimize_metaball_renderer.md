# Prompt: Optimize the Metaball Renderer

## Subject: `metaball_renderer` Performance Overhaul

## Context

The current `metaball_renderer` is a naive, brute-force implementation that is unacceptable for production. It iterates through every ball for every pixel, resulting in `O(width * height * ball_count)` complexity. Its memory management is even worse, causing GPU pipeline stalls by reallocating the entire storage buffer whenever the ball count changes. This will not scale and is a performance disaster waiting to happen.

The system requirements are clear: balls are dynamic. They are constantly moving, being created, and being destroyed by a physics engine. The rendering system must be engineered to handle this reality efficiently.

## Mandate

You will refactor the `metaball_renderer` crate to implement a spatially-accelerated rendering architecture. The goal is to decouple the rendering cost from the total number of balls on screen, instead tying it to the local density of balls.

## Core Requirements

### 1. Implement Spatial Partitioning (Chunking)

The brute-force approach is forbidden. You will implement a spatial grid to accelerate the metaball calculations.

- **Grid Structure**: Divide the screen space into a uniform grid (e.g., 32x32 cells). This will be our "chunking" mechanism.
- **CPU-Side Preparation**: Every frame, before the render pass, you will perform the following steps on the CPU:
  1. Create a mapping from each grid cell to the balls that influence it. A ball's radius determines its area of influence.
  2. Generate a flattened, sorted list of `(cell_id, ball_index)` pairs. This data structure is critical for efficient lookup on the GPU.
- **GPU Data Structures**: You will create and manage the GPU buffers necessary to represent this spatial index. This will likely involve:
  1. A buffer containing the flattened list of ball indices, sorted by cell.
  2. An "index" buffer that, for each grid cell, provides an offset and count into the ball index buffer.

### 2. Eradicate Inefficient GPU Memory Management

The current practice of reallocating the `BallBuffer` is amateurish and must be eliminated.

- **Fixed-Capacity Storage Buffer**:
  - Modify `prepare_buffers` (or a similar setup system) to allocate a `StorageBuffer` for ball data with a large, fixed capacity (e.g., `MAX_BALLS = 4096`).
  - This buffer **MUST NOT** be resized during runtime. Its size is immutable after creation.
- **CPU-Side Pool Management (Free List)**:
  - To handle the creation and destruction of balls, you will implement a "free list" on the CPU.
  - **Destruction**: When a ball is destroyed, its index in the GPU buffer is added to the free list. The data remains, but the slot is marked as available.
  - **Creation**: When a new ball is created, an index is requested from the free list. If available, that slot is reused. If not, the ball is appended to the end of the active block (up to `MAX_BALLS`).
- **Efficient Data Transfer**:
  - The `upload_metaball_buffers` system must be changed. Instead of reallocating, it will perform `queue.write_buffer` calls to update the data within the *existing*, pre-allocated buffer.
  - Since all balls are moving, you will likely update the entire contiguous block of *active* balls each frame. This is predictable and avoids pipeline stalls.

### 3. Refactor the Compute Shader (`compute_metaballs.wgsl`)

The shader logic must be rewritten to leverage the new spatial data structures.

- **Input**: The shader will no longer receive just a giant list of all balls. It will now take as input:
  1. The fixed-capacity `BallDataBuffer` (as a `storage` buffer).
  2. The `GridIndexBuffer` and the sorted ball index list (as `storage` buffers).
  3. Uniforms specifying grid dimensions and active ball count.
- **Logic**: For each pixel, the compute shader invocation will:
  1. Calculate which grid cell it belongs to.
  2. Use the `GridIndexBuffer` to find the offset and count for the relevant balls for that cell.
  3. Loop *only* over that small subset of balls, fetching their data from the `BallDataBuffer` using the retrieved indices.

## Success Criteria

- The `O(width * height * ball_count)` complexity is eliminated. Frame time should remain relatively stable as the number of balls increases, provided they are distributed across the screen.
- The `BallBuffer` GPU resource is allocated once at startup and never reallocated.
- The system correctly handles the creation and destruction of hundreds of balls per second without significant performance degradation or memory-related pipeline stalls.
- The visual output of the metaball rendering remains identical to the original implementation, just faster.

Do not fail.
