# WGPU Shader Fix Prompt: Metaballs Unified (SurfaceNoiseParams) Alignment Audit

## Objective
You WILL fix the WebGPU / wgpu validation error occurring in the `metaballs_unified.wgsl` shader (group(2) binding(2)) where `SurfaceNoiseParams` currently has a reported size of 60 bytes (NOT a multiple of 16) causing pipeline creation failure on downlevel / WebGL-backed platforms that do NOT expose `DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED`.

## Current Failure
```
wgpu error: Validation Error
In Device::create_render_pipeline (label = 'opaque_mesh2d_pipeline')
  In the provided shader, the type given for group 2 binding 2 has a size of 60.
  Device requires uniform buffer binding sizes to be multiples of 16 bytes.
```

## Root Cause
`SurfaceNoiseParams` in WGSL defines 15 scalar values (f32/u32):
```
amp, base_scale, speed_x, speed_y,
warp_amp, warp_freq, gain, lacunarity,
contrast_pow, octaves, ridged, mode,
enabled, _pad0, _pad1
```
Each scalar = 4 bytes → 15 * 4 = 60 bytes.

While Rust mirrors this struct with `#[repr(C, align(16))]`, which pads the host-side size up to 64, the WGSL declared struct *itself* still has a logical size of 60. WebGPU’s uniform buffer layout rules (std140-like constraints on many downlevel adapters) require the bound size to be a multiple of 16. Because the WGSL reflection reports 60, validation fails before pipeline creation completes in browsers / downlevel backends.

## Constraints & Principles
You MUST:
1. Preserve existing binary ordering of semantic fields (no reordering of functional fields without necessity).
2. Maintain forward compatibility for potential future scalar additions.
3. Keep Rust and WGSL definitions strictly in sync (byte-for-byte) to avoid undefined behavior.
4. Prefer the **minimal change** that resolves alignment (add padding) over invasive refactors.
5. Provide a regression test / validation path to ensure future additions do not reintroduce misalignment.

## Fix Plan (Minimal / Recommended)
Add one more 4-byte dummy scalar (`_pad2: u32` or `f32`) at the end of the WGSL struct and mirror it in the Rust `SurfaceNoiseParamsUniform`. This increases the WGSL struct logical size from 60 → 64 bytes (a multiple of 16) satisfying wgpu validation on all targets.

### Updated WGSL Snippet
```wgsl
struct SurfaceNoiseParams {
    amp: f32,
    base_scale: f32,
    speed_x: f32,
    speed_y: f32,
    warp_amp: f32,
    warp_freq: f32,
    gain: f32,
    lacunarity: f32,
    contrast_pow: f32,
    octaves: u32,
    ridged: u32,
    mode: u32,
    enabled: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,   // NEW: brings total size to 64 bytes
};
```

### Updated Rust Snippet (`metaballs.rs`)
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
    pub ridged: u32,
    pub mode: u32,
    pub enabled: u32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32, // NEW padding to match WGSL 64-byte size
}
```

No semantic changes needed elsewhere; initialization paths can ignore the new padding field (leave zeroed). `ShaderType` derive will continue to reflect the struct layout to wgpu; ensure the derive is kept after adding the new field.

## Alternate (More Structural) Option (Not Chosen Now)
Pack related scalars into `vec4<f32>` groups (e.g., 4 * vec4 = 64 bytes) for clearer alignment guarantees and potential SIMD-friendly host-side operations. Rejected for now to minimize churn and avoid re-touching *all* field references; adopt only if further expansion pushes the struct beyond ~64 bytes where grouping reduces fragmentation.

## Step-by-Step Implementation Instructions
1. Edit `assets/shaders/metaballs_unified.wgsl`: append `_pad2: u32` to `SurfaceNoiseParams`.
2. Edit `src/rendering/metaballs/metaballs.rs`: append corresponding `pub _pad2: u32` to `SurfaceNoiseParamsUniform`.
3. (Optional) Add a static assertion style test in Rust verifying `std::mem::size_of::<SurfaceNoiseParamsUniform>() == 64`.
4. Rebuild native (`cargo run --features debug` or release) to confirm no validation errors.
5. Rebuild WASM target (`wasm-bindgen` / Bevy’s build pipeline) and load in browser; verify pipeline now initializes (no wgpu panic).
6. Manually toggle surface noise enable/disable to confirm runtime behavior unchanged.

## Regression Test Idea
Add unit test:
```rust
#[test]
fn surface_noise_uniform_size() {
    assert_eq!(std::mem::size_of::<SurfaceNoiseParamsUniform>(), 64);
}
```
Rationale: Fails fast if future field additions break 16-byte multiple requirement.

## Validation Checklist
- [ ] WGSL struct has 16 scalar fields (64 bytes)
- [ ] Rust struct size_of == 64
- [ ] No wgpu validation error on pipeline creation (native + wasm)
- [ ] Visual output identical (only structural padding added)
- [ ] Surface noise enable/disable still functions

## Post-Fix Recommendation
Document this alignment constraint in an internal shader layout notes section (consider updating `Copilot Instructions (Ball Matcher)` rendering/shader section) to avoid future accidental introduction of 60 / 76 / etc. byte sized uniforms.

## Ready for Execution
Proceed with the minimal padding addition now.
