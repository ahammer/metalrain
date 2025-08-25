---
mode: agent
description: 'Implement dual-axis metaball rendering (independent foreground & background modes) with aggressive cleanup: remove legacy single-mode & separate classic/bevel materials (ExternalBackground path already removed).'
---

# Metaballs Dual-Axis Foreground / Background Modes Prompt (Updated – external background quad removed)

## Purpose
Replace the former monolithic metaball rendering mode with TWO orthogonal, *independently composable* mode dimensions:
1. **Foreground mode** – how the metaball surfaces are shaded (lighting, outlines, transparency strategy).
2. **Background mode** – what appears behind the metaballs (solid fill, procedural noise, gradients, future reactive fields) — all generated INTERNALLY in the unified shader (no separate fullscreen background pass remains).

Independent key controls (must use `just_pressed` detection):
- **PageUp / PageDown**: cycle Background mode (wrap-around).
- **Home / End**: cycle Foreground mode (wrap-around).

Design goals:
- Preserve visual parity for three legacy pairings (Classic, BevelGray, BevelNoise) via explicit (foreground, background) combinations — remove all legacy material & quad duplication.
- Any foreground can pair with any background (Cartesian product) without shader duplication.
- Allow Background stage to optionally consume foreground-derived data (mask, gradient magnitude, normalized field, cluster index) for reactive effects (e.g., halo, ambient bleed) without re-running heavy accumulation.
- Maintain WGPU / Naga compliance: uniform layout unchanged in size/alignment; loops statically bounded; no pointer passing.

## Current Baseline (Code Review Summary)
Reviewed: `src/rendering/metaballs/metaballs.rs`, `assets/shaders/metaballs_unified.wgsl`.

Findings (post-cleanup):
- Unified shader already handles Classic + Bevel variants and background coloration in one pass.
- Uniform layout (`MetaballsData` / `MetaballsUniform`):
  - `v0=(ball_count, cluster_color_count, radius_scale, iso)`
  - `v1=(normal_z_scale, foreground_mode, background_mode, debug_view)`
  - `v2=(win_w, win_h, time_seconds, radius_multiplier)`
- `radius_multiplier` relocated to `v2.w` (semantic remap only; size/alignment unchanged).
- External background path & separate fullscreen quad have been removed; all backgrounds are internally opaque.

Constraint: MUST NOT alter struct size / alignment.

## Objective
Implement & maintain two orthogonal indices (foreground & background) with clean cycling systems and shader branching. Legacy single-mode logic and external background references must not reappear.

### Foreground Shading Modes (initial set)
1. **ClassicBlend** – transparent metaballs (alpha = mask).
2. **Bevel** – bevel lighting (opaque inside blob when paired with opaque background).
3. **OutlineGlow** – thin rim + soft emission (may initially alias Classic; architecture must allow later enhancement).

### Background Modes (current internal set)
1. **SolidGray** – neutral ~0.42 (parity with former BevelGray baseline).
2. **ProceduralNoise** – two-octave value noise (no added loops).
3. **VerticalGradient** – y-based gradient (single lerp + smoothstep).

(Architecture must accommodate future additions without new passes.)

## Uniform & Data Layout (Semantic Mapping)
```
v0: (ball_count, cluster_color_count, radius_scale, iso)
v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
```
- Mode indices written as integer-valued f32.
- `radius_multiplier` always written to `v2.w`.

## Rust-Side Structures (Reference Shape)
```rust
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode { ClassicBlend, Bevel, OutlineGlow }
impl MetaballForegroundMode { pub const ALL: [Self; 3] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow]; }

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballBackgroundMode { SolidGray, ProceduralNoise, VerticalGradient }
impl MetaballBackgroundMode { pub const ALL: [Self; 3] = [Self::SolidGray, Self::ProceduralNoise, Self::VerticalGradient]; }

#[derive(Resource, Debug, Default)] pub struct MetaballForeground { pub idx: usize } // Home/End
#[derive(Resource, Debug, Default)] pub struct MetaballBackground { pub idx: usize } // PageUp/PageDown
```
Cycling systems wrap indices (mod ALL.len()) and log changes.

## WGSL Shader Refactor (Key Points)
- Heavy accumulation code remains unchanged (hot path).
- Post-accumulation branching:
  - Background switch (sets `bg_col`, `bg_alpha`).
  - Foreground switch (computes `fg_col`, `fg_alpha`).
- Composition:
  - Opaque backgrounds: `out_col = mix(bg_col, fg_col, fg_alpha); out_alpha = 1.0`.
  - Classic over opaque: same rule; alpha forced opaque to simplify compositing (or optionally preserve mask—document choice).
- Helper functions (pure):
  - `fg_classic`
  - `fg_bevel`
  - `fg_outline_glow` (can alias classic initially)
  - `bg_solid_gray`, `bg_noise`, `bg_vertical`
- Debug override: grayscale of `best_field / iso` with stable alpha (1.0).

## Performance Constraints
- ZERO additional accumulation loops.
- OutlineGlow must remain cheap (< ~10 ALU ops beyond Classic).
- <3% frame time delta vs prior unified shader path (manual observation acceptable).
- No pipeline recompilation on mode changes (uniform updates only).

## Input Handling & Accessibility
- Use `just_pressed`.
- Log each mode transition with target `"metaballs"`.
- Preserve iso & normal_z_scale tweak controls.
- Future: document controls in README (follow-up PR).

## Data Validation
- Clamp indices via modulo before writing uniforms.
- Guard against out-of-range (log & coerce to 0).
- Always write `radius_multiplier` each update.

## Success Criteria
1. PageUp/PageDown cycle only background modes; Home/End cycle only foreground modes (wrap-around).
2. ClassicBlend + SolidGray approximates prior Classic over neutral background.
3. Bevel + SolidGray matches previous bevel-on-gray; Bevel + ProceduralNoise matches former bevel-on-noise look.
4. Changing background does not alter bevel lighting math; changing foreground does not alter noise/gradient pattern (except expected compositing).
5. Debug view shows scalar field grayscale independent of mode combination.
6. All backgrounds are opaque (no reliance on removed external pass).
7. No pipeline recompilations on mode presses.
8. Shader compiles & runs on native + WASM (layout unchanged).
9. Legacy external background path remains absent (no identifiers or code referencing removed quad).
10. `cargo build` + `cargo clippy` pass with no new warnings related to modes.

## Deliverables
1. Unified WGSL shader with dual-axis logic & documented uniform semantics.
2. Rust resource enums, cycling systems, uniform update adjustments.
3. Removal (already done) of deprecated material/quads documented in comments.
4. Logging on each mode change (plus startup summary).
5. Inline WHY comments for non-obvious math.
6. Manual test checklist results (PASS/FAIL per success criterion).

## Manual Test Plan
- Launch: observe log of initial modes (ClassicBlend + SolidGray).
- Home / End: foreground cycles (background static).
- PageUp / PageDown: background cycles (foreground static).
- Rapidly iterate all 3x3 combinations: stable visuals, no warnings.
- Toggle debug view: grayscale unaffected by mode pairing.
- Observe frame time while cycling quickly (no noticeable stutter).

## Future Extensibility Notes
- Possible bitfield packing of two mode indices if uniform lanes become scarce.
- Potential config-driven mode selection with hot-reload.
- If backgrounds proliferate, consider factoring out into a lightweight optional second pass (deferred decision).

## Constraints / MUST NOT
- Do not change uniform struct size or alignment.
- No storage buffers / textures introduced.
- No duplication of accumulation loops.
- Avoid unexplained magic numbers (document noise scale, gradient colors).
- Preserve existing input tweaks.

## WGPU / Naga Compliance Checklist
- Uniform struct unchanged in size/alignment (semantic remap only).
- No pointer passing to arrays; dominant selection stays inlined.
- All loops statically bounded (guard by counts).
- No recursion / unsupported atomics / barriers in fragment.
- Arithmetic-only noise (no extra bindings).
- 16-byte alignment preserved (vec4 grouping retained).

## Status Note
This prompt has been updated to remove all references to the deprecated external background quad and its enum variant. Any reintroduction must justify added complexity and performance cost.

Proceed with maintenance or further enhancements under these constraints.
