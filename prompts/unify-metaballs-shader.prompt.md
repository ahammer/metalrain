# Unified Metaballs Shader Refactor Prompt

## Purpose
You WILL refactor existing metaball rendering into ONE reusable WGSL shader supporting three modes (Classic transparent, Bevel gray, Bevel noise background) while eliminating duplicated logic and preserving visual parity & performance.

## Background
Current state:
- `metaballs.wgsl`: classic transparent metaballs.
- `metaballs_bevel.wgsl`: bevel + shadow with opaque gray background.
Both duplicate: uniform struct, vertex passthrough, field accumulation (Wyvill kernel), cluster selection, AA edge logic. A third desired variant (bevel + animated procedural noise background) would further amplify duplication if added naïvely.

WGSL lacks textual `#include`; reuse is achieved by factoring shared code into functions within a single shader and branching cheaply on a mode uniform.

## Objectives
You WILL:
1. Introduce one file `assets/shaders/metaballs_unified.wgsl` that implements all three visual modes.
2. Reuse existing uniform layout; DO NOT modify struct size/alignment.
3. Repurpose uniform fields:
   - `v1.y` -> render_mode (float encoded int: 0=Classic, 1=BevelGray, 2=BevelNoise)
   - `v2.z` -> time_seconds (for noise animation)
   - Preserve `v1.w` as debug_view (grayscale field viewer).
4. Factor duplicated logic into WGSL helper functions (accumulation, dominant selection, bevel lighting, AA mask, noise background).
5. Maintain identical iso threshold, AA band, and bevel highlight output vs existing bevel shader.
6. Add animated, subtle, high-contrast-safe noise background for mode 2 (value noise or lightweight hash-based pseudo-Perlin) without textures.
7. Keep per-fragment overhead minimal (branch occurs AFTER heavy accumulation) ensuring <2% perf delta from current bevel mode.
8. Provide graceful fallback when ball_count=0 (classic: transparent discard, bevel modes: show background).
9. Preserve debug_view grayscale output across modes, overriding normal display when enabled.
10. Update Rust side to use single material & mode uniform OR (interim) add mode uniform while leaving legacy materials until verification.

## Non-Goals
- No change to MAX_BALLS / MAX_CLUSTERS constants.
- No introduction of storage buffers, samplers, or textures.
- No introduction of additional bind groups.

## Steps
1. Create `assets/shaders/metaballs_unified.wgsl`:
   - Copy uniform struct & vertex stage exactly (add comments documenting repurposed fields).
   - Implement helper functions:
     - `hash2`, `fade`, `value_noise`, `background_noise_color` (two-octave value noise + palette blending navy→teal→warm accent; ensure cluster color contrast).
     - `accumulate_clusters(p)` implementing K_MAX=12 accumulation.
     - `select_dominant(used, k_field[])`.
     - `compute_mask(best_field, iso, grad, p)` (AA band identical to existing logic).
     - `bevel_lighting(base_col, grad, normal_z_scale)` returning lit color.
     - Optional `apply_shadow(base_bg, best_field, iso)` (reuse simple offset heuristic) guarded by const bool.
   - Fragment flow:
     - Load uniform values; compute `ball_count`, `mode`, `time`, etc.
     - If ball_count==0 -> Early background path (classic: discard; others: return background color opaque).
     - Accumulate clusters once.
     - If used==0 -> same fallback.
     - Select dominant cluster index & derive base cluster color.
     - Compute gradient, mask.
     - If debug_view==1 return grayscale field.
     - `switch(mode)` style branching:
       - 0 Classic: transparent output (`discard` outside iso band; `vec4(color, mask)` inside).
       - 1 BevelGray: background gray (0.42) + optional shadow + bevel lit color mixed by mask, alpha=1.
       - 2 BevelNoise: animated noise background (from `background_noise_color`) + (optional) shadow + bevel lit color mixed by mask, alpha=1.
2. Rust Refactor (initial minimal integration):
   - Add enum variant list: `Classic`, `BevelGray`, `BevelNoise` (rename existing `Bevel` to `BevelGray`).
   - Introduce new material `MetaballsUnifiedMaterial` (copy of existing uniform struct) referencing unified shader for both vertex & fragment.
   - Replace dual quads & materials with one (preferred) OR keep old for safe comparison (temporary). If keeping old during transition, ensure cycling sets `v1.y` on unified material only when that mode is active.
   - Update update system to write:
     - `render_mode` into `uniform.v1.y` as f32.
     - `time_seconds` into `uniform.v2.z` each frame using `Time::elapsed_seconds_f64() as f32`.
3. Mode cycling logic: PageUp/PageDown increments index & updates `render_mode` (stop toggling visibility between separate quads when unified path fully adopted).
4. Remove background quad visibility suppression for bevel once unified approach is final (Classic continues to allow external background; bevel modes supply their own opaque backdrop).
5. Validate visuals & performance (manual quick check; optional frame time logging).
6. After validation (Phase 2 optional): delete legacy shaders & materials, clean up code, update docs.

## Noise Background Specification
- Domain scale: `q = p * 0.004 + vec2(time*0.03, time*0.02)`.
- Two value noise octaves: n = 0.65 * n1 + 0.35 * n2 (clamped).
- Palette blending:
  - c1 = (0.05, 0.08, 0.15)
  - c2 = (0.04, 0.35, 0.45)
  - c3 = (0.85, 0.65, 0.30)
  - mid = smoothstep(0, 0.6, n)
  - hi  = smoothstep(0.55, 1.0, n)
  - base = mix(c1, c2, mid)
  - final = mix(base, c3, hi * 0.35)
  - Optional overall darken ~0.1 to keep high contrast with bright cluster colors.

## Performance Guidance
- Keep noise ALU under ~40 ops; no loops beyond existing ball accumulation.
- Branch only AFTER accumulation.
- Classic path may skip bevel lighting math if mask ≤0 early.
- Avoid extra allocations or dynamic indexing outside existing fixed arrays.

## Debug Support
- `debug_view==1` returns scalar field grayscale (dominant cluster field / iso) independent of mode (still opaque alpha=1 for visibility).

## Validation Checklist
- [ ] Shader compiles (no warnings) & loads on native + WASM.
- [ ] Classic mode visually unchanged vs old classic.
- [ ] BevelGray mode visually matches old bevel (edge smoothness, lighting intensity, shadow similarity).
- [ ] BevelNoise mode shows animated background; metaballs remain readable & bevel highlight intact.
- [ ] PageUp/PageDown cycles modes (wrap-around) without replacing pipeline.
- [ ] Time uniform increments and animates noise smoothly (no flicker / banding).
- [ ] Performance within target (<2% diff) relative to current bevel mode.
- [ ] Transparency only present in Classic mode.
- [ ] Debug view works in all modes.

## Accessibility & Contrast
- Ensure noise palette mid/hi luminance does not converge on cluster colors (test with several palette entries). Adjust final mix scaling if necessary.

## Deliverables
- New WGSL file + unified material implementation.
- Enum & mode cycling updates.
- Time update logic.
- (Optional) Removal of legacy shaders in a follow-up commit after validation.
- Developer note / README snippet summarizing mode semantics & uniform field repurposing.

## Future TODO (Comment in Code)
- Extract common WGSL to procedural macro or build step if variant count grows.
- Add configurable noise palette parameters via extra uniform fields (requires struct expansion + versioning).
- Consider domain-warped third octave if performance headroom allows.

```
