---
mode: agent
description: 'Implement dual-axis metaball rendering (independent foreground & background modes) with aggressive cleanup: remove legacy single-mode & separate classic/bevel materials.'
---

# Metaballs Dual-Axis Foreground / Background Modes Prompt

## Purpose
You WILL replace the existing single (monolithic) metaball rendering mode with TWO orthogonal, *independently composable* mode dimensions (no backward compatibility path retained):
1. **Foreground mode** – how the metaball surfaces themselves are shaded (lighting, outlines, transparency strategy).
2. **Background mode** – what appears behind the metaballs (external scene, solid fill, procedural noise, gradients, future reactive fields).

Independent key controls (must use `just_pressed` detection):
- **PageUp / PageDown**: cycle Background mode (wrap-around).
- **Home / End**: cycle Foreground mode (wrap-around).

Design goals (forward-only):
- Preserve visual parity for THREE legacy visual pairings (Classic, BevelGray, BevelNoise) via explicit new (foreground, background) pairs, then delete all legacy code.
- Provide clean separation so any foreground can pair with any background (Cartesian product) without shader duplication.
- Allow Background stage to *optionally* consume foreground-derived data (mask, gradient magnitude, best_field normalized, cluster index) for reactive effects (e.g., halo, ambient bleed) without re-running heavy accumulation.
- Maintain WGPU / Naga (Bevy) compliance: no pointer passing to arrays (avoid prior bug), uniform layout unchanged in size/alignment, loops statically bounded.
- Aggressively remove: old `MetaballRenderMode` enum, legacy classic/bevel material types & quads, and any fallback mapping logic.

## Current Baseline (Code Review Summary)
Source reviewed: `src/rendering/metaballs/metaballs.rs` & `assets/shaders/metaballs_unified.wgsl`.

Findings:
- Single shader already consolidates Classic (transparent), BevelGray, BevelNoise via `render_mode = v1.y` (float -> int cast) and background logic in fragment path.
- Uniform layout (`MetaballsData` / `MetaballsUniform`) is: `v0=(ball_count, cluster_color_count, radius_scale, iso)`, `v1=(normal_z_scale, render_mode, radius_multiplier, debug_view)`, `v2=(win_w, win_h, time_seconds, reserved)`, followed by dense arrays.
- `v1.z` currently carries `radius_multiplier`; `v2.w` is unused (reserved) making it a candidate for relocating `radius_multiplier` without changing total struct size.
- Fragment logic already *after* accumulation chooses background & shading -> ideal insertion point for decoupled foreground/background branching.
- Previous bug mitigations: pointer-based helper removed (dominant selection inlined) — MUST preserve (avoid pointer params to satisfy Naga / SPIR-V backend stability).

Constraint: We **must not** alter struct size / alignment (WGPU compliance). We repurpose the previously unused lane for `radius_multiplier` and permanently redefine semantics (no runtime detection of legacy layout).

## Objective
You WILL implement two orthogonal indices (foreground & background) and DELETE all legacy single-mode logic. Any references to `render_mode` in code must be removed/updated in the same change.

### Foreground Shading Modes (initial set)
1. **ClassicBlend** – original transparent metaballs (alpha = mask).
2. **Bevel** – existing bevel lighting (opaque inside blob when paired with opaque background; can still be composited over transparent background).
3. **OutlineGlow** (NEW) – optional; thin rim + soft interior emission (can initially alias Classic to reduce scope; architecture must allow enabling later without refactor).

### Background Modes (initial set)
1. **ExternalBackground** – transparent outside blobs (relies on existing background quad entity).
2. **SolidGray** – neutral ~0.42; parity with BevelGray.
3. **ProceduralNoise** – reuse existing two-octave value noise path (no new loops – keep under current ALU budget).
4. **VerticalGradient** (NEW) – lightweight y-based gradient (single lerp + smoothstep) enabling quick visual variety.

(You MAY reduce the initial list if implementation complexity is constrained, but MUST architect for two orthogonal enums.)

## Uniform & Data Layout (Repurposing Without Structural Change)
WGSL struct (unchanged in binary layout):
```
v0: (ball_count, cluster_color_count, radius_scale, iso)
v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
```

Notes:
- Foreground & background mode indices are 0-based integer values written as exact IEEE f32 (no fractional values allowed).
- `radius_multiplier` permanently relocated to `v2.w`; old usage of `v1.z` is invalid and must be purged.

Rationale / WGPU compliance: No size/alignment changes; only semantics changed.

## Rust-Side Changes
1. Remove `MetaballRenderMode` (delete enum & associated cycling systems) and introduce two resources:
```rust
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballForegroundMode { ClassicBlend, Bevel, OutlineGlow }
impl MetaballForegroundMode { pub const ALL: [Self; 3] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow]; }

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballBackgroundMode { ExternalBackground, SolidGray, ProceduralNoise, VerticalGradient }
impl MetaballBackgroundMode { pub const ALL: [Self; 4] = [Self::ExternalBackground, Self::SolidGray, Self::ProceduralNoise, Self::VerticalGradient]; }

#[derive(Resource, Debug, Default)] pub struct MetaballForeground { pub idx: usize } // Home/End
#[derive(Resource, Debug, Default)] pub struct MetaballBackground { pub idx: usize } // PageUp/PageDown
```
2. Key handling systems (order independent – small systems grouped in one tuple for cache locality):
   - `cycle_background_mode`: PageUp=next, PageDown=prev.
   - `cycle_foreground_mode`: End=next, Home=prev (consistent directional semantics: *forward* keys on right side of cluster).
3. A system `apply_modes` updates visibility of background quad(s): Hidden when background != ExternalBackground. Foreground uses only the unified quad. DELETE legacy classic / bevel material types & quads outright in this change.
4. Update `update_metaballs_unified_material`:
   - Write `foreground_mode` to `v1.y`.
   - Write `background_mode` to `v1.z`.
   - Write `radius_multiplier` to `v2.w` ONLY.
5. Maintain existing iso/normal_z_scale behavior & logging for param tweaks.

## WGSL Shader Refactor
You WILL modify (not rename) `metaballs_unified.wgsl` in-place to minimize asset churn:
1. Keep early heavy accumulation EXACTLY as-is (hot path). Post-accumulation: read `foreground_mode` & `background_mode` directly; no legacy detection. Branch in *two* compact `switch` statements (background first, then foreground) using local vars `bg_col`, `bg_alpha`, then apply foreground overlay. Avoid nested large if/else blocks.
2. Foreground rendering functions (structure for composability):
   Implement helpers (pure functions – no side effects):
   - `fn fg_classic(base_col, mask) -> (vec3<f32>, f32)`
   - `fn fg_bevel(base_col, grad, mask, normal_z_scale, bg_col) -> (vec3<f32>, f32)` (may ignore `bg_col` but presence allows future reflected light blending)
   - `fn fg_outline_glow(base_col, best_field, iso, mask, grad) -> (vec3<f32>, f32)` (optional or alias classic initially)
   These return premultiplied or straight RGBA components; pick one convention (recommend *straight* then final pack).
   - ClassicBlend: identical to current classic path (discard or alpha=0 outside iso; inside iso output cluster color with AA mask alpha). Keep transparency.
   - Bevel: identical bevel lighting path (reuse existing code). Output alpha=1 (opaque) but final alpha may still reflect mask only inside field; if outside iso: base background unaffected.
   - OutlineGlow: (option) compute edge_factor = smoothstep(iso - aa, iso, field) * (1.0 - smoothstep(iso, iso + aa, field)) and produce color = cluster_color * 0.25 + vec3(0.9,0.9,1.2)*pow(edge_factor, 0.5); alpha=edge_factor. (Simplified; keep cheap.)
3. Background rendering helpers (executed *before* foreground for clarity):
   - `bg_external()` -> (vec3<f32>, f32=0)
   - `bg_solid_gray()` -> (vec3<f32>(0.42), 1)
   - `bg_noise(p, time)` -> existing noise path reused
   - `bg_vertical(p, viewport_h)` -> gradient function (document color stops)
   Expose gradient & noise functions so *foreground* variants can sample background color if needed for advanced blending (future halos).
   - ExternalBackground: Do nothing (keep transparent outside iso). Only ClassicBlend should rely on this for alpha blending; other foreground modes + external background combination MUST NOT fill underlying pixels.
   - SolidGray: Fill base_color = vec3(0.42) ; alpha=1 always.
   - ProceduralNoise: Reuse existing noise function (value / hash). Add subtle contrast: final_noise = mix(noise_color, noise_color * 0.9, 0.1).
   - VerticalGradient: t = clamp((world_pos.y / viewport_h)*0.5 + 0.5,0,1); base_color = mix(vec3(0.05,0.08,0.15), vec3(0.04,0.35,0.45), smoothstep(0.0,1.0,t));
4. Composition rules (foreground consumes background info):
   - Background first -> `(bg_col, bg_a)`.
   - Foreground returns `(fg_col, fg_a)`.
   - If `bg_a == 0` (External) use standard alpha of foreground only.
   - Else composite: `out_col = mix(bg_col, fg_col, fg_a)`; `out_a = max(bg_a, fg_a)` (opaque backgrounds guarantee out_a=1).
   - ClassicBlend over opaque background still produces consistent silhouette (mask-driven) – no discard necessary; Classic + External retains discard for zero mask.
   - Start with `bg_col` & `bg_alpha` depending on background mode.
   - Apply foreground based on mode:
     - ClassicBlend: if background mode is NOT ExternalBackground and you produced an opaque background, then blend cluster color with `alpha = mask` over `bg_col` (premultiplied or straight) and set output alpha = (bg_alpha==1 ? 1 : mask).
     - Bevel / OutlineGlow: ensure result is opaque if background produced opacity; if ExternalBackground chosen, you still want to keep transparency outside mask.
5. Debug view: Override *before* final composition, using alpha = (bg_a > 0 ? bg_a : 1). Provide scalar field grayscale of `best_field / iso` (consistent diagnostic independent of compositing).
6. Keep branching cost low: use small integer `switch` blocks; avoid nested heavy logic duplication; reuse computed gradient & field.
7. Update top-of-file comments: clearly document dual-mode, migration heuristic, and note any future TODO (e.g., moving to storage buffer for >1024 balls not in current scope).

### Foreground-to-Background Data Sharing (Reactive Backgrounds)
Add internal struct after accumulation:
```
struct ForegroundContext { best_field: f32, mask: f32, grad: vec2<f32>, iso: f32, cluster_index: u32, cluster_color: vec3<f32> };
```
Return / pass this to background reactive helpers (for future features like glow or field-dependent noise modulation) **without** re-running accumulation. For now, background modes ignore it except potential future vertical gradient tint influenced by cluster color (commented TODO).

## Performance Constraints
You MUST:
- Add ZERO additional loops in hot path.
- Avoid recomputing noise for pixels fully covered by ClassicBlend early discard (branch early).
- Keep OutlineGlow optional; its math is minimal (~10 ALU ops). If disabled via feature flag, keep placeholder branch returning ClassicBlend behavior.
- Validate <3% frame time delta vs previous unified shader in BevelNoise path (manual observation acceptable; instrument optional).

## Input Handling & Accessibility
- Keys must be resilient: only react on `just_pressed`.
- Provide log lines with clear targets (`metaballs`) e.g., `info!(target="metaballs", "Foreground mode -> {:?}", fg.current())`.
- Document controls in README snippet (follow-up PR).
- Do NOT remove iso tweak inputs (MetaballIsoInc/Dec).

## Data Validation
- Clamp foreground/background indices with modulo operations using `ALL.len()`.
- Always write `radius_multiplier` to `v2.w` each update.
- Enforce indices < enum length before writing to uniform; if an out-of-range value appears (should not), treat as 0 in Rust side (fail-fast via log warning).

## Success Criteria
You MUST satisfy ALL:
1. PageUp/PageDown cycles ONLY background modes; Home/End cycles ONLY foreground modes (wrap-around) with independent state.
2. ExternalBackground + ClassicBlend pair reproduces previous Classic look (visual parity acceptable within small perceptual tolerance).
3. Bevel + SolidGray reproduces previous BevelGray look; Bevel + ProceduralNoise reproduces previous BevelNoise look.
4. Switching background does NOT alter bevel lighting results; switching foreground does NOT alter background noise/gradient pattern (opacity interactions aside).
5. Debug view displays scalar field grayscale regardless of both modes.
6. Transparent output outside blobs ONLY when (foreground=ClassicBlend AND background=ExternalBackground) (single well-defined case).
7. No pipeline recompilations per key press (uniform update only).
8. Shader compiles & runs on native + WASM (struct unchanged size / alignment).
9. Legacy classic/bevel materials & mode enum fully removed from codebase in same PR.
10. Code passes `cargo build` and `cargo clippy` without new warnings relevant to touched code.

## Deliverables
You WILL output / implement:
1. Updated WGSL unified shader implementing dual-axis logic & documented uniform semantics.
2. Rust resource enums, cycling systems, update system modifications.
3. Immediate removal of deprecated separate material paths (classic / bevel) in same change.
4. Logging on each mode change plus a startup summary of initial modes.
5. Inline WHY comments for non-obvious calculations (e.g., outline glow shaping, noise domain scale).
6. Manual test checklist results (list each success criterion with PASS/FAIL) in output.

## Manual Test Plan (Include in Output)
- Launch app: observe initial mode (log). Should be ClassicBlend + ExternalBackground.
- Press Home / End: foreground cycles; background static.
- Press PageUp / PageDown: background cycles; foreground static.
- Combine every foreground with every background quickly: no panic, stable visuals.
- Debug view toggle (if existing) reflects field irrespective of combinations.
- Performance observation: no stutter on rapid cycling.

## Future Extensibility Notes (Comment in Code)
- Consider packing modes into a single `u32` bitfield if uniform slots become scarce.
- Potential to expose modes via config file and hot-reload.
- Optionally factor background generation into separate fullscreen pass if complexity grows (avoid shader bloat).

## Constraints / MUST NOT
You MUST NOT:
- Increase uniform struct size.
- Introduce storage buffers or textures.
- Duplicate accumulation loops.
- Hardcode magic numbers without a clarifying comment (noise scale, gradient colors).
- Break existing input tweak behavior.

## Output Format Requirements
When executing this prompt you WILL provide:
1. Diff(s) for Rust & WGSL files edited/added.
2. Build verification summary.
3. Manual test checklist with PASS/FAIL markers.
4. Follow-up TODO list (if any parity gaps remain).

## WGPU / Naga Compliance Checklist (You MUST honor)
- Uniform struct size unchanged (no added members – only semantic remap).
- No pointer passing to arrays (retain inlined dominant selection pattern).
- All loops have static upper bounds (`ball_count` limited by MAX_BALLS, cluster accumulation constrained by K_MAX early exit semantics).
- Avoid dynamic indexing outside bounds; guard with `ball_count` & `cluster_color_count` checks (already present).
- No recursion; no unsupported atomics or barriers in fragment stage.
- Keep texture-free design (value noise only uses arithmetic) to avoid extra bindings.
- Maintain 16-byte alignment: all `vec4<f32>` groups unchanged.

Proceed to implement.
