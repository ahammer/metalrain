---
mode: agent
description: 'Generate and integrate a secondary metaballs shader with bevel lighting, drop shadow, solid background, and PageUp/PageDown mode cycling in Bevy 0.16.'
---

# Metaballs Secondary Shader (Bevel + Shadow) Prompt

You WILL implement a secondary visual style for existing metaballs in a Bevy 0.16 project using WGSL. You MUST keep the existing classic shader fully functional while adding a new selectable mode featuring: (1) solid (opaque) neutral grey background, (2) flat color fills with a Photoshop-style bevel (45° light from top-left), (3) soft drop shadow beneath the metaball blobs, (4) PageUp / PageDown cycling of metaball render modes, (5) automatic disabling (hiding) of the existing background plugin/entities when the bevel mode is active, restoring them when not, and (6) a modular design permitting future additional styles without broad refactors.

## Research-Based Best Practices (Bevy 0.16 + WGSL)
You MUST follow these summarized best practices:
1. Keep Material2d implementations minimal and data-driven; avoid branching shaders per frame where a simple mode uniform or distinct material type suffices.
2. Reuse existing uniform layout (`MetaballsUniform`) to avoid pipeline churn; add a small `u32` style/mode field in an added `v3` vec4 (padding keeps 16-byte alignment) OR create a second material asset type if shader path must differ.
3. Prefer a separate WGSL file for the bevel variant for clarity (easier iteration) while still sharing accumulation code (duplicate with TODO comment to unify later if needed).
4. For bevel/emboss: use precomputed analytic gradient (already available from the field) and a normalized light direction (e.g., `normalize(vec3(-0.707, 0.707, 0.5))`) to generate a faux 3D normal (z reconstructed from gradient magnitude) for a single-step Lambert + optional rim.
5. For soft drop shadow in a single pass, offset the sample point along a 2D shadow vector BEFORE iso test (e.g., `shadow_offset = vec2(6.0, -6.0)` in world px). Accumulate a binary (or smoothly faded) mask, blur cheaply via two–three taps (cheap box) or distance-based falloff.
6. Keep AA band similar to original (analytic based on gradient) so silhouette quality remains high.
7. Background disable should NOT remove plugin types globally; simply set `Visibility::Hidden` (or despawn) for background quad entities when entering bevel mode; re-show when returning to classic.
8. Use a resource enum for modes; implement clean key handling with `Input<KeyCode>` OR integrate with existing input map (prefer consistent approach used for iso tweak). PageUp increases mode index, PageDown decreases (wrap-around).
9. Provide clear, testable success criteria & logging (info! lines) when mode changes.

## Definitions & Targets
You MUST implement:
1. New WGSL shader file: `assets/shaders/metaballs_bevel.wgsl`.
2. New material asset OR reuse existing with a mode uniform (choose one approach & stay consistent). Recommended: introduce `MetaballsBevelMaterial` (simplifies swapping fragment shader path without conditional logic). Keep `MetaballsMaterial` untouched.
3. Resource enum:
```rust
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballRenderMode { Classic, Bevel }
impl MetaballRenderMode { pub const ALL: [Self; 2] = [Self::Classic, Self::Bevel]; }
#[derive(Resource, Debug)] pub struct MetaballMode { pub idx: usize }
```
4. Mode cycling system triggered by PageUp / PageDown (key detection) that updates `MetaballMode.idx` (wraps) and logs the new mode.
5. A system that reacts to mode changes and:
   - Ensures only one metaball quad entity is Visible (classic vs bevel).
   - Hides background quad(s) (component `BackgroundQuad`) when in Bevel mode; restores on Classic.
6. Spawn *both* quads (classic + bevel) at startup with appropriate Z ordering (classic retains current, bevel same Z). Only one visible at a time.
7. The bevel shader must produce fully opaque output (alpha=1) with a neutral grey background: do not discard; fill background first.
8. Provide a shadow layer inside same fragment: render shadow BEFORE main shape fill by evaluating field at `p + shadow_offset`; composite using `max` or ordered blend into a `color` accumulator.
9. Keep performance reasonable: limit extra loops; reuse accumulation for primary shape, run a trimmed pass for shadow (e.g., early exit after threshold or bounding radius heuristics). Document any O(N) passes clearly.
10. Ensure WASM embed path logic matches existing pattern (`include_str!` fallback) for the new shader.

## Detailed Implementation Steps
You MUST execute or produce code for each step in order:
1. Create `metaballs_bevel.wgsl`:
   - Copy accumulation constants & uniform struct layout from `metaballs.wgsl`.
   - Add solid background fill: start `var out_col = vec3<f32>(0.42);` (neutral mid-grey) and final alpha 1.0.
   - Perform field accumulation (same as classic) to get dominant cluster base color.
   - Compute gradient-based normal: `let n = normalize(vec3(grad.x, grad.y, normal_z_scale * 0.75));` (tune scalar) then lighting: `diff = clamp(dot(n, light_dir), 0.0, 1.0);` Add subtle ambient 0.35.
   - Bevel: raise contrast around iso by applying a curve `bevel_intensity = pow(diff, 0.9)` and a secondary highlight `spec = pow(max(dot(reflect(-light_dir, n), view_dir), 0.0), 24.0)*0.35`.
   - Shadow: define `let shadow_vec = vec2<f32>(6.0, -6.0);` accumulate a second field at `p - shadow_vec`; if above iso, darken background via overlay: `out_col = mix(out_col, out_col * 0.35, shadow_mask)` with a soft fade (use gradient magnitude or distance field to edge for smoothing).
   - Edge AA: keep one-pixel band mask logic; but final color remains opaque (alpha=1). For pixels outside iso, do NOT discard; they just stay as background (avoid holes).
2. Add new material struct `MetaballsBevelMaterial` (parallel to existing) implementing `Material2d`; reference the new shader path.
3. Extend plugin: spawn both quads:
   - Tag classic with `MetaballsClassicQuad` and new one with `MetaballsBevelQuad`.
   - Insert `Visibility::Hidden` on the bevel quad initially.
4. Add new systems:
   - `cycle_metaball_mode` (runs in Update): reads `Input<KeyCode>` for `PageUp` / `PageDown` (or integrate with input map if present—choose consistent method, document).
   - `apply_metaball_mode` (after cycling): toggles visibilities on quad entities and hides/shows background quad(s).
5. Background Integration: query all `BackgroundQuad`; set `Visibility::Hidden` when mode == Bevel, else `Visibility::Visible`.
6. Logging: `info!(target: "metaballs", "Render mode switched to {mode:?}");`.
7. Tests / validation (manual assertions): run, press PageUp => bevel visible, background hidden, shading + shadow appear; PageDown cycles back.
8. Add documentation comment blocks at top of new shader summarizing bevel & shadow pipeline for maintainers.
9. Keep existing `MetaballsParams` reuse (normal_z_scale & iso) so UI adjustments still influence bevel shading.
10. Ensure constant reuse: unify `MAX_BALLS`, `MAX_CLUSTERS`; do NOT duplicate variable numeric constants; copy or reference identical values for consistency.

## Shader Pseudocode Snippet (Bevel Core)
```wgsl
// inside fragment after accumulation
let light_dir = normalize(vec3<f32>(-0.707, 0.707, 0.5));
let grad2 = grad; // 2D gradient
let n = normalize(vec3<f32>(-grad2.x, -grad2.y, normal_z_scale));
let diff = clamp(dot(n, light_dir), 0.0, 1.0);
let ambient = 0.35;
let base_lit = ambient + diff * 0.75;
let spec = pow(max(dot(reflect(-light_dir, n), vec3<f32>(0.0,0.0,1.0)), 0.0), 24.0) * 0.35;
let bevel_col = base_col * base_lit + spec;
out_col = mix(out_col, bevel_col, mask); // blend into background only where metaball exists
```

## Mode Cycling Example Code Snippet
```rust
fn cycle_metaball_mode(mut mode: ResMut<MetaballMode>, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::PageUp) { mode.idx = (mode.idx + 1) % MetaballRenderMode::ALL.len(); }
    if keys.just_pressed(KeyCode::PageDown) { mode.idx = (mode.idx + MetaballRenderMode::ALL.len() - 1) % MetaballRenderMode::ALL.len(); }
}
```

## Success Criteria
You MUST satisfy ALL:
1. Two distinct modes toggle via PageUp/PageDown (wrap-around). 
2. Bevel mode draws opaque frame (no transparency), neutral grey background visible.
3. Classic mode unchanged visually & performance.
4. Background grid hidden only in bevel mode, re-shown when leaving.
5. Drop shadow offset consistent and soft (fade, not hard silhouette).
6. No discarded pixels outside metaballs in bevel mode (avoids blending artifacts over previously rendered passes).
7. Logging confirms each mode change.
8. WASM build path includes the new shader (mirrors existing embed pattern).
9. No panics or warnings during runtime due to missing bindings or layout mismatch.
10. Code passes `cargo build` (Bevy 0.16) without additional feature flags beyond existing config.

## Output Requirements
When executing this prompt you WILL:
1. Create the WGSL shader file with full code (no placeholders) at `assets/shaders/metaballs_bevel.wgsl`.
2. Modify Rust source (plugin + materials) with minimal, surgical changes preserving style.
3. Add new material & systems and ensure registration order is documented.
4. Add concise inline comments explaining WHY (bevel math rationale, shadow trade-offs) not WHAT.
5. Provide a brief test run summary (manual verification plan) within the output.
6. List every file changed with a one-line justification.

## Constraints
You MUST NOT:
1. Break or rename existing classic shader assets.
2. Introduce global state where a resource suffices.
3. Use magic numbers without a short comment (especially shadow offset, ambient value, spec power).
4. Over-engineer (no trait objects or dyn dispatch for just two modes yet—keep extensible but simple).
5. Remove existing AA smoothing band.

## Extensibility Guidance
Design so adding a third mode only requires: a new WGSL file (or branch), include in enum + ALL array, spawn new quad (or reuse single quad with mode uniform), and small visibility logic extension.

## Final Delivery Format
Your response WILL include:
1. Implementation diff blocks for all changed Rust & new WGSL files.
2. Build success confirmation.
3. Manual test checklist outcomes.
4. Notes on future improvements (optional blur pass separation, uniform-driven single-shader polymorphism).

Begin now.
