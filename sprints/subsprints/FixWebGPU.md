# Agent Spike: WebGPU Compute (Storage Buffers) — Updated Plan & Audit

## Objective

Enable our Bevy WASM build to run compute shaders using `var<storage>` buffers under **core WebGPU** (no WebGL2 fallback) while keeping native (desktop) backends stable and ensuring shaders reliably load on web.

## Current Status (Post‑Attempt Audit)

| Aspect | Result | Notes |
|--------|--------|-------|
| WASM window / surface | ✅ | Graphics context & buffers created. |
| Compute pipelines queued | ✅ | Pipeline nodes entered Loading state. |
| WGSL shader load (wasm) | ❌ | Pipelines never reached Ready → shader asset load likely failed or delayed (filesystem delivery). |
| Native desktop run | ❌ | Forcing `Backends::BROWSER_WEBGPU` broke adapter selection on non-web targets. |
| Storage buffer limits log | ❌ | No diagnostic system; lack of visibility into adapter limits. |
| Fallback (uniform) removal | ✅ (implicit) | We retained storage binding, no evidence of downgrade path. |
| Explicit `webgpu` feature usage | Incomplete | Workspace `bevy` dependency lacks the `webgpu` feature flag. |

## Root Cause Analysis

1. Shader Non-Load on WASM
   - We rely solely on runtime `AssetServer` filesystem fetches (`asset_server.load("shaders/…")`). For `wasm-server-runner`, assets must be under the served root. If not copied/symlinked, requests 404 and pipeline compilation stalls.
   - No logging of shader load failures; silent stall until panic would occur only on explicit `Err` state.

2. Desktop GPU Regression
   - Unconditionally inserting `WgpuSettings { backends: Some(Backends::BROWSER_WEBGPU) }` invalidates native adapter discovery. `BROWSER_WEBGPU` is only meaningful in browsers — must be gated behind `cfg(target_arch = "wasm32")`.

3. Missing Diagnostics
   - No system enumerates adapter limits; absence of early assert allowed silent fallback suspicion.

4. Feature Configuration Gap
   - `webgpu` feature for Bevy not explicitly enabled → risk that downlevel / compatibility pathways still compile or optional WebGPU-specific code not activated.

5. Asset Root Divergence
   - Multiple demos each set their own `AssetPlugin` path; a centralized helper exists (`game_assets`), but `metaballs_test` uses a manual relative path. Inconsistency increases risk of wrong asset path in wasm.

## Updated Strategy (Revamped)

Focused on reliability, diagnostics, and native/web parity:

1. **Feature Enablement**: Add `"webgpu"` to the workspace `bevy` dependency (or gated behind a new workspace feature `web_webgpu` if we want optionality). Remove any `webgl2` reference (none present now, keep watching).
2. **Backend Selection (Scoped)**: Insert `WgpuSettings` with `backends: Some(Backends::BROWSER_WEBGPU)` only for `wasm32`. Let native auto-select (Vulkan/Metal/DX12) – no forced override.
3. **Shader Delivery Reliability**: Introduce a hybrid approach:
   - Default: filesystem loading (fast iteration, hot reload on native).
   - Optional feature `embed_shaders` → embed critical WGSL into the binary for wasm release / CI to eliminate 404 and load latency.
4. **Diagnostics & Assertions**:
   - Startup system logs: backend, adapter name (if exposed), and `max_storage_buffers_per_shader_stage`.
   - Assert (or error log + panic in debug) if the limit < 1.
   - Periodic debug log if compute pipelines still `Loading` after N frames (signals asset load failure).
5. **Asset Path Consistency**: Use centralized `configure_demo()` / `configure_workspace_root()` across demos; audit each demo main to remove duplicated manual `AssetPlugin` setups.
6. **WASM Dev Script Enhancements** (`scripts/wasm-dev.ps1`):
   - Pre-run copy or sync `assets/` into the build output directory served by `wasm-server-runner` (skip when `embed_shaders` feature is active).
   - Optional `-Embed` switch adds `--features embed_shaders` to the cargo run invocation.
7. **Index.html Guard**: Add (or keep) a WebGPU capability check for immediate user feedback, not a silent fail.
8. **README Update**: Document WebGPU-only requirement, how to enable embedding, and diagnostics expectations.

## Shader Delivery Approaches Considered

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| A. Always embed | Deterministic, no network latency | Larger WASM; no hot reload | Not chosen (iteration cost) |
| B. Filesystem only | Hot reload friendly | Risk of 404 / race on wasm | Insufficient alone |
| C. Trunk bundler | Robust static pipelining | Adds new tooling now | Later consideration |
| D. Hybrid flag (`embed_shaders`) | Flexible, CI-stable, dev-friendly | Slight complexity | ✅ Selected |

## Revised Work Plan (Actionable Steps)

### Step 0: Final Discovery (DONE)

Confirmed absence of `webgl2` features; found shader loads via `AssetServer` only; pipelines rely on those handles.

### Step 1: Cargo / Feature Updates

1. Add `"webgpu"` to workspace `bevy` dependency features.
2. Introduce `[features] embed_shaders = []` at workspace root (or in `metaball_renderer` + `game_assets` if scoping preferred).
3. (Optional) Add `web_webgpu` workspace feature if we need conditional CI matrix – otherwise always enable.

### Step 2: Conditional WgpuSettings

Wrap backend force:

```rust
#[cfg(target_arch="wasm32")]
app.insert_resource(WgpuSettings {
    backends: Some(Backends::BROWSER_WEBGPU),
    limits: Limits { max_storage_buffers_per_shader_stage: 1, ..Default::default() },
    ..Default::default()
});
```

Do NOT insert on native.

### Step 3: Shader Embedding (Feature-Gated)

```rust
#[cfg(feature = "embed_shaders")] const COMPUTE_METABALLS_WGSL: &str = include_str!("../../assets/shaders/compute_metaballs.wgsl");
#[cfg(feature = "embed_shaders")] fn register_embedded_shaders(world: &mut World) {
    use bevy::asset::Assets; use bevy::prelude::*; use bevy::render::render_resource::Shader;
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    let handle = shaders.add(Shader::from_wgsl(COMPUTE_METABALLS_WGSL));
    // store handle somewhere (resource) so pipelines can use it immediately.
}
```

Pipelines: if `embed_shaders`, skip `asset_server.load` and use stored handle.

### Step 4: Filesystem Delivery (Non-Embedded Path)

Enhance `wasm-dev.ps1` to sync assets into the served directory prior to `cargo run`.

### Step 5: Diagnostics Systems

Add after plugins:

```rust
fn log_adapter_limits(render_device: Res<bevy::render::renderer::RenderDevice>) {
    let l = render_device.limits();
    info!(target: "diagnostics", "Adapter limits: max_storage_buffers_per_shader_stage={}", l.max_storage_buffers_per_shader_stage);
    assert!(l.max_storage_buffers_per_shader_stage >= 1, "Storage buffers per stage == 0 (unexpected WebGL path)");
}
```

Optional system counting frames until pipeline readiness to warn after e.g. 120 frames.

### Step 6: README & Index.html

Document required browsers, embedding flag, and diagnostic output signature. Add WebGPU guard script if absent.

### Step 7: Verification

1. Native run → confirm successful backend selection (no BROWSER_WEBGPU forced, compute still runs).
2. WASM dev (filesystem) → verify network requests for WGSL succeed (no 404), pipeline ready quickly.
3. WASM with `--features embed_shaders` → zero WGSL network requests; immediate pipeline readiness.
4. Console shows limit ≥ 1.

## Updated Success Criteria

| Criterion | Updated Definition |
|-----------|--------------------|
| Storage buffer availability | Limit logged and ≥ 1 on all targets |
| Shader reliability | Pipelines reach Ready within 2–3 frames (embed) or a short window (filesystem); fail fast with clear log if not |
| Native stability | Desktop run unaffected; no adapter not found errors |
| Web fallback removal | No implicit downgrade to uniforms; no `WGPU_SETTINGS_PRIO=webgl2` reliance |
| Observability | Adapter limits & pipeline readiness logged; assertion on invalid limit |

## Guardrails / Gotchas

| Risk | Mitigation |
|------|------------|
| 0 limit still appears | Confirms unintended WebGL path / environment; assert early |
| Shader 404 on wasm | Asset sync step or enable `embed_shaders` |
| Pipeline stuck in Loading | Add frame-count warning & ensure correct handle source selection |
| Native backend break | Keep backend override behind `cfg(target_arch="wasm32")` |
| Drift in asset roots | Standardize via `configure_demo()` helpers |

## Implementation Checklist (to execute next)

1. Cargo: add `webgpu` feature to `bevy` dependency.
2. Add `[features] embed_shaders = []` and gate code.
3. Add wasm-only `WgpuSettings` insertion helper.
4. Implement embedded shader registration + handle resource.
5. Modify compute pipelines to branch on feature for handle acquisition.
6. Enhance `scripts/wasm-dev.ps1` asset sync and `-Embed` switch.
7. Add diagnostics systems (limits + pipeline readiness optional).
8. Update README (WebGPU-only, embedding flag, expected logs).
9. Optional: Add automated smoke test (headless wasm build) verifying no compile errors.

## Deliverables

- Updated Cargo / feature configuration.

- Conditional backend + diagnostics code merged.
- Hybrid shader delivery implemented; default dev iteration unchanged.
- README section: “WebGPU-only & Shader Delivery Modes”.
- Script updates for wasm dev + embed flag.
- Console evidence: adapter limits and pipeline readiness.

## Commit Message Template

```
feat(webgpu): stabilize WebGPU compute (storage buffers) with hybrid shader delivery & diagnostics

- Add Bevy webgpu feature; remove any implicit webgl2 paths
- Gate BROWSER_WEBGPU backend to wasm only
- Introduce optional embed_shaders feature for deterministic wasm builds
- Add adapter limits logging + assertion
- Improve wasm dev script asset sync & embed mode
```

## Follow-Up (Optional Enhancements)

- Add a small integration test that instantiates the app for a few frames and checks pipeline cache state (native only).

- Introduce trunk-based build pipeline later for production packaging and hashed assets.
- Explore automatic shader pre-warm metrics / timing logs to monitor WGSL compile latency.

---

> Previous plan retained below for historical context has been superseded by the updated strategy above.
