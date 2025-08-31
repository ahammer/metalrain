---
description: "Refactor metaballs pipeline: storage buffer balls, tiled accumulation, defensive bounds, full cluster id metadata, early-exit"
---

# Metaballs Refactor / Optimization Prompt

You WILL implement the following enhancements to the metaballs rendering path (starting on the Rust side, then WGSL):

## Objective Summary
1. Replace brute-force per-fragment loop over all balls with a tile-local loop (screen-space spatial binning).
2. Move `balls` array out of the large uniform (`MetaballsUniform`) into a STORAGE buffer (read-only in fragment shader).
3. Add defensive `min(ball_count, actual_buffer_len)` clamping everywhere count is consumed (Rust + WGSL).
4. Stop truncating cluster id to 8 bits in metadata output; provide a higher-fidelity encoding (at least 16 bits, ideally full 24). Avoid breaking non-metadata modes.
5. Introduce early exit in accumulation once the dominant field >= iso when that foreground mode does not require precise gradient or multi-cluster ordering (configurable / safe modes only).

## Current State (Baseline)
Reference files:
- Rust material & population: `src/rendering/metaballs/metaballs.rs`
- Shader: `assets/shaders/metaballs_unified.wgsl`
Key points:
- `MetaballsUniform` packs a fixed-size `balls: [Vec4; 1024]` and `cluster_colors: [Vec4; 256]` in a uniform buffer.
- Fragment shader calls `accumulate_clusters` which brute-force scans `ball_count` balls each pixel.
- Metadata mode encodes cluster/orphan color slot as `A = min(cluster_idx,255)/255.0` (8-bit truncation).
- No early-exit; gradient always computed when a ball contributes.

## High-Level Design Changes
You WILL:
1. Introduce a CPU-built screen-space tiling system (uniform tile grid) mapping tiles -> contiguous list of ball indices, uploaded each frame (or when dirty) into storage buffers.
2. Split GPU data into:
   - Small uniform (params, counts, viewport sizes, iso, modes, scaling, etc.) — keep existing param vec4 packs minus the large arrays.
   - Storage buffer A: `Balls` (SoA or AoS). Each ball entry: `vec4<f32>` (x, y, radius, packed_cluster_flags) maintained for shader compatibility. (Optionally extend to u32 flags via second buffer if needed.)
   - Storage buffer B: `TileIndex` header array: one entry per tile containing `(offset, count)` into a flat index list.
   - Storage buffer C: `TileBallIndices` flat `u32` list of ball indices (contiguous; built every frame or when topology changes).
   - (Optional) Storage buffer D: cluster colors (or keep cluster colors in a smaller uniform if size acceptable). You MAY keep `cluster_colors` in uniform for simplicity since 256*16B = 4KB (acceptable). Document decision.
3. Provide defensive clamping for any shader loop: `let safe_ball_count = min(ball_count, balls_buffer_len);` using an additional field (e.g., `v3.x = balls_len_f32`) or a dedicated uniform lane.
4. Encode cluster id in metadata with >= 16 bits precision:
   - Plan A (preferred): Use RG channels to store a 16-bit unsigned cluster id: `R = low8/255`, `G = high8/255`; Keep existing signed-distance proxy in B and clickable mask in A (or reassign channels—document mapping). OR
   - Plan B: Store cluster id as float in A (exact for <= 16,777,216) and move mask to G, SDF to R, keep B for future flags. Choose mapping that minimizes downstream breakage and clearly update consumer expectations.
   - Provide migration note & feature flag (`METABALLS_METADATA_V2`).
5. Implement an accumulation early-exit path when safe:
   - Safe when foreground mode is ClassicBlend OR OutlineGlow and surface edge modulation is disabled AND debug_view != gradient-dependent mode AND metadata mode is not active.
   - Early exit condition: once current dominant cluster field >= effective_iso AND (for OutlineGlow) no glow outward expansion requires sub-iso sampling (OutlineGlow still needs edge factor; ensure iso-crossing correctness—if uncertain, gate behind opt-in flag `EARLY_EXIT_OUTLINE_GLOW`).
   - Bevel & Metadata modes REQUIRE gradients; disable early exit there.

## Step-by-Step Implementation (Rust Side First)
You WILL perform these Rust changes before shader edits.

### 1. Data Structure Additions
Create new Rust structs & resources:
```
pub struct GpuBall { pub pos: Vec2, pub radius: f32, pub cluster_slot: u32 } // Will be cast/packed
#[derive(Resource, Default)] pub struct BallTilingConfig { pub tile_size: u32 } // default 64
#[derive(Resource, Default)] pub struct BallTilesMeta { pub tiles_x: u32, pub tiles_y: u32 }
```
Decide tile size (e.g., 64 px) configurable via `GameConfig` or constant; store in `BallTilingConfig`.

### 2. Tile Builder System
Add a system (after balls/clusters update) that:
1. Reads all ball positions & radii (after radius scaling & multiplier applied) to compute AABB in screen space.
2. Derives tile coverage: compute tile indices min/max inclusive clamp to grid.
3. Appends ball index into per-tile `Vec<u32>` (use preallocated `Vec<Vec<u32>>>`).
4. After aggregation, performs a single pass to flatten into contiguous `tile_ball_indices: Vec<u32>` and `tile_headers: Vec<(u32 offset, u32 count)>` in tile order.
5. Uploads/updates wgpu buffers (through Bevy `RenderAsset` or manual `RenderQueue`) only if counts changed or capacity insufficient (re-use staging to avoid allocations). Keep a `capacity` & `len` to support defensive clamping.

### 3. Storage Buffer Integration
Modify `MetaballsUnifiedMaterial`:
1. Remove `balls` array from `MetaballsUniform` (shrink struct).
2. Add `#[storage(X, read_only)] balls: Vec<GpuBall>` or if macro not supported in 0.16 for custom layout, create a parallel bind group resource (custom `RenderAsset`) bound in the pipeline layout. Validate Bevy 0.16 `AsBindGroup` capability; if unsupported, fallback to separate bind group created in a custom `Material2d` extension (`prepare_material` stage) that binds storage buffers at group(2)/later binding indices.
3. Ensure `ball_count` lane still provided in uniform and add `balls_len` lane for clamping (e.g., repurpose `v2.w` if free or add a new `v3: Vec4` uniform—update WGSL accordingly).

### 4. Cluster Colors
Retain in uniform for now; simply update assignment unchanged (fewer moving parts). Document that moving them later is optional optimization.

### 5. Defensive Clamping
In the material update system, after pushing balls to the storage buffer, set `ball_count = min(actual_balls, MAX_BALLS_CAP_EXPOSED)` where `actual_balls` is dynamic. Set a uniform `balls_len` = actual buffer length for shader cross-check.

### 6. Feature Flags / Debug
Add cargo features or runtime toggles in config for:
- `metaballs_early_exit` (on by default).
- `metaballs_outline_exit` (off by default—experimental early-exit for outline mode).
- `metaballs_metadata_v2` (new encoding). Provide log lines when enabled.

### 7. Metadata Encoding Update
If `metaballs_metadata_v2`:
- Chosen mapping (example):
  - R: signed-distance proxy (unchanged)
  - G: clickable mask
  - B: high 8 bits of cluster id (cluster_id >> 8) / 255.0
  - A: low 8 bits (cluster_id & 0xFF) / 255.0
  -> Provides 16-bit cluster id (0..65535). Document consumer merge formula.
Else keep legacy encoding for backward compatibility.

### 8. Gradient Requirement Flagging
Before dispatching draw, determine if active mode requires gradients. Provide a boolean uniform lane (e.g., `v1.w` repurposed if debug_view can move; or add new lane) to allow shader to skip gradient accumulation in early-exit path.

## Shader Changes (WGSL)
You WILL modify `metaballs_unified.wgsl`:
1. Remove `balls` big array from uniform struct; replace with:
```
struct GpuBall { pos_radius: vec4<f32>; } // (x,y,radius, cluster_flags)
@group(2) @binding(Bx) var<storage, read> balls: array<GpuBall>;
```
2. Add storage buffers:
```
struct TileHeader { offset: u32, count: u32, _pad0: u32, _pad1: u32; }
@group(2) @binding(By) var<storage, read> tile_headers: array<TileHeader>;
@group(2) @binding(Bz) var<storage, read> tile_ball_indices: array<u32>;
```
3. Provide uniform lanes: `(tiles_x, tiles_y, tile_size, balls_len)` in a new vec4.
4. Compute tile id in fragment:
```
let tile_coord = clamp(vec2<i32>(floor((p - origin)/tile_size_f)), vec2<i32>(0), vec2<i32>(i32(tiles_x-1), i32(tiles_y-1)));
let tile_index = u32(tile_coord.y) * tiles_x + u32(tile_coord.x);
let th = tile_headers[tile_index];
```
5. Replace brute-force scan with:
```
for (var j: u32 = 0u; j < th.count; j = j + 1u) {
  let ball_i = tile_ball_indices[th.offset + j];
  if (ball_i >= safe_ball_count) { break; }
  // fetch & accumulate (same logic as before)
  // Early exit condition inserted here
}
```
6. Define `let safe_ball_count = min(ball_count, balls_len);`.
7. Metadata cluster id encoding:
   - If `#define METABALLS_METADATA_V2` (use `override` or preprocessor if using naga `@id`—if not available, use separate shader or runtime uniform flag), output according to new mapping.
8. Early-exit logic:
   - Track current dominant cluster index & field.
   - After updating an existing cluster field, test if `enable_early_exit && !needs_gradient && field >= effective_iso`; if true, break loops (both tile loop and potential per-ball loops). For multi-cluster logic, ensure still produce correct `mask` (field saturated at >=iso).
   - For OutlineGlow (if enabled early exit), ensure you still approximate glow thickness; fallback: disable in OutlineGlow unless opt-in flag set.
9. Remove 8-bit truncation: Only apply `min(cluster_idx, 65535u)` (or documented maximum) for new encoding; legacy path remains for compatibility.

## Defensive / Edge Cases
You WILL handle:
- Zero tiles (fallback to brute-force if viewport smaller than tile size or tile size misconfigured).
- Empty tile (`th.count == 0`) — quickly early-out to background.
- Overflow: If total indices exceed `u32::MAX` (improbable), log and clamp.
- Radii extremely small -> avoid zero division; keep `1e-5` guards.
- Ball outside screen: still inserted into tiles overlapping extended bounding box if any; optional: cull completely if fully outside.

## Validation Plan
You WILL add or update tests (Rust) for:
1. Tiling builder: each ball appears in all tiles it overlaps; counts correct.
2. Defensive clamp: artificially set `ball_count > actual_len` and confirm shader uniform `ball_count` is ignored beyond `actual_len` (can be a CPU-level assertion or WGSL unit test if pipeline test harness exists).
3. Metadata V2 encoding round-trip (cluster id reconstruct). Provide helper to reconstruct `u16` from RG or from A if single-channel approach chosen.
4. Early-exit functional equivalence: For a frame with gradient-free mode, run accumulation with early-exit disabled vs enabled (CPU reference) and assert identical mask & dominant cluster for pixels where `best_field < iso` and equal classification for `>= iso`.

## Success Criteria
You MUST meet all of these to consider task complete:
- Shader compiles & runs (no validation errors) with new storage buffers.
- Memory footprint reduced (uniform size notably smaller) & no WebGPU binding limits exceeded.
- Visual output identical (within epsilon) to baseline for gradient-required modes; metadata mode yields higher-fidelity cluster ids.
- Performance improvement: reduced fragment time (profile or log) when many balls present (> ~200) due to tiling.
- Safe early-exit path produces no visual artifacts in enabled modes.

## Migration / Rollout Steps
1. Implement code behind feature flag(s) default ON for storage + tiling, OFF for experimental outline early-exit.
2. Provide fallback path (if storage buffers unsupported—WASM + WebGPU should be fine) to old uniform loop (retain code for a short deprecation period if desired).
3. After validation, remove old brute-force path.

## Deliverables Checklist
- [ ] Updated Rust structs & resources (tiling, storage buffer prep)
- [ ] Systems for building & uploading tile data
- [ ] Material / bind group changes
- [ ] WGSL modifications (storage buffers, tile scan, early-exit, metadata encoding)
- [ ] Feature flags & logging
- [ ] Tests updated/added
- [ ] Documentation in code comments summarizing new data flow

You WILL execute each step in order, verifying compilation incrementally. You MUST not regress existing tests. You WILL add targeted profiling instrumentation (timing the tiling build & average tile occupancy) behind a `debug` feature log target `metaballs_prof`.

---
END OF PROMPT
