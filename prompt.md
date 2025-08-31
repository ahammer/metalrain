## Metaballs Shader Refactor Prompt (Target: `assets/shaders/metaballs_unified.wgsl`)

<!-- Summary -->
You WILL refactor and extend the WGSL shader `assets/shaders/metaballs_unified.wgsl` to address correctness, clarity, and extensibility issues identified in an audit. You MUST implement the mandatory fixes first, then optional enhancements if time permits. You MUST preserve existing visual output within a tolerance (see Success Criteria) except where changes are explicitly intended (metadata encoding, adaptive AA). All edits MUST be localized and minimal unless a rewrite is explicitly directed.

---
### 1. Objectives
**Primary Goals (MANDATORY):**
1. Fix early-exit correctness when iso may shift due to surface-noise iso mode (`sn_mode == 1`).
2. Improve metadata mode encoding to remove channel overloading ambiguity (clean separation of SDF + click mask + cluster id u16).
3. Center surface noise after contrast application logic (eliminate mean bias when `contrast != 1.0`).
4. Provide adaptive SDF normalization scale for metadata mode (replace hardcoded `d_scale = 8.0`).
5. Add gradient-aware mask option with robust fallback (retain legacy constant ramp for platforms where derivatives misbehave). No driver-unstable ops (avoid `dpdx/dpdy`).

**Secondary Enhancements (Optional AFTER primary):**
6. Gate/disable early-exit if gradient required or iso can shift; micro-opt path comment.
7. Abstract foreground/background mode discriminants into a generated header block (constexpr section) to ensure sync with Rust enum.
8. Add dev assert guards (debug flag lane) for out-of-bounds tile index or `tile.offset + tile.count` overflow.
9. Prepare future path for moving `cluster_colors` to a sampled 1D texture (scaffolding only, feature flag off by default).

---
### 2. Constraints
You MUST:
- NOT change existing bind group layout or binding indices in this iteration.
- Keep `MAX_CLUSTERS`, `K_MAX` unchanged.
- Maintain metadata legacy compatibility behind a feature flag uniform bit (reuse existing `metaballs.v4.z` boolean lane as `metadata_v2_enabled`).
- Avoid adding new uniform buffers; pack new scalar toggles into unused lanes (`metaballs.v4.z / v4.w`) or repurpose reserved lanes with documented mapping.
- Keep performance regression < 2% GPU time (heuristic) for typical scene (assume 1080p, average 6 clusters visible, tiles unchanged).

---
### 3. Uniform Layout Adjustments
Repurpose `metaballs.v4` lanes:
```
v4.x = enable_early_exit (unchanged)
v4.y = needs_gradient (hint) (unchanged semantic)
v4.z = metadata_v2_enabled (already used) – keep meaning
v4.w = enable_adaptive_mask (new; 0 = legacy, 1 = adaptive gradient-based)
```
Add new scaling control for metadata SDF:
If adaptive scale enabled (see Section 5), compute dynamic scale; no new uniform required—derive from existing ball radii & viewport.

---
### 4. Fix: Early-Exit with Iso Shift (MANDATORY)
Problem: Accumulation early-exit uses `effective_iso` before potential modification by surface noise iso shift (`sn_mode == 1`), causing under-filled silhouettes.
Solution:
1. Precompute `iso_shift` BEFORE calling `accumulate_clusters_tile` when `sn_enabled && sn_mode == 1` using surface noise function (needs only `p`, `time_seconds`). This path cannot rely on field values.
2. Set `effective_iso = iso + (iso_shift - amp * 0.5)` where `amp = surface_noise.v0.x`.
3. Pass that `effective_iso` into `accumulate_clusters_tile` so early-exit condition is correct.
4. For additive surface noise mode (`sn_mode == 0`), keep current sequence (apply after accumulation) because it depends on the field, but DISABLE early-exit if additive mode is active (field could cross iso only after perturbation). Implementation: inside fragment, set `local_enable_early_exit = enable_early_exit && !(sn_enabled && sn_mode == 0u) && !needs_gradient` and pass that into accumulator.

---
### 5. Adaptive Metadata SDF Scale (MANDATORY)
Replace constant `d_scale = 8.0` with:
```
// Attempt to approximate dominant blob pixel radius.
// We lack per-cluster radius after accumulation, so we: 
//   - Track an approximate contributing radius while accumulating for the dominant cluster only.
// Add a new array in AccumResult: approx_r[K_MAX] (f32) storing last seen scaled_r for that cluster.
// After dominant selected, let r_px = approx_r[dom] (already world units == pixels in current coordinate space if world_pos maps 1:1 to pixels; if not, scale by viewport ratio if required).
// d_scale = clamp(r_px * 0.25, 4.0, 24.0)  // heuristic
```
If no radius captured (zero), fallback to 8.0.

---
### 6. Metadata Mode Encoding (MANDATORY)
Legacy (metadata_v2_enabled == 0):
```
R = normalized SDF proxy
G = clickable mask
B = 0.0 constant
A = cluster_u8
```
New (metadata_v2_enabled == 1):
```
R = normalized SDF proxy
G = clickable mask
B = cluster_id_hi8 (cid16 >> 8) / 255.0
A = cluster_id_lo8 (cid16 & 255) / 255.0
```
Remove earlier ambiguity where B doubled as “non-clickable mask”. Remove variable `non_clickable` usage in new path. Update comments accordingly.

---
### 7. Surface Noise Centering (MANDATORY)
Issue: Contrast exponent applied before centering introduces mean shift.
Revision (both background and surface noise helpers):
1. Accumulate raw fractal noise to `n_raw` in [0,1].
2. Apply any ridged transform per octave (unchanged semantics) during accumulation.
3. After normalization & clamp: compute `n_c = n_norm - 0.5`.
4. If `contrast != 1.0`: apply `n = pow(clamp(n_norm, 0.0, 1.0), contrast)` THEN re-center: `n = n - 0.5` (maintaining zero mean assumption). For amplitude usage that expects [0,1], re-map as needed: `n01 = n + 0.5`.
5. For iso shift mode: use `n01` but with bias removal: `delta = (n01 - 0.5) * amp + 0.5 * amp` (or directly use centered form consistent with earlier code). Keep consistent across both noise utilities.
Document difference.

---
### 8. Gradient-Aware Mask (MANDATORY Implementation with Toggle)
Add path if `enable_adaptive_mask == true`:
```
grad_len = max(length(grad), 1e-5)
// approximate pixel footprint scalar: fp = 1.0 (assume 1 world unit == 1 pixel) else compute from viewport
aa_width = clamp(iso / (grad_len + 1e-5) * 0.5, 0.75, 4.0)
mask = smoothstep(iso - aa_width, iso + aa_width, best_field)
```
Fallback to legacy `compute_mask` when flag is 0.
Keep deterministic (no derivatives). Provide inline doc.

---
### 9. AccumResult Radius Tracking (MANDATORY for adaptive SDF)
Extend `AccumResult` with:
```
approx_r: array<f32, K_MAX>
```
Populate whenever you add or update a cluster entry with that ball’s `scaled_r` (the radius after radius_scale * radius_multiplier). Use last-written value (good enough heuristic).
Backward compatibility: If not used (metadata mode off) no behavioral change except minor register pressure.

---
### 10. Early-Exit Guard (Secondary)
Inside accumulation: Accept an `allow_early_exit` boolean precomputed outside instead of recomputing gradient requirement each loop. Early-exit only when `allow_early_exit` true.

---
### 11. Dev Assertions (Secondary)
If a debug flag is available (e.g. `debug_view == 2`) output a magenta pixel when `tile.offset + tile.count` would exceed a new uniform limit (`metaballs.v4.w` repurposed? Not enough lanes). OPTIONAL placeholder comment only—actual uniform for total index length not yet present. Do NOT implement bounds read; just add a comment scaffolding.

---
### 12. Palette Future (Secondary Scaffold)
Add commented block describing how to swap `cluster_colors` to a 1D sampled texture (`@group(2) @binding(6) sampler/filter + texture_2d` or `texture_1d` once stabilized). Do NOT add binding now.

---
### 13. Code Modification Checklist (Execution Order)
You MUST apply modifications in this order:
1. Update `AccumResult` struct & initialization loops for `approx_r`.
2. Modify accumulation logic to store `scaled_r` in `approx_r[idx]`.
3. Precompute iso shift BEFORE accumulation if needed; adjust call arguments.
4. Pass new `allow_early_exit` flag into accumulation (rename parameter for clarity).
5. Implement adaptive mask path with toggle.
6. Replace metadata encoding block per Section 6.
7. Implement adaptive SDF scale using `approx_r`.
8. Refactor noise centering (both background & surface noise functions) per Section 7.
9. Update comments reflecting new semantics (remove outdated TODOs, add new TODO tags ONLY for deferred items: TEXTURE_PALETTE, STORAGE_METADATA_TEXTURE).
10. Add header constants block ensuring sync instructions with Rust (comment only; no code-gen tooling in shader).

---
### 14. Success Criteria
You MUST ensure:
- Visual output (non-metadata) difference: average per-channel absolute delta < 0.01 for frames where adaptive mask disabled and sn_mode additive unchanged.
- Metadata mode: R channel remains monotonic with true SDF proxy; change in scaling accepted if smooth & bounded.
- No out-of-bounds buffer reads introduced.
- WGSL validates (no syntax errors) and compiles under existing pipeline.
- Branch complexity increase minimal (no nested switches added beyond current count).

---
### 15. Testing Guidance
Describe (in comments or separate doc) a CPU-side test harness to:
1. Render reference frame before changes (screenshot or GPU readback).
2. After refactor, render with `enable_adaptive_mask = 0` to compare diff (tolerance 0.01).
3. Enable adaptive mask and verify thinner edges still anti-aliased.
4. Toggle `metadata_v2_enabled` and confirm channel mapping (B/A encode hi/lo 8 bits) via readback of known cluster id > 255.
5. Force `sn_mode == 1` with iso shift and confirm silhouettes identical to variant with early-exit disabled globally.

---
### 16. Documentation Updates (MANDATORY)
Insert concise block comment at top summarizing: version/date, list of implemented improvements (EarlyExitIsoFix, MetadataV2, AdaptiveSDF, AdaptiveMask, NoiseCenteringFix). Remove stale TODO lines replaced by executed tasks.

---
### 17. Example Snippet (Illustrative Only – DO NOT Paste Verbosely)
```wgsl
// Before accumulation:
var effective_iso = iso;
var local_enable_early_exit = (metaballs.v4.x > 0.5) && !(sn_enabled && sn_mode == 0u) && !needs_gradient;
if (sn_enabled && sn_mode == 1u && surface_noise.v0.x > 0.00001) {
    let delta = surface_noise_scalar(p, time_seconds); // unbiased after centering fix
    effective_iso = iso + (delta - surface_noise.v0.x * 0.5);
}
var acc = accumulate_clusters_tile(p, tile, ..., local_enable_early_exit, effective_iso);
```

---
### 18. Deliverables
You WILL provide:
1. Updated WGSL file with all mandatory changes.
2. Inline comments documenting each change section.
3. No extraneous whitespace churn; unrelated code unchanged.

---
### 19. Prohibited
You MUST NOT:
- Introduce dynamic loops over `MAX_CLUSTERS` outside existing accumulation path.
- Replace existing noise with texture lookups (future task).
- Break binding interface or uniform struct memory layout (except for semantic reinterpretation of reserved lanes documented here).

---
### 20. Final Review Checklist (Self-Verify BEFORE Output)
Tick internally (do not output the word DONE, just ensure compliance):
[] Early-exit iso shift fixed
[] Additive mode early-exit disabled
[] Adaptive mask toggle path present
[] Metadata V2 encoding clarified
[] Adaptive SDF scale implemented with fallback
[] Surface & edge noise centering fix applied
[] AccumResult radius captured
[] Comments updated & version header added
[] No function signature breaks except accumulator param rename
[] Shader compiles (syntactically valid WGSL)

---
### 21. Style & Commenting
You WILL:
- Keep line width reasonable (< 120 chars).
- Use precise, action-oriented comments (avoid speculative language except in TODO tags).
- Prefix new TODOs with category (e.g., `// TODO(TEXTURE_PALETTE): ...`).

---
### 22. Execution
Perform the refactor now, adhering strictly to the ordered checklist (Section 13). Produce only the updated WGSL source as output (no narrative) when completing this prompt in an automated context.

---
### 23. Future (Informational Only — Do NOT Implement Now)
- TEXTURE_PALETTE: Move `cluster_colors` to sampled 1D texture.
- STORAGE_METADATA_TEXTURE: Separate metadata write path for richer picking structure.
- CLICKABILITY_FLAGS: Bit-pack cluster + flags into dedicated u32 in storage buffer.

---
### 24. Acceptance
Refactor is acceptable when mandatory goals met, optional items either implemented or left with explicit TODO tags, and tests in Section 15 pass within tolerance.

<!-- End of Prompt -->
