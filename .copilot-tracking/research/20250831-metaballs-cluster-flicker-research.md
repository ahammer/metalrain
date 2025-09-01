<!-- markdownlint-disable-file -->
# Task Research Notes: Metaballs Cluster Flicker >256

## Research Executed

### File Analysis
- assets/shaders/metaballs_unified.wgsl
  - Defines `const MAX_CLUSTERS : u32 = 256u;` and allocates `cluster_colors: array<vec4<f32>, MAX_CLUSTERS>` inside uniform `MetaballsData`. All cluster color lookups index this fixed-size uniform array. Cluster index comes from `b.w` (a float in storage buffer), cast via `u32(b.w + 0.5)`. Metadata mode packs cluster id differently (legacy 8-bit in A channel, or 16-bit split B/A when `metadata_v2_enabled`). Accumulation function skips contributions whose `cluster` index `>= cluster_color_count`. No bounds guard against cluster indices wrapping; overflow on CPU side maps excess clusters to slot 0.
- src/rendering/metaballs/metaballs.rs
  - Mirrors shader with `pub const MAX_CLUSTERS: usize = 256;` and uniform struct containing `[Vec4; MAX_CLUSTERS]`. During frame build: iterates `clusters.0.iter()` assigning sequential slots until `slot_count >= MAX_CLUSTERS { break; }`. Additional (overflow) clusters are silently dropped from color assignment; later any orphan/popped balls that would create new color slots are also prevented once cap reached, defaulting to slot 0. Ball GPU buffer encodes slot index as `cluster_slot` in `Vec4.w` (f32). No persistent mapping of stable cluster ids to slots; ordering of `clusters.0` directly determines which 256 clusters receive unique colors.
- src/physics/clustering/cluster.rs
  - Builds `clusters.0` each frame from a `HashMap` of connected components; final vector `new_clusters` preserves insertion order based on iteration over `comps.values()` (hash map value iteration => non-deterministic ordering when hash seed / component set changes). Stable `cluster.id` (u64) is tracked but *not* used to sort before GPU color slot assignment.

### Code Search Results
- MAX_CLUSTERS
  - Found in WGSL (2) and Rust (1 primary + usages) confirming hard cap of 256 for color array.
- cluster_color_count
  - Used in fragment shader to early-continue if cluster index exceeds populated count; protects reads but causes contributions from overflow clusters to be dropped.

### External Research
- (No external sources required; issue arises from internal fixed-size uniform cap and ordering.)

### Project Conventions
- Standards referenced: Shader/Rust constant mirroring (`MAX_CLUSTERS`), uniform packing comments in WGSL, resource construction patterns in Bevy plugin.
- Instructions followed: Focused internal code analysis per user request (#codebase, specific file), avoided speculative external assumptions.

## Key Discoveries

### Project Structure
Rendering path: CPU builds per-frame `MetaballsUniform` (including fixed cluster color array) + storage buffers (balls, tiles). Fragment shader samples cluster colors via array index derived from per-ball `cluster_slot`. Clustering system computes dynamic clusters independently; no persistent slot assignment layer.

### Implementation Patterns
- Fixed uniform-size palette: Hard limit 256 colors because uniform buffer embeds color array (approx 4KB). Easier binding model but imposes strict cap.
- Sequential slot assignment each frame: Order of `clusters.0` dictates which clusters fit under cap; excess ignored.
- Non-deterministic cluster ordering source: HashMap iteration when aggregating connected components leads to potential reordering when cluster membership changes—particularly volatile near cap.
- Overflow fallback behavior: Excess clusters' balls get slot index 0 (during orphan/popped fallback or because their cluster never mapped), causing visual color aliasing/mixing. As clusters churn around the threshold (e.g., cluster splits/merges, creation/destruction), which logical clusters map to slots 0..255 can oscillate frame-to-frame => perceived flicker.
- Metadata encoding limitations: In Metadata mode with legacy (v2 disabled), cluster id is truncated to 8 bits; above 255 all encoded as 255. With v2 enabled (flag in `metaballs.v4.z`), 16-bit cluster id supported in output channels B/A—solves metadata readback, not the color array cap.
- Storage precision: `cluster_slot` stored as f32 is accurate for integers up to 16,777,216—so precision isn’t the cause at 256.

### Complete Examples
```wgsl
// Excerpt showing cap and usage
const MAX_CLUSTERS        : u32 = 256u;
struct MetaballsData { /* ... */ cluster_colors: array<vec4<f32>, MAX_CLUSTERS>, };
// In accumulation
let cluster = u32(b.w + 0.5);
if (cluster >= cluster_color_count) { continue; }
```

```rust
// Slot assignment (truncates beyond 256)
for cl in clusters.0.iter() {
    if slot_count >= MAX_CLUSTERS { break; }
    let slot = slot_count; slot_count += 1;
    let c = color_for_index(cl.color_index).to_srgba();
    mat.data.cluster_colors[slot] = Vec4::new(c.red, c.green, c.blue, 1.0);
    for &e in &cl.entities { slot_map.insert(e, slot); }
}
```

### API and Schema Documentation
Internal shader/Rust contract: `v0.y` communicates `cluster_color_count`; fragment guards reads using this value. No dynamic length beyond compile-time constant `MAX_CLUSTERS`.

### Configuration Examples
```rust
pub const MAX_CLUSTERS: usize = 256; // Raising requires matching WGSL constant & potential UBO size review
```

### Technical Requirements
- Need stable visual output when logical cluster count exceeds 256 (current flicker).
- Maintain performance (avoid large uniform expansions; 4KB currently minimal, doubling modest cost but still static ~8KB if 512).
- Support >256 clusters without color alias flicker, or degrade gracefully with stability.

## Recommended Approach
Introduce a persistent, stable mapping layer from stable `cluster.id` (u64) to a limited set of GPU color slots, decoupling instantaneous cluster vector ordering from color slot assignment. Use an LRU or deterministic (e.g., sort by cluster.id) policy; when overflow occurs, evict the least-recently-used or highest-id cluster to a designated fallback color slot that remains constant, eliminating frame-to-frame oscillation. Optionally migrate `cluster_colors` to a storage buffer (SSBO) to lift the 256 cap entirely on platforms permitting larger bindings (preferred long-term), but the immediate fix for flicker is stable deterministic assignment with eviction. Steps: (1) Maintain `HashMap<u64, u16>` persistent_slot_map resource. (2) On frame build, reuse existing slot for each active cluster.id; assign new slot if capacity left; if full and new cluster appears, choose eviction policy (e.g., remove cluster not seen for N frames) and reuse its slot. (3) Populate uniform color array only for active slots; unused slots zeroed. (4) Pass active slot count as today. (5) Ensure ordering of writes stable (iterate slots ascending). This removes flicker because cluster->slot mapping only changes on explicit eviction, not due to unordered HashMap iteration churn at threshold.

### Why `MAX_CLUSTERS` Exists (Evidence-Based Rationale)
1. Uniform Buffer Simplicity: Embedding `[Vec4; 256]` inside a single uniform struct (`MetaballsUniform` / `MetaballsData`) allows all per-frame scalar params + color palette to reside in one bind group binding (index 0) without extra bind layout complexity. This matches the comment: `// NOTE: cluster_colors retained here; future optimization could move to storage buffer if needed.`
2. Alignment / Size Comfort Zone: 256 * 16 bytes = 4096 bytes for colors. Total uniform size remains modest (< 6 KB including scalar vec4s). Many GPUs & WebGPU implementations have conservative default limits for individual uniform buffer binding sizes; staying small reduces portability risk (especially WebGPU where min limits: 64 KB per uniform buffer binding, but historically some stacks had perf cliffs with larger UBOs).
3. Shader Static Array Indexing: Fixed-size array enables compile-time unrolling / bounds knowledge and simpler WGSL code. Dynamic-length runtime arrays in uniform buffers aren’t supported; storage buffers are required for unbounded growth.
4. Avoiding Extra Storage Binding Early: The design already uses three storage buffers (balls, tile headers, tile_ball_indices). Adding another for colors was likely deferred to minimize memory traffic & binding churn during initial refactor.
5. Anticipated Typical Cluster Counts: Initial gameplay likely assumed far fewer than 256 simultaneously visible logical color groups; thus 256 chosen as a “will never hit it” sentinel during early development.

### Can We Lift the Requirement Entirely?
Yes, by migrating the palette to a storage buffer (SSBO) and removing the compile-time `MAX_CLUSTERS` dependency in both Rust and WGSL. Practical ceiling then shifts to storage buffer length limits (WebGPU spec guarantees at least 128 MB per storage buffer; actual feasible size constrained by allocation strategy). With this change, cluster count becomes bounded only by gameplay logic, memory, and performance of accumulation loops.

### SSBO Migration Plan (Color Palette)
Contract Changes:
- Remove `cluster_colors` array from `MetaballsUniform`.
- Introduce new storage buffer (e.g., `@group(2) @binding(6)`): `struct ClusterColor { rgba: vec4<f32>; }; @group(2) @binding(6) var<storage, read> cluster_colors: array<ClusterColor>;`
- Pass `cluster_color_count` still via uniform (`v0.y`).
- Replace `metaballs.cluster_colors[cluster_idx].rgb` with `cluster_colors[cluster_idx].rgba.rgb`.

CPU Side Adjustments:
- New `ShaderStorageBuffer` asset holding a Vec<Vec4> sized to active cluster count each frame (or persisted across frames and resized only on growth to avoid realloc noise).
- Remove zero-fill loop over fixed array; just build vector sized `slot_count`.
- Update bind group layout / material derive: add `#[storage(6, read_only)] cluster_colors: Handle<ShaderStorageBuffer>`, adjust indices of any subsequent resources (ensure WGSL updated consistently). Because indices 3-5 currently used, next free is 6; maintain forward compatibility.

Performance Considerations:
- Memory Bandwidth: Reading one `vec4<f32>` from storage vs uniform; on many GPUs uniform may be slightly faster/cached, but difference negligible for one fetch per fragment (dominant cost is accumulation). If profiling shows regression, consider moving frequently accessed small subset (e.g., top-N cluster colors) back into push/uniform, but unlikely necessary.
- Binding Cost: Adds one more storage buffer binding; minimal overhead.
- Cache Locality: Palette likely fits in L1 anyway; storage vs uniform difference is small given sequential access pattern is sparse (one lookup per pixel).

WebGPU / Portability:
- Storage buffers widely supported; need to ensure `ShaderType` derive for new palette buffer or manual creation via `ShaderStorageBuffer::from(&colors[..])`.
- Ensure WGSL uses `array<ClusterColor>` (runtime-sized per creation, but WGSL still compiles referencing by index; length known at runtime only).

Edge Cases / Safety:
- Guard cluster index as today: `if (cluster >= cluster_color_count) { continue; }` remains valid.
- When cluster count is zero, supply at least one dummy color to avoid zero-length storage buffer binding pitfalls (mirrors existing ball buffer guard logic).

Migration Steps (Incremental):
1. Introduce SSBO (parallel path) behind feature flag `metaballs_cluster_colors_ssbo`.
2. Duplicate palette population into Vec<Vec4>, create / update storage buffer asset.
3. Shader: add conditional compilation path or (simpler) permanently switch to storage buffer and delete uniform array; keep old code commented temporarily during transition (then remove once validated).
4. Test visual parity with <256 clusters, then stress >256 ensuring no flicker & correct colors.
5. Remove `MAX_CLUSTERS` or repurpose as soft advisory (e.g., for debugging warnings when cluster count explodes unexpectedly).

### Trade-Off Comparison (Uniform Array vs SSBO)
| Aspect | Uniform Array (Current) | Storage Buffer (Proposed) |
|--------|-------------------------|---------------------------|
| Capacity | Hard compile-time cap (256) | Practically unbounded (memory limited) |
| Binding Count | Fewer (no extra) | +1 storage binding |
| Simplicity | Single struct | Slightly more plumbing |
| Flicker Risk >256 | High (ordering churn) | None (no truncation) |
| Portability | Very safe | Also safe (WebGPU core) |
| Future Extensions (per-cluster metadata) | Constrained by uniform size | Easy: extend struct (e.g., flags, emissive) |

### When to Still Keep a Cap
Even with SSBO, you may impose a *soft* cap for gameplay or performance (e.g., warn beyond 4096) because accumulation loops scale with number of clusters indirectly (more clusters ⇒ potentially more overlapping balls, though actual field work is per-ball). The cluster palette fetch itself is O(1); not the bottleneck.

### Recommended Path Update
Given your preference for SSBO: prioritize direct migration to storage buffer for palette, bypassing interim persistent-mapping fix unless you need a very fast patch. Persistent mapping becomes moot once truncation removed (ordering still changes but no flicker since all clusters mapped distinctly). If color stability (unchanging hues per stable cluster.id) is desired, add optional deterministic palette assignment keyed by `cluster.id` hashing (seeded) – independent concern from capacity.

### Optional Enhancement: Per-Cluster Metadata Expansion
Once in SSBO, extend `ClusterColor` to:
```wgsl
struct ClusterData { color: vec4<f32>, flags: u32, emissive: f32, pad: vec2<f32>; } // 32B aligned
var<storage, read> clusters: array<ClusterData>;
```
Allows future features: clickability flags, scoring, glow intensity, etc., without repacking into balls.

### Risks & Mitigations
- Bind Index Drift: Ensure all shader & Rust updates aligned; mismatched indices -> rendering black. Mitigation: integration test asserting material layout.
- Web Targets: Validate WASM pipeline; ensure not exceeding default limits (palette buffer small relative to guarantees).
- Regression in Early Exit Logic: None expected; cluster palette independent from field accumulation.

### Decision Snapshot
Current blocker (flicker) fundamentally due to fixed-size uniform truncation. SSBO migration cleanly eliminates the root constraint; complexity increase is minimal and future-proofs design for richer metadata.

## Implementation Guidance
- **Objectives**: Eliminate flicker when logical clusters >256; preserve color stability; prepare path to >256 support later.
- **Key Tasks**: 
  1. Add persistent `ClusterSlotMap { map: HashMap<u64, u16>, lru: VecDeque<u64> }` resource.
  2. During cluster build, for each cluster.id: if in map mark as recently used; else allocate new slot or evict.
  3. Replace sequential loop assigning slots from transient `clusters.0` ordering with iteration over active clusters sorted by their assigned slot id.
  4. For balls: look up cluster.id -> slot (instead of index position) when writing `GpuBall`.
  5. Leave WGSL unchanged initially (still capped 256) ensuring consistency.
  6. (Optional) Feature gate alternative path using storage buffer for colors to remove cap (future iteration).
- **Dependencies**: Access to stable `cluster.id` (already present), capacity constant (`MAX_CLUSTERS`), per-frame cluster list, color palette function.
- **Success Criteria**: (a) No visible flicker adding/removing clusters around 256 boundary; (b) Cluster colors remain stable over time barring explicit eviction; (c) Performance unaffected significantly (<1% frame cost change); (d) Unit/integration test confirming persistent mapping consistency across artificial reordering of `clusters.0`.
