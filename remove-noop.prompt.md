<!--
Prompt Name: Remove No‑Op Compute & Introduce Gradient Prepass (Phase 1)
Purpose: You WILL replace the existing placeholder no‑op compute pass with a real gradient + field accumulation compute prepass that prepares half‑resolution data for future shading (bevel, adaptive AA, outline, metadata). You WILL NOT integrate the produced texture(s) into the fragment/material path yet. This is an incremental, non‑visual change (visual output MUST remain identical to current main branch). The pass must be fully wired (native + WASM) and performance‑oriented, leveraging existing tiling buffers.
Scope: Remove old compute plumbing, add gradient compute pipeline + WGSL shader, produce & maintain reusable textures + stats, implement scheduling & toggles, add tests & documentation.
References: `src/rendering/metaballs/compute_noop.rs` (to be removed), `metaballs_unified.wgsl`, `systems.rs` (tiling + uniform setup), `gpu.rs` (layout), `.github/copilot-instructions.md` (architecture & performance guidelines).
CRITICAL: Do NOT alter existing material bind group indices or `MetaballsUnifiedMaterial` layout. All new resources must be isolated to the compute pipeline.
-->

## 1. High‑Level Goal
You WILL implement a half‑resolution gradient+field compute prepass that:
1. Computes per‑pixel (half-res) scalar field (metaball iso field) and its screen‑space gradient (dF/dx, dF/dy).
2. (Optional Phase 1 add) Tracks dominant cluster id for that pixel (mirroring fragment dominant selection) for later lighting decisions.
3. Writes results into a single `RGBA16Float` storage texture (field, grad.x, grad.y, cluster_id_float) OR two textures (`RG16F` + `RG16F`) if platform requires (choose single RGBA16F for simplicity now).
4. Uses existing CPU‑built tile acceleration (`tile_headers`, `tile_ball_indices`) to limit per‑pixel ball iteration.
5. Leaves the fragment shader untouched (it still brute‑forces balls); visual parity guaranteed.

## 2. Success Criteria (MANDATORY)
All must hold:
1. The old no‑op compute module + shader are removed (file deletion + plugin wiring removed) with no compile residue.
2. New gradient compute WGSL shader created at `assets/shaders/metaballs_gradient_compute.wgsl`.
3. A new Rust module `src/rendering/metaballs/gradient_compute.rs` provides pipeline setup, texture allocation, node registration, & stats.
4. Render graph ordering: gradient compute node executes in the same slot the no‑op did: after `Node2d::StartMainPass` and before `Node2d::MainOpaquePass` (do NOT change other edges).
5. Texture resolution = `(viewport_w/2).ceil() x (viewport_h/2).ceil()`; reallocated on resize (handle window dimension changes efficiently; avoid per-frame reallocate if unchanged).
6. Field equation matches fragment path’s cubic kernel exactly (bitwise equivalent for same samples).
7. Gradient uses analytic derivative of kernel (NOT finite differences) for accuracy + perf.
8. Cluster dominance logic (if implemented) respects same clustering identification (dominant by accumulated field contribution). Track up to `CLUSTER_TRACK_MAX` clusters just like fragment.
9. All loops are bounded; no panics, no dynamic heap growth per workgroup dispatch beyond reasonable shared memory.
10. Visual output identical (validated manually / test harness) since fragment still ignores gradient texture.
11. WASM build works; shader embedded (mirrors pattern used for unified material & legacy no‑op).
12. No new warnings from wgpu validation (correct binding types, usage flags).
13. Tests cover: texture allocation on resize, dispatch occurs, field parity for a small controlled scene (within epsilon), and non-regression after multiple frames.
14. Config toggle (reuse or add) to enable/disable compute prepass (`MetaballsParams` or new `MetaballsGradientToggle` resource). When disabled: node early-returns; no dispatch.

## 3. Non‑Goals (Phase 1)
You MUST NOT:
1. Sample gradient texture in fragment shader yet.
2. Remove or change existing per-pixel accumulation in `metaballs_unified.wgsl`.
3. Modify material bind group layouts / indices.
4. Introduce multi-pass downsampling chains or mipmaps.
5. Add new config file fields beyond an optional runtime toggle resource (defer config integration unless already present).

## 4. Data Format & Memory Contract
Primary output: `RGBA16Float` storage (and sampled) texture.
Channel packing:
* R: field value F (sum of cubic contributions, unclamped)
* G: dF/dx
* B: dF/dy
* A: dominant cluster id as float (0.0 if none / empty). (Reserve for future; can be 0 now if cluster logic deferred.)

Justification:
* 16-bit floats sufficient (field ∈ [0, ~num_balls]) — gradient magnitude stability acceptable.
* Single texture simplifies binding & future sampling.

Texture usage flags: `STORAGE_BINDING | TEXTURE_BINDING | COPY_SRC`.

## 5. Analytic Kernel & Derivatives
Given fragment kernel:
```
x = 1 - (d^2 / r^2)   (inside only)
F_contrib = x^3
```
Let `d2 = (p - c)^2`, `r2 = r^2`.
Inside region derivative: `dF/dx = 3x^2` and `x = 1 - d2/r2`.
Vector derivative w.r.t p:
```
dF/dp = dF/dx * dx/dp
dx/dp = - (2 / r2) (p - c)
=> dF/dp = -6 x^2 (p - c) / r2
```
Accumulate gradient only for inside balls (same condition as field). Sum contributions (linear).

## 6. Tiling Utilization
For each output pixel:
1. Map pixel → world position consistent with fragment’s `world_pos` mapping (center aligned). Use the exact inverse of the fragment vertex path:
    * Fragment world position today: `world_pos = (px + 0.5 - w*0.5, py + 0.5 - h*0.5)` where `(px,py)` are full‑resolution integer pixel coordinates and `w,h` are viewport dimensions.
    * Half‑resolution invocation coordinates: `(hx, hy)`.
    * The 2×2 full‑res block covered spans full‑res pixel centers: `2hx + 0.5` and `2hx + 1.5` horizontally, likewise for y.
    * Choose the true block center: `px_center = 2hx + 1.0`, `py_center = 2hy + 1.0` (average of the two centers), yielding:
       `world_x = px_center - w*0.5`, `world_y = py_center - h*0.5`.
    * This replaces the earlier approximate `*2.0 + 0.5` (keep ONLY `+1.0`). For odd viewport sizes, clamp `px_center` to `w - 0.5` (same for y) to avoid sampling slightly outside logical extent for the last half‑res column/row.
2. Derive tile index: floor((world_pos - view_origin) / tile_size).
3. Fetch tile header; iterate only its balls via `tile_ball_indices`.
4. For each ball, compute field contrib + gradient analytic derivative.
5. Track cluster contributions (optionally; if skipped, set A=0). If implemented: replicate small fixed arrays (size `CLUSTER_TRACK_MAX`), find/insert cluster id and accumulate field.

Early exit: If tile `count == 0` output zeros.

## 7. WGSL Shader (Outline)
File: `assets/shaders/metaballs_gradient_compute.wgsl`.
Bindings (Group 0 suggested — independent from material group(2)):
```
@group(0) @binding(0) var<uniform> metaballs: MetaballsData;        // Mirror struct (reuse existing layout definition copied from fragment).
@group(0) @binding(1) var<storage, read> balls: array<GpuBall>;
@group(0) @binding(2) var<storage, read> tile_headers: array<TileHeader>;
@group(0) @binding(3) var<storage, read> tile_ball_indices: array<u32>;
@group(0) @binding(4) var<storage, read> cluster_palette: array<ClusterColor>; // (optional for future)
@group(0) @binding(5) var gradient_out: texture_storage_2d<rgba16float, write>;
```
Workgroup size: start with `8x8`.
Indexing: `gid = @builtin(global_invocation_id).xy` -> pixel coordinate inside half-res extents; guard if outside.
No dynamic loops aside from bounded cluster arrays.
Avoid derivative-dependent functions (only arithmetic); no textureSamples here.

## 8. Rust Module Implementation Steps
Create `gradient_compute.rs` with:
1. WASM shader embedding (OnceLock pattern) analogous to previous no‑op.
2. Resources:
   * `MetaballsGradientPipeline { pipeline_id, shader, layout, logged }`
   * `MetaballsGradientImages { tex: Handle<Image>, size: UVec2 }`
   * `MetaballsGradientToggle(pub bool)` (default true).
   * (Optional) `MetaballsGradientStats { dispatches: u64 }`.
3. System: `prepare_gradient_pipeline` (Render schedule) – load shader, create bind group layout(s), queue compute pipeline.
4. System: `prepare_gradient_target` – allocate / resize half-res image when window size changes, set texture usage flags.
5. Render graph node: `MetaballsGradientComputeNode` — binds pipeline + sets bind groups, dispatches over `ceil(w/8), ceil(h/8)`.
6. Registration in `MetaballsPlugin::build`: replace old no‑op insertion with gradient pipeline init + node insertion.
7. Bind group creation each frame only if something changed (cache handles; avoid per-frame reallocation).
8. Early return in node if toggle false OR pipeline/image not ready.

## 9. Bind Group & Layout Details
Single bind group (group 0) with entries matching WGSL binding list. Use `ShaderType` reflection or manual descriptor:
```
BindGroupLayoutEntries: UNIFORM (metaballs), READ_ONLY_STORAGE (balls), READ_ONLY_STORAGE (tile_headers), READ_ONLY_STORAGE (tile_ball_indices), READ_ONLY_STORAGE (cluster_palette), STORAGE_TEXTURE (rgba16float, write)
```
Do NOT reuse material group(2). This isolation preserves existing material assumptions.

## 10. Scheduling & Ordering
* Add systems to Render schedule identical ordering as prior no‑op (ensuring prepass still precedes opaque pass).
* Node edges: `StartMainPass -> GradientComputeNode -> MainOpaquePass` (unchanged relative to old node). Remove old node edges.

## 11. Removal Tasks
1. Delete `src/rendering/metaballs/compute_noop.rs`.
2. Delete `assets/shaders/metaballs_noop_compute.wgsl`.
3. Remove references in `metaballs.rs` plugin build.
4. Remove any re-exports or tests referencing `MetaballsNoop*`.

## 12. Testing Plan
Add tests (feature `bevy_ci` or standard) focusing on pure logic & resource state:
1. `gradient_allocates_and_resizes`: Simulate window resize; assert texture size halves & updates only when needed.
2. `gradient_dispatch_runs`: After a frame with balls present & toggle true, stats.dispatches > 0.
3. `field_parity_small_scene`: Spawn 2 balls, run gradient pass, then (OPTIONAL) run a CPU replicate of field at a few sample points downscaled and compare within epsilon (0.5% relative) to texture values read back (if readback infra acceptable; else unit-test derivative function separately by factoring math into a pure helper reused by shader). If GPU readback heavy, document skipped.
4. `toggle_disables_dispatch`: When toggle false, dispatch count stable across frames.
5. (Optional) `cluster_channel_default_zero`: If cluster logic unimplemented, assert A=0.

## 13. Performance Considerations
* Half-res reduces pixel count 4×.
* Tiling avoids O(N_balls) per pixel; worst case fallback still bounded.
* Use `u32` indices; keep per-pixel arrays in private function scope (WGSL stack) not large (> a few hundred bytes).
* Consider workgroup local accumulation only later (Phase 2) — keep Phase 1 simple (each invocation isolates pixel work).
* Avoid branching where possible; cluster tracking small unrolled loops.

## 14. Logging
Targets: `metaballs`.
* INFO once: "Gradient compute prepass active (half-res field/gradient)".
* DEBUG (optional guarded by feature): resize events.
* Avoid per-frame spam; throttle.

## 15. Future Integration (Deferred)
* Fragment shader sampling path (add binding for gradient texture & branch on `needs_gradient`).
* Adaptive AA using gradient magnitude.
* Bevel normal shading using normalized (dF/dx, dF/dy, normal_z_scale).
* Optional second texture storing distance-to-iso or encoded normal for compression.
* Workgroup / tile-level cooperative loading of ball subsets.

## 16. Rollback Strategy
Revert commit removing no-op OR comment out gradient node insertion & toggle off; no material changes ensure rapid rollback.

## 17. Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Binding mismatch (validation errors) | Keep bind group isolated (group 0), test early on both native & WASM. |
| Resize thrash | Only recreate image if size changed. |
| Perf regression (extra compute time) | Toggle off by default if uncertain; measure before enabling for end-users. |
| Divergent field formula | Share a pure Rust helper mirroring WGSL kernel and test parity. |

## 18. Completion Checklist
- [ ] Old no-op files & references removed.
- [ ] New WGSL gradient shader added & loads (native + WASM). 
- [ ] Pipeline + bind group created without warnings.
- [ ] Half-res texture created & persists; resizes correctly.
- [ ] Dispatch occurs when enabled; skipped when disabled.
- [ ] Field & gradient values sane (non-NaN, finite) in debug sampling.
- [ ] Tests added & passing (or documented if certain GPU readbacks deferred).
- [ ] Single one-time INFO log present.
- [ ] Visual parity confirmed (manual capture). 

## 19. Implementation Order (Step-by-Step)
1. Delete no-op shader & module, remove plugin wiring.
2. Add gradient WGSL file (skeleton with bindings, kernel accumulation, analytic gradient, TODO cluster accumulation).
3. Add `gradient_compute.rs` resources + systems + node.
4. Add WASM shader embedding analog.
5. Insert node & resources in plugin build.
6. Add toggle resource (default true) & (optional) debug key to flip for benchmarking.
7. Implement resize system & integrate into Render schedule before node (or inside prepare system).
8. Implement cluster accumulation (optional this phase; else set A=0).
9. Add tests.
10. Run clippy & tests; verify no regressions.

## 20. Minimal WGSL Skeleton (Illustrative – DO NOT Omit Final Implementation Details)
```wgsl
// metaballs_gradient_compute.wgsl (Phase 1)
@group(0) @binding(0) var<uniform> metaballs: MetaballsData;
@group(0) @binding(1) var<storage, read> balls: array<GpuBall>;
@group(0) @binding(2) var<storage, read> tile_headers: array<TileHeader>;
@group(0) @binding(3) var<storage, read> tile_ball_indices: array<u32>;
@group(0) @binding(4) var<storage, read> cluster_palette: array<ClusterColor>;
@group(0) @binding(5) var gradient_out: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(gradient_out);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
   // Map half-res pixel (hx,hy) to world_pos (consistent with fragment path)
   let vp = metaballs.v2.xy;          // (w, h)
   let pixel = vec2<f32>(f32(gid.x), f32(gid.y)); // (hx, hy)
   // Full-res logical center coordinates for the 2x2 block: (2hx + 1, 2hy + 1)
   let full_center = pixel * 2.0 + vec2<f32>(1.0, 1.0);
   // world_pos = (center - (w/2, h/2))
   let world_pos = full_center - vp * 0.5;
   // (Optional clamp for odd dims can be added if needed)

  // Derive tile + accumulate field/gradient (pseudo; fill in like fragment logic).
  var field: f32 = 0.0;
  var grad: vec2<f32> = vec2<f32>(0.0, 0.0);
  var cluster_id: f32 = 0.0; // (optional placeholder)
  // TODO: tile header lookup & iteration
  textureStore(gradient_out, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(field, grad.x, grad.y, cluster_id));
}
```

## 21. Optional Cluster Accumulation (If Included Now)
Replicate small fixed arrays as in fragment; choose dominant at end and store ID in A. If deferred, explicitly document in code `// Phase 2: cluster dominance` and always write `A=0.0`.

## 22. Documentation Update
Add a short README section or TODO marker: “Gradient compute prepass (half-res) present but not yet consumed by shader; future integration will harness bevel + adaptive AA.” If README auto-generator script exists (`generate-readme` task), extend its template; else leave TODO.

---
## **Prompt Builder**: Requesting Validation
Prompt Tester, please execute this prompt for a minimal scenario:
* Window: 800x600, thus half-res target 400x300.
* 3 balls (identical radius 40) arranged so all overlap center; 2 distinct color groups (cluster ids 0,1,0).
* Toggle enabled.
Describe: (a) expected dispatch grid, (b) approximate central pixel field & gradient vs analytical expectation, (c) confirmation of visual parity, (d) any ambiguities.

## **Prompt Tester**: Following remove-noop Prompt Instructions (Cycle 1)
Following the instructions, I would:
1. Remove files: delete `compute_noop.rs` & `metaballs_noop_compute.wgsl`; strip plugin wiring.
2. Add new WGSL gradient shader + Rust module per sections 7–9.
3. Half-res size: 800x600 -> 400x300; workgroups 8x8 -> dispatch dims = ceil(400/8)=50, ceil(300/8)=38 => dispatch(50,38,1).
4. Central pixel world_pos ~ (0,0). For 3 overlapping balls radius 40 at slight offsets, each contributes F_i ≈ (1 - d^2/r^2)^3. If arranged tightly, assume d≈0 for all => each ≈1^3=1. Field ≈3. Gradient≈0 (symmetric). Stored: R≈3, G≈0, B≈0, A=dominant_cluster (depends which cluster accumulates larger total; with 0,1,0 pattern cluster 0 has 2 units vs cluster 1 has 1 => A=0 if implemented; else 0 placeholder).
5. Fragment still brute-forces balls — visuals unchanged.
6. Ambiguities observed:
   * Exact mapping from half-res pixel to world_pos: spec outlines but slight nuance on 0.5 center; may need explicit formula mirroring fragment’s vertex path.
   * Whether to clamp field or leave raw (spec says raw). Need numeric range guidance for large N.
   * Cluster accumulation optional; if deferred, require explicit A=0 semantics.
   * Toggle storage location (config vs resource) not fully standardized.
7. Success criteria otherwise clear; tests relying on GPU readback may need conditional compilation or a helper.

Feedback: Clarify world position reconstruction formula with an explicit example, and specify storing raw (unclamped) field. Otherwise prompt is actionable.

<!-- Validation Cycle 1 complete -->
