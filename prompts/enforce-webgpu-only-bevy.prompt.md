# Bevy WebGPU-Only Enforcement Prompt

## Purpose
Force a Bevy 0.16 project that already depends on `wgpu` 26.x to run strictly on WebGPU backends (no WebGL / GL fallback) across native and WASM builds, ensuring configuration, feature flags, and runtime settings reject unintended GL paths. Provide corrective actions if misconfiguration is detected.

## When You Use This Prompt
Use when reviewing or updating a Bevy project's rendering setup to guarantee:
- Only modern WebGPU backends are compiled / requested.
- `webgl` feature is not enabled anywhere (Bevy or wgpu).
- WASM build uses `Backends::BROWSER_WEBGPU` and fails fast if unavailable instead of silently falling back.

## Preconditions (You MUST verify)
1. Bevy version is 0.16.x.
2. wgpu dependency present with `default-features = false` and contains `features = ["webgpu", "wgsl"]`.
3. No dependency enables `webgl` (search Cargo.toml / cargo metadata).
4. No code path manually enables GL (no `Backends::GL`).
5. For WASM builds, a feature gate or cfg block sets backends to `Backends::BROWSER_WEBGPU` only.

If any precondition fails: You MUST output a remediation plan before changing code.

## Required Actions
You WILL perform these steps in order:
1. Scan Cargo.toml for any of:
   - `webgl` feature in `wgpu` or transitive crate enablement.
   - Bevy feature enabling WebGL (e.g. `webgl` via `bevy_webgl2` in older versions – SHOULD NOT exist in 0.16).
2. If found, remove those features and add an explicit explanatory comment near the wgpu dependency why WebGL is disabled.
3. Ensure `bevy` dependency does NOT override default features to reintroduce GL unless explicitly required for non-web; leave defaults unless they bring GL (they don't for 0.16 without webgl feature).
4. Add (or confirm) a startup resource insertion configuring wgpu:
```rust
use bevy::prelude::*;
use bevy::render::settings::{WgpuSettings, Backends, RenderCreation};

pub struct ForceWebGpuPlugin;
impl Plugin for ForceWebGpuPlugin { fn build(&self, app: &mut App) {
    // Highest priority: override before renderer initializes
    app.insert_resource(RenderCreation::Automatic(WgpuSettings {
        backends: Some(Backends::VULKAN | Backends::DX12 | Backends::METAL | Backends::BROWSER_WEBGPU),
        ..Default::default()
    }));
}}
```
5. For WASM target specifically, restrict strictly to browser WebGPU backends:
```rust
#[cfg(target_arch = "wasm32")]
app.insert_resource(RenderCreation::Automatic(WgpuSettings {
    backends: Some(Backends::BROWSER_WEBGPU),
    ..Default::default()
}));
```
6. Insert a runtime check (WASM) after startup: if adapter info indicates downlevel GL (should be unreachable without webgl feature), log error and panic with a clear message.
7. Add documentation snippet to README under Rendering clarifying WebGPU-only stance and reasoning (consistency, feature parity, modern pipeline requirements, no GL path maintenance burden).
8. Output diff(s) for added plugin file (e.g. `src/rendering/force_webgpu.rs`) and registration inside main/game plugin.

## Validation Steps (You MUST perform)
- Build native: `cargo build` (should succeed; no GL warnings).
- Build WASM (example): `wasm-pack build` or `cargo build --target wasm32-unknown-unknown` (depends on project tooling). Confirm no GL backend compile gates appear.
- (Optional) Run with `RUST_LOG=wgpu=info` and confirm logged backend is one of METAL/DX12/VULKAN/BROWSER_WEBGPU only.
- Grep for `Backends::GL` and ensure zero matches.
- Grep for `webgl` in repository; ensure zero matches.

## Remediation Guidance (If Failures)
- If a platform requires GL (very old browsers), declare it unsupported; do NOT reintroduce WebGL fallback in this project.
- If CI builds fail due to missing backend (e.g., headless Linux without Vulkan), add `Backends::PRIMARY` subset selectively via env-controlled override but never `GL`.

## Output Format Rules
You MUST:
- Provide explicit file diffs for modifications / additions.
- List any commands executed for validation.
- Summarize final state: features removed, plugin added, docs updated.

You MUST NOT:
- Add WebGL fallback logic.
- Suppress hard failure when WebGPU unavailable on browser.
- Introduce unrelated refactors.

## References (Authoritative)
- Bevy 0.16 `WgpuSettings` docs: fields `backends` note default enables DX12/METAL/VULKAN; GL only with `webgl` feature.
- wgpu 26 `InstanceDescriptor.backends` semantics: `Backends::BROWSER_WEBGPU` isolates to WebGPU adapters; no opportunistic GL fallback when feature absent.

## Success Criteria
- Cargo manifest free of `webgl` indicators.
- Code sets `RenderCreation::Automatic` with explicit backend mask (native) & stricter mask (wasm).
- Failing fast on unsupported browsers (clear panic message referencing WebGPU requirement).
- README states WebGPU-only policy.

---
Use imperative remediation summary at the end of your output: "WEBGPU-ONLY ENFORCED" if all steps complete.
