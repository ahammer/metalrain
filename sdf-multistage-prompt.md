<!--
Prompt Name: Metaballs Multi‑Stage Pipeline (Monolith Removal)
Purpose: You WILL completely remove the legacy monolithic shader + material (`metaballs_unified.wgsl` & `MetaballsUnifiedMaterial`) and replace it with a clean, explicit multi‑stage pipeline beginning with: (1) Distance Field + Dominant Cluster Compute, (2) Simple Composition (mask + color over background). You WILL lay a foundation that can be incrementally extended with Normals, Lighting, Surface Noise, Advanced Clustering, and Shadows WITHOUT revisiting the architectural core.
Scope (Phase 1 / MVP): Shader removal, new compute + compose shaders, new plugin + render graph node, intermediate textures, reuse existing GPU data (balls, tiles, palette, sdf atlas). No normals / bevel / surface noise yet.
CRITICAL: Preserve WASM compatibility, uphold existing uniform struct layout (or provide a transitional compatibility layer), avoid feature regression for core visuals (field-based iso surface & coloring). NO partial duplication of the old path—fully culled.
References: `src/rendering/metaballs/metaballs.rs` (for data prep patterns), `assets/shaders/metaballs_unified.wgsl` (for field + SDF logic to port), `.github/copilot-instructions.md` (all architectural & performance conventions), existing tiling + palette resources.
Rollback: Git revert only. No runtime toggle to legacy path.
-->

## 1. High‑Level Goal
You WILL refactor the metaball rendering into a multi‑stage GPU pipeline: a compute pass producing a distance field & dominant cluster id textures, then a lightweight fullscreen composition pass that applies iso threshold + palette color over a background. The legacy monolithic shader is deleted.

## 2. Success Criteria (MANDATORY)
All MUST be satisfied:
1. Legacy Removal: `assets/shaders/metaballs_unified.wgsl` deleted; `MetaballsUnifiedMaterial`, related systems & plugin registration removed (no dead imports, no unused types left except reused structs like `GpuBall`).
2. New Shaders: Three new WGSL files created:
   * `metaballs_field.comp.wgsl` – compute distance field + dominant cluster id per pixel using existing tiles + balls + SDF glyph shapes.
   * `metaballs_compose.frag.wgsl` – sample field + cluster id textures, apply iso mask, blend foreground cluster color with minimal background (SolidGray only in MVP) producing RGBA.
   * `fullscreen_passthrough.vert.wgsl` – simple fullscreen vertex stage (clip‑space quad) (or reuse existing but isolate from legacy naming).
3. Intermediate Textures: Two render‑sized GPU images allocated & resized on window changes:
   * Field Texture: r32float STORAGE + TEXTURE (write by compute, read by fragment).
   * Cluster Texture: r8uint STORAGE + TEXTURE (dominant cluster id, 255 sentinel for none).
4. Render Graph: Custom node dispatches compute BEFORE the 2D composition draw. Node schedules after tile building (`MetaballsUpdateSet`) and before the main 2D pass (or a dedicated composition pass).
5. Uniform Continuity: Existing uniform data struct lanes used where required (v0: counts, v2: viewport/time, v3: tiling meta, v5: sdf flags). Unused lanes tolerated; no reshuffling that would break SDF atlas loading.
6. Visual Parity: Result matches previous monolithic shader for: iso thresholding, SDF glyph silhouette masking, palette color selection (dominant cluster), alpha mask shape. Minor floating point variance acceptable (< 1e-3 mask difference).
7. Performance: Frame time not worse than legacy path at equal resolution (objective: <= legacy ±5%). Compute workgroup dimensions chosen conservatively (8x8) for WASM portability; no >256 thread groups.
8. SDF Path: Glyph masking logic preserved (rotation, uv derivation, feather half‑width semantics using existing v5.y). Hard fallback (shape index 0) behaves identically to analytic circle field.
9. Safety: No panics; all missing resources early‑return gracefully (empty textures cleared to zero). WASM build passes without requiring additional features.
10. Tests: Add at least one integration or unit test verifying (a) compute iso invariants on a synthetic single ball (center field > rim field), (b) cluster id sentinel when no balls.
11. Logging: Informational log once on pipeline init (target="metaballs") and warn if allocation / resize fails (with fallback strategy). No per‑pixel logging.
12. Clippy & Tests: `cargo clippy --all-targets --all-features` & test suite pass.

## 3. Non‑Goals (Phase 1)
You MUST NOT implement normals, bevel lighting, surface noise modulation, multi‑cluster blending, shadows, background noise gradients, or metadata output (reserved hooks only). You MUST NOT add new config fields unless strictly necessary for gating; reuse existing iso / radius multiplier / palette paths.

## 4. Data & Binding Contracts (Reuse)
Group(2) (unchanged):
```
@binding(0) MetaballsData (uniform)
@binding(1) NoiseParams (unused in MVP)
@binding(2) SurfaceNoiseParams (unused)
@binding(3) balls (storage readonly)
@binding(4) tile_headers (storage readonly)
@binding(5) tile_ball_indices (storage readonly)
@binding(6) cluster_palette (storage readonly vec4 colors)
@binding(7) sdf_atlas_tex (optional)
@binding(8) sdf_shape_meta (storage readonly)
@binding(9) sdf_sampler
```
New Group(3):
```
@binding(0) write storage texture r32float  (field_out)
@binding(1) write storage texture r8uint    (cluster_out)
@binding(2) sampled texture_2d<f32> field_in     (view of field_out) 
@binding(3) sampled texture_2d<u32> cluster_in   (view of cluster_out)
@binding(4) sampler linear_or_point (point acceptable; filter not needed for MVP)
```
(If Bevy renderer resists dual binding as storage + sampled in same pass, create two views / bind groups—document deviation.)

## 5. Compute Shader Specification (`metaballs_field.comp.wgsl`)
Workgroup Size: `@workgroup_size(8,8,1)`.
Inputs: viewport dims (v2.xy), tiles meta (v3.xyz), iso (v0.w), radius scaling (v0.z * v2.w), SDF enable + feather (v5.x,y), arrays.
Per pixel algorithm:
1. Compute global pixel coords (gid.xy). Early return if out of bounds.
2. Convert to world space: `world = ((vec2(px,py)+0.5)/vec2(w,h) - 0.5) * vec2(w,h)`.
3. Tile look-up: derive tile index -> fetch header -> iterate its indices.
4. For each candidate ball:
   * Load center & scaled radius (base * radius_coeff).
   * Skip if radius <= 0.
   * Compute polynomial field contribution `f_i = (1 - d2/r2)^3` (skip if negative).
   * Extract packed shape + cluster id; if SDF enabled & shape>0 run glyph sampling (rotate, uv, clamp, mask) & multiply contribution.
   * Track dominant cluster: if `f_i > best_f` update `(best_f, cluster_id)`.
5. Write outputs:
   * field_out[pixel] = best_f (or 0.0 if none)
   * cluster_out[pixel] = cluster_id (or 255u if none)
No atomics, no barriers beyond implicit per invocation.

## 6. Fragment Composition (`metaballs_compose.frag.wgsl`)
Inputs: field texture (sampled), cluster id texture (uint), palette, iso, viewport dims, background mode (restrict to SolidGray in MVP ignoring v1.z if nonzero).
Steps:
1. Sample field f.
2. `mask = smoothstep(iso*0.6, iso, f)`.
3. If mask == 0: output background RGBA (0 alpha).
4. Else: sample cluster id; if sentinel (255) treat as background.
5. Clamp cluster index to palette_count - 1 (palette count from v0.y; treat 0 => single fallback color index 0).
6. `rgb = mix(bg, fg, mask)`; alpha = mask.
7. Output vec4.

## 7. Vertex Shader (`fullscreen_passthrough.vert.wgsl`)
Standard single quad in clip space [-1,1]²; pass through a UV or world pos if needed (for future background effects). MVP can reconstruct world from gl_FragCoord if simpler.

## 8. Rust Refactor Steps (Ordered)
1. Delete `assets/shaders/metaballs_unified.wgsl`.
2. Remove `MetaballsUnifiedMaterial` type, related plugin registration & systems from `src/rendering/metaballs/metaballs.rs` (retain reusable structs: `GpuBall`, tiling systems, palette storage). If file becomes bloated with legacy only logic, split KEEP code into `metaballs_data.rs` and shrink original file or rename module to `metaballs_data`.
3. Create new module: `src/rendering/metaballs_pipeline/` with:
   * `mod.rs` – defines `MetaballsPipelinePlugin`.
   * `resources.rs` – intermediate texture handles, resized flag.
   * `graph.rs` – render graph node implementing compute dispatch (extract / prepare phases updating bind groups on resize).
   * `shaders.rs` (optional) – constants for shader paths & loaders (wasm include_str! pattern mirrored).
4. Add intermediate texture creation system (Startup + on resize) producing:
   * `Image` for field (Format::R32Float, usage: TEXTURE_BINDING | STORAGE_BINDING | COPY_SRC)
   * `Image` for cluster (Format::R8Uint)
   Provide default clear values (0, sentinel 255 for cluster via compute init pass or texture clear if Bevy exposes; else first frame compute writes).
5. Port tile build system to remain unchanged; ensure it runs before compute node.
6. Implement compute node:
   * Acquire pipeline (lazy init) with layout referencing group(2) existing + group(3) outputs.
   * For each frame: ensure dispatch dims = ceil(vw/8), ceil(vh/8).
7. Composition material:
   * Simple material struct referencing uniform (MetaballsData) + sampled views of field/cluster + storage palette.
   * Vertex = fullscreen pass; Fragment = composition shader.
   * Spawn quad at z=50 (reuse existing transform ordering).
8. Uniform update system: replicate essential fields from old path (iso, counts, time, radius scale, viewport size). Remove unused lanes gracefully.
9. SDF enable logic: keep same inference from existing resources (atlas enabled & config flags) setting v5.x and feather v5.y.
10. Palette upload: reuse existing palette buffer logic (NO functional change). cluster count stored in v0.y.
11. Remove shadow logic & other unused lanes for MVP (leave zeros). Document future repurposing in code comments.
12. Logging: On plugin init: `info!(target="metaballs", "Multi-stage metaballs pipeline initialized (compute+compose)")`.

## 9. Testing & Validation
Add tests (integration or unit):
1. `single_ball_field_profile`: Build minimal app, insert one ball radius R at origin; after one frame read back a small CPU copy of field texture (via readback pipeline or expose helper) OR call the pure CPU analog function verifying center field > edge field > far field.
2. `empty_scene_outputs_background`: No balls -> cluster texture sentinel & alpha 0 output.
3. `sdf_mask_respect`: Provide a ball with shape index 0 vs nonzero ensuring shape>0 path multiplies contribution (simulate SDF sample==1 interior case via test double if direct sampling awkward).
If GPU readback is heavy, factor field contribution + glyph mask logic into a pure helper in Rust mirrored by shader (already partially exists) and test that.

## 10. Performance Considerations
You WILL:
* Use 8x8 workgroups initially. Document a comment explaining rationale & a TODO to benchmark 16x16.
* Avoid per-frame allocations in compute node (cache bind groups & pipeline). Recreate ONLY on resize or when image handles change.
* Avoid iterating all balls inside compute (tiling assures localized work). Ensure tile headers & indices unchanged.
* Ensure cluster palette storage not rebuilt when length unchanged.

## 11. Edge Cases & Handling
* Zero balls: write field=0 cluster=255 sentinel.
* Palette length 0: treat as 1 with fallback color (index 0) – prevents OOB (mirror legacy guard).
* Atlas absent or disabled: treat shape_idx>0 as analytic circles (skip SDF sampling path conditionally).
* Extremely small radii: contribution may underflow; acceptable—clamped after polynomial.
* Resize mid-frame: ensure next frame compute dispatch uses updated textures. Use a dirty flag.

## 12. Logging Policy
Target `metaballs` only:
* INFO on init.
* DEBUG (optional) on resize with new dims.
* WARN if texture allocation fails (fallback: skip compute -> composition outputs empty background; log once until recovery).
No verbose per-frame cluster counts (remove prior periodic log).

## 13. Documentation Updates
Update README generation script input (or add TODO) to reflect:
* New multi-stage pipeline description.
* Field + cluster textures intermediate.
* Future extension hooks.
Add a brief comment in removed legacy file path commit message referencing this prompt.

## 14. Future Extensions (DO NOT IMPLEMENT NOW)
* Normal buffer (rg16f or octahedral packed) derived from field central differences.
* Lighting compute pass (blinn-phong / bevel extrusion) writing color+mask buffer.
* Multi-cluster accumulation (top-K) stored in separate structured buffer for stylized blending.
* Background procedural noise moved to separate compute feeding a cached texture.
* Surface noise & edge distortion using SDF grad; requires stable gradient buffer.
* Shadow via offset sampling of field / signed distance thickness estimation.

## 15. Completion Checklist
You MUST verify before merge:
- [ ] Legacy shader & material removed (grep shows no references to `metaballs_unified` or `MetaballsUnifiedMaterial`).
- [ ] New shaders compiled without validation errors (native + wasm).
- [ ] Field & cluster textures allocated & resized on window change.
- [ ] Compute dispatch executes (log or debug marker) before composition draw each frame.
- [ ] Visual parity within acceptable delta for iso + palette.
- [ ] SDF glyph silhouettes still render (when atlas active) with feather preserved.
- [ ] Tests added & passing.
- [ ] Clippy & wasm build succeed.
- [ ] README (or TODO) updated referencing multi-stage pipeline.

## 16. Rollback Strategy
Git revert of the commit(s) removing legacy path. No in-code fallback. If emergency mid-development: temporarily retain local copy of old shader in a branch (not committed to mainline after this migration).

## 17. Implementation Order (Enforced)
1. Add new shaders (empty stubs) & pipeline module scaffold.
2. Introduce textures + compute node (no logic yet) -> build passes.
3. Port field accumulation (compute) using existing scalar code.
4. Port SDF glyph masking logic.
5. Implement cluster dominance (single id only).
6. Implement fragment composition (solid background only).
7. Remove legacy material & shader, adjust plugin wiring.
8. Add tests.
9. Add README update / TODO.
10. Final polish (logging, comments, clippy).

## 18. Failure Modes & Warnings
If any of the following occur you MUST stop & address before continuing:
* Validation error: storage texture format mismatch (fix binding layout / usage flags).
* Workgroup size unsupported on wasm target (reduce to 8x8 or query limits).
* Palette indexing OOB (add clamp guard for length=0 path).
* SDF sampling produces NaNs (verify clamp & feather path; enforce finite results with `clamp`).

## 19. Code Comments & Style
* Each WGSL file MUST include a short header block summarizing purpose + future extension hooks.
* Compute shader MUST mark TODOs for normals & multi-cluster extension with `// TODO:` tags.
* Keep functions small; factor SDF evaluation identical to legacy to ease diff tracking.
* Avoid over-optimizing prematurely; readable first-pass.

## 20. Security & Safety
No new IO. No dynamic unsafe Rust. Keep all GPU resource creation guarded against zero-dimension textures.

<!-- End of Prompt v1 -->
