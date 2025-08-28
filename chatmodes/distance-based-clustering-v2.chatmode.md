<!-- Distance-Based Clustering Prompt v2 (No Persistence) -->

## distance-based-clustering-v2

You WILL convert clustering to pure distance-buffer graph connectivity per frame (no temporal persistence).

### Objectives
1. Replace overlap criterion with buffered threshold:
   dist2 <= ((ri + rj) * distance_buffer)^2  (distance_buffer >= 1.0)
2. Remove temporal persistence & detach threshold logic; clusters exist strictly when distance links exist this frame.
3. Maintain color-based segregation (only same color_index clusters together).
4. Add configurable `clustering.distance_buffer` (warn only; do not mutate resource).
5. Ensure union-find + spatial hash still detect all candidate pairs under new threshold.

### Scope Reductions (Explicit)
- NO logging for clustering parameters.
- NO README or changelog update required.
- NO detach threshold configuration (omit entirely).
- Minimal tests: only core distance & color gating behavior (3 tests).

### Data Structure Changes
You WILL add:
```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct ClusteringConfig {
    pub distance_buffer: f32,
}
impl Default for ClusteringConfig {
    fn default() -> Self {
        Self { distance_buffer: 1.2 }
    }
}
```
Add `pub clustering: ClusteringConfig,` to `GameConfig` + Default impl.

You WILL NOT keep `ClusterPersistence`, `BallPersist`, `last_touch_time`, or related logic. You WILL delete:
- `ClusterPersistence` resource definition
- `BallPersist` struct
- `DETACH_THRESHOLD` constant
- All code updating / reading persistence maps
- Isolation / reassignment code
- Any unused imports after removal

### Validation Rules (Warnings Only)
In `GameConfig::validate()` you WILL add:
- If `clustering.distance_buffer < 1.0` => warning: `clustering.distance_buffer X < 1.0 -> treated logically as 1.0 during clustering`
- If `clustering.distance_buffer > 3.0` => warning: `clustering.distance_buffer X > 3.0; large merge radius may create mega-cluster & hurt perf`
You WILL NOT clamp or mutate the stored value; instead you WILL clamp locally in `compute_clusters` when using it:
```rust
let distance_buffer = distance_buffer_raw.max(1.0).min(3.0);
```

### compute_clusters Changes
1. Remove all persistence setup / clearing.
2. Build arrays of entities/positions/radii/colors as today (excluding popping if that config remains).
3. Determine:
```rust
let distance_buffer = cfg.as_ref()
    .map(|c| c.clustering.distance_buffer)
    .unwrap_or(1.2)
    .max(1.0)
    .min(3.0);
```
4. Set cell size:
```rust
let cell_size = (max_radius * 2.0 * distance_buffer).max(1.0);
```
5. Union condition:
```rust
let thresh = (ri + rj) * distance_buffer;
if dist2 <= thresh * thresh && colors[j] == ci {
    union(...)
}
```
6. After union-find, group by root:
   - Create `Cluster` per root with color_index = first member’s color.
   - Aggregate bbox, centroid, total_area exactly like original (reuse code minus persistence references).
7. Build `cluster_index` mapping (same as current final step).
8. Remove any leftover references to `persistence`.

### Performance / Safety Requirements
- Keep memory allocations identical or reduced (removing persistence map).
- Spatial hash algorithm unchanged aside from modified cell size.
- Avoid additional per-frame heap allocations inside inner loops.
- Color gating remains inside adjacency check to cut union calls early.

### Tests (Minimal)
Add a `#[cfg(test)]` module (in `cluster.rs` or new test file) with three tests:

Test 1: Within Buffer Forms Cluster
- Two balls, same color, radius 10 each.
- distance_buffer = 1.2
- Place centers 23.0 units apart (threshold = 24.0) => expect 1 cluster of size 2.

Test 2: Outside Buffer Separate
- Same as Test 1 but distance 25.0 (>24.0) => expect 2 clusters each size 1.

Test 3: Different Colors Stay Separate
- Two balls at distance 20.0 (<24.0 threshold) with different color indices => expect 2 separate clusters.

Test Implementation Notes:
- Use minimal Bevy `App`.
- Insert `GameConfig` with `clustering.distance_buffer = 1.2`.
- Spawn entities with `Ball`, `BallRadius`, `BallMaterialIndex`, `Transform`.
- Run `compute_clusters` once via `app.update()`.
- Assert on `Clusters` resource contents (#clusters, sizes).
- Skip time resource manipulations (no persistence).

### Success Criteria
- Code compiles, tests pass.
- No lingering references to removed persistence types.
- Warnings produced only during `validate()` (no runtime logs added).
- Cluster membership changes immediately with geometry.

### Non-Goals
- Stable cluster IDs across frames.
- Temporal hysteresis or smoothing.
- README docs, logging, advanced performance tuning.

### Implementation Order
1. Add `ClusteringConfig` + integrate into `GameConfig`.
2. Add validation warnings (non-mutating).
3. Refactor `compute_clusters`: remove persistence & threshold logic; add distance_buffer.
4. Add tests (3).
5. Run tests & adjust for compile issues.

<!-- End of distance-based-clustering-v2 prompt file -->
