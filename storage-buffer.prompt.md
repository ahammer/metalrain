---
description: 'Plan and prompt for migrating metaballs cluster palette from fixed-size uniform array to storage buffer (SSBO) with WebGPU portability'
tags: ['rendering','wgsl','webgpu','bevy','migration','metaballs','storage-buffer']
---

# Metaballs Cluster Palette Storage Buffer Migration Prompt

## Purpose
You WILL migrate the metaballs rendering pipeline from a fixed-size uniform array (`MAX_CLUSTERS = 256`) palette to an unbounded (practically large) storage buffer (SSBO) palette while maintaining WebGPU portability, determinism, and performance.

## Success Criteria
You MUST achieve all of:
1. Visual stability: No flicker when cluster count > 256.
2. Deterministic color mapping across frames for persistent clusters (stable cluster `id`).
3. No hard-coded small upper limit baked into WGSL; capacity grows to at least thousands of clusters (target 8K) bounded only by buffer size.
4. WebGPU default limit compliance (no required limits escalation for typical use): avoid exceeding default `maxStorageBuffersPerShaderStage` (>=8) and `maxBindingsPerBindGroup` (>=1000).
5. Zero regressions in existing metadata encoding paths (metadata mode v2 stays functional).
6. Benchmark parity: Frame time regression < 2% vs baseline at 256 clusters.
7. Graceful fallback: If storage buffer allocation fails or feature unavailable (edge case), system reverts to legacy uniform path with warning.

## Constraints & Reference Limits (WebGPU Spec)
You MUST design within default guaranteed limits (WebGPU §3.6.2):
- maxStorageBuffersPerShaderStage ≥ 8 (we currently use 0; adding 1 keeps total well below limit).
- maxStorageBufferBindingSize ≥ 134,217,728 bytes (128 MiB).
- minStorageBufferOffsetAlignment ≤ 256 bytes (alignment required if using dynamic offsets; we will NOT use dynamic offsets initially).
You MUST keep the palette buffer size << 128 MiB (e.g., 8K clusters × 16 bytes RGBA32 = 128 KB) for headroom.

## High-Level Migration Phases
1. Introduce data model & mapping layer.
2. Add storage buffer WGSL declarations & bind group layout changes (add one binding only).
3. Populate SSBO from CPU with stable ordering.
4. Switch shader color lookup to SSBO.
5. Remove MAX_CLUSTERS references from dynamic logic (keep as legacy constant only if needed for fallback path).
6. Add fallback path & feature flag.
7. Add tests & perf benchmarks.
8. Remove / deprecate legacy uniform palette once validated.

## Detailed Steps

### 1. Stable Mapping Layer
You WILL create a persistent map `ClusterId -> PaletteIndex` maintained across frames.
- Data structure: `HashMap<u64, u32>` plus a `Vec<Vec4>` for colors in index order.
- On each frame: For each live cluster id, if absent assign next index and push color. If cluster disappears for N frames (configurable, default 120), you MAY compact later (deferred optimization).
- You MUST ensure indices are contiguous from 0..cluster_count-1 each frame when sent to GPU (compaction pass rewriting indices). Simpler approach: rebuild a sorted array each frame: Collect clusters, sort by `cluster.id`, assign indices in sorted order; stable deterministic ordering without long-term holes. Prefer sorted approach initially for simplicity & determinism; document potential O(n log n) cost (n ≤ few thousands → negligible).

Decision: You WILL adopt Sorted Ordering Strategy (by u64 id) for v1.

### 2. Rust Types & Resources
You WILL add a new GPU buffer resource type `ClusterPaletteStorage`:
```
pub struct ClusterPaletteStorage {
    pub buffer: Buffer, // wgpu handle via Bevy's RenderDevice
    pub length: u32,    // number of vec4 entries
    pub capacity: u32,  // allocated capacity
}
```
You WILL manage it inside existing render extraction/prepare systems in `metaballs` module.

Capacity Policy: Start with power-of-two capacity (e.g., 512). When `length > capacity`, double capacity up to a hard safety ceiling (e.g., 16384). Reallocate buffer, copy existing data (or rebuild entirely each frame since cost small). You MUST zero-fill unused tail to avoid undefined reads if shader over-reads due to bounds bug (defensive measure).

### 3. WGSL Changes
You WILL add a new binding for the palette storage buffer.
Group/Binding Selection:
- Reuse existing bind group for metaballs if slot available; else create new group dedicated to large buffers. You MUST confirm current group layout (inspect existing). For this prompt, assume we can add `@group(2) @binding(6)` as previously proposed (adjust if conflicts found during implementation). Document any changes.

WGSL Additions:
```
struct ClusterColor { value: vec4<f32>; };
@group(2) @binding(6) var<storage, read> cluster_palette: array<ClusterColor>;
```
You WILL remove uniform `cluster_colors` array from the palette portion once migration flag enabled. During dual-path phase, keep both and branch by a uniform flag `use_storage_palette`.

Lookup Change:
- Replace `metaballs.cluster_colors[cluster_idx].rgb` with `cluster_palette[cluster_idx].value.rgb` when storage path active.
Bounds Safety: You MUST clamp `cluster_idx` by `min(cluster_idx, cluster_color_count - 1u)` to avoid undefined OOB results (defense-in-depth despite CPU guarantee).

### 4. Bind Group Layout / Pipeline
You WILL update pipeline layout construction to include the new storage buffer. In Bevy, this may require modifying the material or custom render pipeline descriptor. Ensure visibility FRAGMENT (and VERTEX if vertex stage also needs color—currently only fragment likely). Use read-only storage (`var<storage, read>`), not uniform, to allow large size.

### 5. CPU Upload Path
You WILL stage data into a `Vec<[f32;4]>` then write to buffer via queue write (wgpu queue `write_buffer`) each frame (small ~10 KB typical). If clusters > ~4096 consider persistent mapped buffer optimization (defer until needed).

### 6. Feature Flag & Fallback
You WILL introduce a config option (RON & CLI) `metaballs.palette_mode = "uniform" | "storage" | "auto"`.
- auto: Use storage if device limits `maxStorageBufferBindingSize` >= required_size AND `maxStorageBuffersPerShaderStage` slack exists (≥1 remaining) else fallback.
- uniform: Force legacy path.
- storage: Force new path (warning & fallback if unsupported).
You WILL log capability decision at startup in debug mode.

### 7. Removal of MAX_CLUSTERS Constraint
You WILL retain `MAX_CLUSTERS` only in legacy code path; new path uses dynamic `cluster_color_count` from storage length. You MUST ensure all shader logic referencing `MAX_CLUSTERS` for early exits is refactored to use runtime count where safe. Keep early-out optimization loops based on actual count to avoid unnecessary iteration.

### 8. Testing Strategy
You WILL add tests:
- Unit test: Deterministic ordering: given cluster ids [10, 2, 5] after build order varied, produced palette order [2,5,10] mapping stable across frames.
- Render test (if headless): Compare pixel hash for frames with 300 clusters across two consecutive frames—hashes MUST match (no flicker) ignoring variable metadata unrelated to color.
- Overflow test: 3000 clusters → verify no panic, buffer capacity growth, and `length == cluster_count`.
- Fallback test: Force `uniform` mode; verify truncation warning when >256 clusters present.

### 9. Performance Validation
You WILL instrument frame timing before & after with 1K clusters scenario, capturing min/mean/max over 300 frames. Accept if ≤2% regression average. If >2%, profile: potential hotspots (sorting, buffer writes). Optimize by reusing sorted vector or switching to incremental map.

### 10. Migration Rollout Phases
Phase A (Dual): Both uniform & storage paths behind feature flag; default `auto`.
Phase B (Stabilize): After soak testing & no regressions, change default to `storage`.
Phase C (Deprecate): Emit warning when `uniform` selected manually; document removal timeline.
Phase D (Remove): Delete uniform palette array and constant (except maybe for historical docs).

## Risk Mitigation
- Driver Quirks: If storage buffer binding triggers validation errors on certain adapters, catch & fallback.
- Large Cluster Counts: Sorting O(n log n) – at 8K clusters cost negligible (<1 ms) on modern CPUs; monitor.
- Memory Growth: Doubling capacity ensures amortized O(1) reallocation; enforce hard cap 16K clusters with graceful degradation (warn & clamp) to avoid runaway memory.
- Shader Divergence: Branch on `use_storage_palette` uniform; later remove branch once legacy path removed.

## Implementation Order (Actionable Checklist)
1. Add config flag + runtime capability query.
2. Implement cluster collection + stable sorted ordering producing `Vec<[f32;4]>` palette.
3. Add CPU-side storage buffer resource management (allocate, grow, upload).
4. Extend bind group layout & pipeline to include storage buffer.
5. Modify WGSL to declare storage buffer & conditional lookup.
6. Add runtime uniform flag `use_storage_palette` & branching in shader.
7. Wire extraction/prepare systems to set counts & flag.
8. Update rendering code removing hard cap logic in storage path (no truncation).
9. Add tests (unit + integration) & bench harness.
10. Document feature in CHANGELOG & README (rendering section).
11. Enable `auto` by default.
12. Validate on native + Web (wasm32) builds.

## WGSL Snippet (Dual Path Example)
```
struct ClusterColor { value: vec4<f32>; };
@group(2) @binding(6) var<storage, read> cluster_palette: array<ClusterColor>;
@group(2) @binding(0) var<uniform> metaballs: MetaballsUniform; // existing

fn get_cluster_rgb(idx: u32) -> vec3<f32> {
  let count = cluster_color_count; // from uniform updated with storage length
  let safe = select(0u, idx, idx < count);
  if (metaballs.use_storage_palette > 0u) {
    return cluster_palette[safe].value.rgb;
  } else {
    return metaballs.cluster_colors[safe].rgb;
  }
}
```

You MUST ensure `cluster_color_count` equals storage length when storage path active.

## Telemetry & Logging
You WILL add debug log on first frame:
`PaletteMode=Storage clusters=XYZ capacity=ABC bytes=... limitRemainingStorageBuffers=...`
This aids diagnostics.

## Post-Migration Cleanup Criteria
You WILL remove legacy uniform path only after:
- All tests pass in storage mode.
- 2 release cycles without user reports of regressions.
- Performance targets met.

## Documentation Updates
You WILL update:
- CHANGELOG.md: Added: Storage-buffer backed metaballs palette removing 256 cluster cap (behind auto feature flag).
- Any rendering docs referencing MAX_CLUSTERS.
- Provide troubleshooting section: If colors flicker with storage disabled → enable storage or reduce clusters.

## Example CHANGELOG Entry
```
### Added
- Optional storage-buffer palette for metaballs clusters (feature flag `metaballs.palette_mode=auto|storage|uniform`). Removes 256 cluster cap when enabled.

### Deprecated
- Uniform-based cluster palette path (will be removed after stabilization).
```

## Validation Prompt (For Prompt Tester)
Prompt Tester, you WILL validate this migration plan by:
1. Enumerating the concrete code artifacts to add/change (files & symbols) based on this plan.
2. Simulating creation of WGSL diff for dual-path palette lookup.
3. Producing Rust pseudocode for buffer allocation & upload logic with capacity growth.
4. Demonstrating test cases (inputs → expected outputs) for ordering & overflow.
5. Listing potential ambiguities or missing details.

You WILL report any ambiguity; if none critical, mark plan ready for implementation phase.

## Open Questions (To Resolve During Implementation)
- Exact existing bind group indices & bindings (adjust @binding(6) if collision).
- Whether vertex stage ever needs cluster color (currently assumed fragment only).
- Maximum expected cluster count in real gameplay scenarios (tune initial capacity).

You MUST document resolutions for these during the implementation PR.

---
End of storage-buffer migration prompt.
