# Metaball Surface Noise Feature Prompt

## Objective
Add an organic, animated high‑frequency surface noise modulation to metaball borders (irregular outlines) that is **independently configurable** from the existing background procedural noise. The noise perturbs the *effective field / iso contour* to create wavy, living edges without materially impacting interior lighting or performance scalability.

## Constraints & Alignment
- Preserve existing uniform binary layout for `MetaballsData`; add **new dedicated uniform** block for surface noise (do NOT repack existing structs to avoid shader/asset cache invalidation).
- Follow repository conventions (see `Copilot Instructions (Ball Matcher)`): small focused additions, early exits for disabled features, config defaults + validation, GPU alignment (16‑byte), minimal per‑fragment cost.
- Maintain WASM path embedding pattern (add new uniform only; no extra shader file unless necessary).
- Avoid large performance regressions: O(1) additional noise evaluation per fragment when enabled; skip entirely when disabled or amplitude == 0.

## User‑Facing Config Additions (`game.ron`)
Add a new top‑level block (parallel to `noise`):
```
surface_noise: (
    enabled: true,
    mode: 0,              // 0 = field add, 1 = iso shift (see below)
    amp: 0.08,            // amplitude in field/iso units (~0.03 – 0.15 typical)
    base_scale: 0.008,    // inverse spatial scale (higher => finer detail)
    warp_amp: 0.3,        // optional domain warp (0 disables)
    warp_freq: 1.2,       // domain warp frequency multiplier
    speed_x: 0.20,        // animation velocities
    speed_y: 0.17,
    octaves: 4,           // 1..6, clamp if >6
    gain: 0.55,           // fBm amplitude decay
    lacunarity: 2.05,     // frequency growth
    contrast_pow: 1.10,   // post shaping
    ridged: false,        // ridged variant
),
```
Defaults should yield a subtle organic ripple. Document in sample config.

## Data Model Changes
In `core::config::config.rs`:
1. Define `SurfaceNoiseConfig` (serde default) mirroring above fields.
2. Add field `pub surface_noise: SurfaceNoiseConfig` to `GameConfig` + default.
3. Validation (startup warning accumulation pattern):
   - Clamp `amp` to `[0.0, 0.5]` (prevent extreme aliasing).
   - Clamp `octaves` to `[0,6]` (`0` = disabled / fast path if user sets) but prefer using `enabled=false`.
   - Ensure `base_scale > 0` (fallback to default if <= 0).

## GPU Uniform Additions
Extend `MetaballsUnifiedMaterial` with a new uniform:
```rust
#[repr(C, align(16))]
#[derive(Clone, Copy, ShaderType, Debug, Default)]
pub struct SurfaceNoiseParamsUniform {
    pub amp: f32,
    pub base_scale: f32,
    pub speed_x: f32,
    pub speed_y: f32,
    pub warp_amp: f32,
    pub warp_freq: f32,
    pub gain: f32,
    pub lacunarity: f32,
    pub contrast_pow: f32,
    pub octaves: u32,
    pub ridged: u32,      // 0|1
    pub mode: u32,        // 0=field add,1=iso shift
    pub enabled: u32,     // 0|1 early branch
    pub _pad0: u32,
    pub _pad1: u32,
}
```
Binding index: **`@group(2) @binding(2)`** (next sequential after existing background noise binding(1)). Add `#[uniform(2)] surface_noise: SurfaceNoiseParamsUniform` to the material.

## Shader (`metaballs_unified.wgsl`) Changes
1. Add new `struct SurfaceNoiseParams` matching Rust layout & declare:
```wgsl
@group(2) @binding(2)
var<uniform> surface_noise: SurfaceNoiseParams;
```
2. Implement scalar noise helper `surface_noise_scalar(p, time)` reusing existing `value_noise`, domain warp pattern, fBm (strip color palette). Keep max 6 octave loop with early `break`.
3. Integration point options:
   - **Mode 0 (field add)**: in fragment after computing `best_field`, before mask: `best_field_mod = best_field + surface_noise.amp * (n - 0.5);`
   - **Mode 1 (iso shift)**: compute `effective_iso = iso + surface_noise.amp * (n - 0.5);` and pass that to `compute_mask`.
4. Early exit branch:
```wgsl
if (surface_noise.enabled == 0u || surface_noise.amp <= 0.00001) { /* skip */ }
```
5. Keep derivative‐based AA intact (only change inputs). Avoid re‑evaluating noise more than once per fragment.
6. Debug gating: if `debug_view==1` maintain original scalar field visualization; optionally add sub‑mode later.

## CPU Update Path
In `update_metaballs_unified_material` system:
- Populate `surface_noise` uniform every frame from `cfg.surface_noise` (mirroring existing `noise` update). Convert bools / mode to `u32`.
- Reuse early bail on `!toggle.0`.

## Performance Considerations
- Single additional noise evaluation per pixel; with fBm (4 octaves default) still bounded. Provide guidance: Setting `octaves` >4 with bevel foreground increases GPU cost; note in docs.
- Domain warp is optional: if `warp_amp == 0` skip warp path.
- Branch on `enabled` uniform ensures zero overhead when disabled (compiler may DCE path).

## Testing / Validation
1. Start with default surface noise ON; verify borders show subtle animation (compare frame captures ~2s apart).
2. Toggle `enabled=false` -> borders return to current smooth shape (no color shifts).
3. Switch `mode` between 0 and 1: ensure visual difference (mode 1 should preserve interior field intensities, shifting threshold instead; mode 0 may slightly thicken / thin blobs locally).
4. Stress: Increase `amp` to 0.25 & `octaves=6` confirm still stable (no NaNs, no shimmering alias beyond acceptable). If artifacts: document recommended upper bounds.
5. WASM build: confirm new binding works (embedding path unchanged). Ensure binding order matches material definition.

## Potential Edge Cases & Mitigations
| Case | Issue | Mitigation |
|------|-------|------------|
| Amp too high | Jagged alias edges | Clamp & warn; doc recommended range |
| Octaves high | Perf drop | Clamp to 6; warn >4 |
| base_scale tiny | Large low‑freq wobble (not high freq) | Document expected usage; no hard clamp |
| mode=1 + large amp | Large contour shifts cause popping | Encourage lower amp when iso near extremes |

## Step‑By‑Step Implementation Plan
1. Add `SurfaceNoiseConfig` + defaults + integrate into `GameConfig`.
2. Extend validation collecting warnings (amp clamp, octaves clamp, base_scale fallback).
3. Update `game.ron` (and sample) adding `surface_noise` block.
4. Add `SurfaceNoiseParamsUniform` Rust struct; update `MetaballsUnifiedMaterial` with new uniform (binding 2) & default impl.
5. Modify `setup_metaballs` to initialize uniform from config.
6. Update `update_metaballs_unified_material` to refresh surface noise params each frame.
7. Modify WGSL:
   - Add uniform struct & binding.
   - Add scalar noise helper (reusing existing primitives).
   - Inject conditional perturbation before mask computation.
8. Rebuild & run; visually validate + capture performance (frame time) before/after using release build.
9. Adjust documentation: README (regenerate via existing script) & add config field description.
10. Optional: Add input mapping for runtime toggle (future enhancement) without config reload.

## Acceptance Criteria
- Configurable organic border ripple visible when enabled; disabled path produces exactly prior visuals (bitwise identical within floating tolerance).
- No crashes / panics; shader compiles native & WASM.
- Performance regression < ~5% at default settings at typical resolution.
- All new fields `#[serde(default)]`; absence in older configs does not break load.

## Stretch Ideas (Defer)
- Temporal stabilization via low‑pass filter on noise (reduce flicker for high base_scale).
- Distinct noise per cluster color (adds indexing complexity—defer).
- Distance based attenuation (less perturbation for tiny balls to avoid aliasing).

## Summary
This design cleanly layers a lightweight surface perturbation into the existing metaball pipeline with minimal risk and isolated configuration. It maintains backward compatibility and adheres to established repository patterns while enabling richer, more organic visuals.

---
Generated with accessibility in mind; please still review & test.
