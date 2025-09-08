<!--
Prompt Name: Metaballs Unified Shader Restoration (Advanced Rendering + SDF Integration)
Purpose: You WILL restore the richer foreground/background rendering, per-cluster accumulation, adaptive AA mask, bevel / outline glow lighting, metadata output, and background noise/gradient modes from the `main` branch shader into the CURRENT simplified SDF‑enabled shader contract (`metaballs_unified.wgsl`) on the `sdf` branch WITHOUT breaking the new SDF shape + distance field sampling path or its uniform/binding layout additions (v5/v6 semantics, glyph SDF mask, feather logic). You WILL preserve WASM compatibility, uniform packing, and binding indices. Heightfield / height‑map or legacy height generation logic present in older iterations is deprecated and MUST NOT be reintroduced.
Scope: Shader WGSL refactor + (minimal) Rust side adjustments needed ONLY to feed any reinstated scalar lanes already defined in the current uniform contract. No new bindings. Optional incremental CPU optimizations (tiling early‑exit reinstatement) allowed but gated.
CRITICAL: Maintain current contract exactly for all existing uniforms & bindings; ONLY add logic that can operate with already supplied data. If a new scalar is absolutely necessary, pack it into UNUSED padding lanes already present (document). Avoid changing struct field counts or ordering.
References: `assets/shaders/metaballs_unified.wgsl` (current simplified), user‑provided MAIN version snippet (2025‑08‑31), `.github/copilot-instructions.md` (rendering, performance, safety), glyph SDF integration prompt (for shape index packing & sampling expectations).
-->

## 1. High‑Level Goal
You WILL merge the feature richness of the MAIN shader into the CURRENT SDF‑capable simplified version so that:
1. Per‑cluster (group) accumulation returns (dominant cluster detection, gradient, approximate radius capture) while retaining optional SDF glyph masking per ball.
2. Foreground modes: ClassicBlend, Bevel, OutlineGlow, Metadata fully restored & extended to cooperate with SDF glyph silhouettes.
3. Background modes: SolidGray, ProceduralNoise, VerticalGradient restored.
4. Adaptive edge mask (gradient aware) & legacy mask both available (toggle via existing `metaballs.v4.w`).
5. Metadata mode produces enhanced diagnostic channels while accommodating SDF (raw glyph sample, mask, signed distance proxy) under a debug sub‑flag.
6. SDF sampling (distance field > 0.5 interior) continues to gate per‑ball contribution and optionally visualize glyph metrics in Metadata mode.
7. No heightfield / displaced normal fields reintroduced (DEPRECATED: any previously present height or pseudo-depth layering beyond bevel normal reconstruction).

## 2. Success Criteria (MANDATORY)
All MUST be satisfied:
1. Uniform / binding layout unchanged: structs (`MetaballsData`, noise, surface_noise, balls, tile_headers, tile_ball_indices, cluster_palette, sdf_atlas_tex, sdf_shape_meta, sdf_sampler) remain identical in field order and binding indices (0..9) as current simplified shader.
2. v5 / v6 semantics preserved: (v5.x = sdf_enabled, v5.y = feather_halfwidth_norm, v5.z = channel_mode (reserved), v5.w = max_gradient_samples), (v6.x/v6.y/v6.z/v6.w = atlas_width, atlas_height, atlas_tile_size_px, gradient_step_scale). No repurposing.
3. Cluster accumulation: Reimplemented using either
   a. Existing tile list (tile_headers + tile_ball_indices) with early exit when allowed, OR
   b. Fallback full scan loop behind a compile‑time or uniform gate if tile data not populated.
   In both cases contributions respect SDF masking (mask multiplies per‑ball field contribution).
4. Gradient & approx radius recorded for dominant cluster when `needs_gradient` (as before). Gradient math unaffected by SDF except masked magnitude scaling (document limitation).
5. Adaptive mask logic (gradient based) reinstated behind `enable_adaptive_mask` (metaballs.v4.w>0.5). Legacy mask retained for fallback.
6. Foreground shading parity: visual output (qualitatively) matches MAIN within tolerance when SDF disabled (±0.02 average RGB difference) under ClassicBlend in a benchmark scene.
7. Metadata mode: retains legacy channels (R distance proxy, G clickable mask, B high8 or SDF debug if glyph present, A low8 / cluster u8). When SDF glyph selected & `debug_view==3` (new reserved debug code), override: R=raw_sdf, G=mask, B=distance_vis, A=cluster_low8 (or metadata v2 mapping if enabled).
8. Performance: No additional dynamic allocations per fragment. Tile early‑exit path measured (optional manual test) reduces average fragment iterations vs naive loop for dense scenes (log instrumentation stub acceptable, compiled out in release).
9. WASM build unaffected (no unsupported features). Validation via `cargo build --target wasm32-unknown-unknown` passes.
10. No regressions in SDF glyph silhouette correctness: circular fallback (shape_idx==0) unchanged; masked contributions never increase field outside original circle support.
11. Documentation comments updated at top of shader summarizing new hybrid feature set & reserved fields.
12. No heightfield logic (no extra textures, no parallax, no multi‑sample derivatives) appears in final shader.

## 3. Non‑Goals (You MUST NOT)
You MUST NOT:
1. Change or reorder any uniform/struct fields or bindings.
2. Introduce new sampler / texture bindings (future palette texture remains TODO).
3. Add multi‑pass rendering or additional entry points.
4. Reintroduce removed heightfield / displacement map code.
5. Depend on derivative ops (dpdx/dpdy) for AA (keep deterministic mask approach for adapter stability).
6. Expand fixed array maxima inside uniform structs (GPU limits risk) or add unbounded loops beyond existing caps.

## 4. Current vs Target Delta (Analysis)
| Aspect | Current Simplified (sdf branch) | Main Version (reference) | Target Hybrid |
|--------|--------------------------------|---------------------------|---------------|
| Field Accumulation | Single pass over all balls; no grouping; no gradient | Cluster grouping with K_MAX tracked + gradient | Reintroduce grouping & gradient; integrate SDF mask per ball pre‑accumulation |
| Background Modes | None (implicit black / flat) -> Actually classic grayscale only | SolidGray / Noise / Vertical | Restore all three with selection via `bg_mode` (already packed) |
| Foreground Modes | Classic grayscale; Metadata gist | ClassicBlend / Bevel / OutlineGlow / Metadata | Full set; adapt shading to masked field |
| Adaptive Mask | Not present | Gradient aware optional | Restore; guard with v4.w flag |
| Early Exit | None | Early exit when not needing gradient & threshold reached | Reintroduce with SDF aware iso adjustments (iso unaffected here yet) |
| Surface Noise | Removed (placeholder uniforms) | Foreground edge modulation (surface noise additive / iso shift) | REINTRODUCE optional surface noise only if we decide to preserve – BUT user did not request noise explicitly? Main features mention shading, backgrounds. Surface noise optional; include additive variant minimal. |
| SDF Glyph Mask | Present (circle masked by glyph sample) | Absent | Retain and integrate pre‑accumulation |
| Metadata V2 | Not yet (current simplified just shows sample/mask debug) | Implemented hi/lo cluster | Add metadata v2 toggle using existing `metaballs.v4.z` (rename maintain) |

## 5. Data & Packing Contracts (Recap)
Per‑ball packed gid (GpuBall.data0.w): Upper 16 bits = shape_index (SDF glyph), Lower 16 bits = cluster/group id.
`cluster_palette` buffer indexed by group id; length passed via `metaballs.v0.y` (cluster_color_count).
SDF sample interpretation: sample > 0.5 inside; mask = smoothstep(-f, f, sample-0.5) with feather half‑width f = clamp(feather_norm, 1e-5, 0.5).
Masked contribution: `fi_masked = fi * mask_val` (applied BEFORE grouping accumulation). Gradient remains derivative of base metaball field only (limitation documented).

## 6. Foreground & Background Mode Behavior (Target)
Foreground modes (fg_mode): 0 ClassicBlend, 1 Bevel, 2 OutlineGlow, 3 Metadata (unchanged discriminants).
Background modes (bg_mode): 0 SolidGray, 1 ProceduralNoise, 2 VerticalGradient.
Debug view (debug_view):
* 0 Normal
* 1 Raw Field Grayscale override (except metadata)
* 2 Reserved (future assertions)
* 3 SDF Glyph Debug (Metadata only: raw sample/mask/distance channels)

## 7. Adaptive Mask Logic
If `enable_adaptive_mask (v4.w>0.5)` and gradient available for dominant cluster:
```
grad_len = max(length(grad), 1e-5)
aa_width = clamp(iso / grad_len * 0.5, 0.75, 4.0) // heuristics preserved
mask = smoothstep(iso - aa_width, iso + aa_width, best_field)
```
Else legacy ramp: `mask = smoothstep(iso * 0.6, iso, best_field)`.

## 8. Early Exit Policy
Allow early exit ONLY when: `!needs_gradient && !sdf_enabled` OR (`sdf_enabled` but feather small & no surface noise additive). For first restore iteration you MAY simply guard by `!needs_gradient` and measured performance is acceptable (document).

## 9. Surface Noise (Optional Minimal Reintroduction)
If reintroduced: only additive post accumulation mode (sn_mode=0) using existing surface_noise uniforms. Iso‑shift mode can be deferred. Provide TODO for iso‑shift early exit integration later. Keep code block compartmentalized so it can be feature‑gated out trivially.

## 10. Metadata Mode (Hybrid SDF Support)
When `fg_mode==3`:
* If `debug_view==3 && last_shape_idx>0`: Output `(raw_sdf_sample, last_sdf_mask, glyph_distance_vis, cluster_low8_norm)`.
* Else implement Metadata V1/V2 logic: R = normalized signed distance proxy (dominant cluster) centered at 0.5 iso; G = clickable mask (use mask); B/A = cluster id hi/lo if metadata_v2_enabled else (0, cluster_u8_norm).
* Signed distance proxy uses same adaptive radius scaling from MAIN; use approx radius of dominant cluster if available else fallback 8.0 scale.

## 11. Implementation Steps (ORDERED)
1. Copy header constant & doc blocks from MAIN into current shader; merge with existing SDF header commentary; add Hybrid Feature Summary section.
2. Reintroduce constant definitions (K_MAX, epsilons, etc.). Keep SDF constants separate.
3. Port `AccumResult` struct & `accumulate_groups_tile` logic; update ball data access pattern (`b0 = balls[bi].data0` etc.) & cluster id extraction (`cluster = packed_gid & 0xFFFFu`).
4. Insert SDF mask integration inside accumulation loop: if SDF mask computed for ball & contrib>0 then scale `fi` by mask before group accumulation.
5. Track `approx_r` (scaled radius) like MAIN for adaptive SDF distance proxy scale.
6. Add gradient computation conditioned on `needs_gradient`.
7. Reintroduce `compute_mask`, adaptive mask path.
8. Restore background helper functions (noise_color, bg_solid_gray, bg_noise, bg_vertical) using existing noise uniforms (these are still bound; safe to reuse). Keep value noise implementation (hash2, fade, value_noise). Ensure no name conflicts with SDF helpers.
9. Port bevel, outline glow functions; adapt parameter names; ensure no reliance on removed fields.
10. Metadata path: integrate hybrid logic (SDF glyph debug variant). Use local `last_sdf_sample`, `last_sdf_mask`, `last_shape_idx` tracked during accumulation (update when a ball with shape mask influences the dominant cluster OR simply store most recent SDF sampled ball; acceptable initial heuristic—document).
11. Provide fallback when `acc.used==0` returning background color or metadata sentinel.
12. Optionally reintroduce early exit with simple guard; mark TODO to refine with iso shift + SDF interplay.
13. Clean up unused functions from simplified version (e.g., `shade_classic`, `shade_metadata`) replaced by new FG pipeline.
14. Add top‑level comment delineating new version date & change log summary.
15. Run build & adjust any naming collisions.
16. Spot test: a) SDF disabled should visually match MAIN reference (qualitatively). b) SDF enabled with glyph shows glyph silhouette colored & lit. c) Metadata SDF debug shows distinct channels.
17. (Optional) Add a `debug_view==4` for SDF feather band visualization (deferred; document TODO only).

## 12. Testing Strategy
Add / update automated tests (Rust side) where practicable:
1. `surface_noise_uniform_size` already validates uniform size; ensure restoration doesn’t change layout.
2. NEW shader logic mostly visual; add a small CPU side reference field accumulation function (pure) mirroring grouping and SDF mask multiplication to test: grouping sum invariance vs naive accumulation (unit test with deterministic inputs).
3. Add test ensuring mask monotonicity: increasing feather width never produces sharper (lower) alpha at iso edge (approx via sampling synthetic gradient scenarios).
4. If not feasible inside timeframe, document TODO with rationale; implement at least one pure function test for `compute_mask` adaptive vs legacy path.

## 13. Performance Considerations
* Keep loops bounded (K_MAX=12 clusters tracked).
* Use early exits when `allow_early_exit && best_field >= iso` and `!needs_gradient`.
* Avoid texture sampling for balls whose analytic `contrib==0` (already done) or shape_idx==0.
* Defer multi‑sample SDF gradient (max_gradient_samples reserved) – leave stub referencing v5.w & v6.w if future per‑pixel SDF normal retrieval implemented.
* Document limitation: gradient ignores SDF mask, so fine glyph features may over‑inflate adaptive AA width; acceptable initial tradeoff.

## 14. Logging & Debugging Hooks
Shader itself cannot log; add TODO markers where CPU side can optionally enable a stats overlay counting average balls processed per fragment (instrument accumulation iterations via atomic or debug storage buffer—deferred).
Add `// DEBUG:` prefixed comments around key decision branches (adaptive mask, SDF mask gating) to assist future temporary instrumentation.

## 15. Documentation Updates
Update shader header with:
* Version date.
* Summary bullet list of restored features.
* NOTE that heightfield path deprecated.
Add README section (if README generation script supports) summarizing SDF + Advanced Modes interplay (TODO if script not yet aware).

## 16. Edge Cases & Handling
* Zero clusters contributing -> return background (or metadata sentinel RGBA = (1,0,0,0)).
* All contributing balls share shape_idx==0 (circle) -> identical to legacy visuals.
* Feather width 0 (hard edge) -> clamp to epsilon (already in sdf_mask).
* Very small dominant gradient -> fallback distance proxy R channel = 0.5.
* Cluster id >= palette length -> clamp to last color (prevent OOB).

## 17. Deferred / TODO (Document but DO NOT Implement Now)
* Proper SDF‑aware gradient (finite difference texture sampling outside per‑pixel budget; would use v5.w sample count & v6.w step scale).
* Iso shift surface noise path (sn_mode==1) reintegration aligning early exit threshold.
* Palette texture sampling (1D LUT) for cluster colors.
* Per‑ball clickability flag packing into gid high bits.
* Parallel accumulation using subgroup ops or per‑tile compute path.

## 18. Completion Checklist
- [ ] Uniform & binding layout unchanged (diff vs previous commit shows WGSL logic only).
- [ ] Foreground modes restored & selectable.
- [ ] Background modes restored & selectable.
- [ ] Adaptive & legacy mask both function (visual manual test).
- [ ] Metadata mode includes SDF debug variant when `debug_view==3`.
- [ ] SDF glyph masking still works (glyph edges honor feather).
- [ ] No heightfield code present.
- [ ] Build passes native + wasm.
- [ ] At least one new unit test for mask or accumulation pure logic OR TODO documented with justification.
- [ ] README or TODO note for documentation update added.

## 19. Rollback Strategy
If visual regressions severe: temporarily switch material to legacy simplified shader (retain file), OR compile‑time feature gate advanced accumulation & shading (introduce `ADVANCED_METABALLS` cfg). Because layout unchanged, rollback is just selecting the old fragment entrypoint; keep old code under `#if 0` style block or separate file for quick diff until stable.

## 20. Risk Mitigation
* Implement incrementally: (a) accumulate+mask, (b) background modes, (c) FG shading modes, (d) metadata hybrid, (e) adaptive mask.
* After each stage, build & visually sanity check.
* Use small synthetic scenes (few balls, one glyph) and dense scenes for performance spot checks.

---

## **Prompt Builder**: Requesting Validation
Prompt Tester, please follow the restore-shader prompt using this scenario:
Scene: 9 balls across 3 clusters (ids 0,1,2). Clusters 0 & 1 have glyph shapes (shape_idx=5 for 'A', shape_idx=9 for 'Z'), cluster 2 analytic circle (shape_idx=0). SDF enabled (v5.x=1), feather_halfwidth_norm=0.2, iso=1.0, fg_mode=1 (Bevel), bg_mode=1 (ProceduralNoise), adaptive_mask enabled, metadata_v2 flag off, debug_view=0. All balls sufficiently large so glyph area fully visible. Dominant cluster at test pixel = cluster 1 with masked field 0.95 < iso initially then plus another contributing ball raises to 1.05.
Demonstrate: (a) how masked field accumulates per cluster, (b) adaptive mask computation, (c) bevel lighting inputs, (d) resulting RGBA blend with background, (e) metadata SDF debug output for same pixel if debug_view switched to 3.
Identify any ambiguities or missing guidance encountered.

<!-- End of Prompt v1 -->
