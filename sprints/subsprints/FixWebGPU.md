# Agent Spike: WebGPU Compute (Storage Buffers) — Updated Plan & Audit

## Objective

Enable our Bevy WASM build to run compute shaders using `var<storage>` buffers under **core WebGPU** (no WebGL2 fallback) while keeping native (desktop) backends stable and ensuring shaders reliably load on web.

## Phase 1 Results (Executed)

| Aspect | Target | Actual Result | Status |
|--------|--------|---------------|--------|
| Bevy `webgpu` feature | Enable | ✅ Added to workspace dependency | Complete |
| `embed_shaders` feature | Add crate feature | ✅ Added to `metaball_renderer` | Complete |
| Wasm-only backend gating | Conditional insert | ⚠️ Skipped (WgpuSettings not Resource) | Acceptable - default works |
| Shader embedding path | Compile-time inclusion | ✅ Both compute shaders embed when feature enabled | Complete |
| Adapter limits diagnostics | Log & assert | ✅ Logs `max_storage_buffers_per_shader_stage=10` | Complete |
| Pipeline readiness watchdog | Warn after N frames | ❌ Not implemented | Phase 2 |
| Asset path (wasm) | Sync or configure | ❌ Still `../../assets` → 404s | **Blocking** |
| Meta file suppression | AssetMetaCheck | ❌ Still defaults → 404 .meta | Phase 2 |
| wasm-dev script sync | Copy to served path | ⚠️ Copies to wrong location | **Blocking** |
| README updates | Document approach | ✅ Added WebGPU section | Complete |

## Current Runtime Failure (Phase 1 Exit State)

**Symptom**: Panic at `pipeline.rs:410` with "Pipeline could not be compiled because the following shader could not be loaded"

**Browser Console**: 404 errors for:

- `/assets/shaders/compute_metaballs.wgsl.meta`
- `/assets/shaders/compute_3d_normals.wgsl.meta`
- `/assets/shaders/present_fullscreen.wgsl.meta`
- (Likely also the `.wgsl` files themselves, though may not show distinct 404s)

**Root Causes Identified**:

1. **Asset path mismatch**: Demo uses `AssetPlugin { file_path: "../../assets" }` but wasm-server-runner serves from demo package root
2. **Script sync destination wrong**: Copies to `embedded_assets/` but runtime never looks there
3. **Meta file noise**: Default `AssetMetaCheck::Always` generates extra failing requests
4. **Eager pipeline creation**: `FromWorld` queues pipeline before shader `Handle` resolves → premature compile attempt

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

## Phase 2 Work Plan: Fix Asset Delivery & Pipeline Staging

**Objective**: Eliminate 404s, prevent premature pipeline compilation, add observability.

### Step 1: Fix Asset Path for WASM ✅ PRIORITY

Change `demos/metaballs_test/src/lib.rs` to use conditional AssetPlugin configuration:

```rust
use bevy::asset::{AssetPlugin, AssetMetaCheck};

pub fn run_metaballs_test() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins({
            #[cfg(target_arch = "wasm32")]
            {
                DefaultPlugins.set(AssetPlugin {
                    file_path: "assets".into(),
                    processed_file_path: "imported_assets/Default".into(),
                    meta_check: AssetMetaCheck::Never,
                    watch_for_changes_override: Some(false),
                    ..default()
                })
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                DefaultPlugins.set(AssetPlugin {
                    file_path: "../../assets".into(),
                    ..default()
                })
            }
        })
        // ... rest of setup
        .run();
}
```

**Why**: On wasm32, `wasm-server-runner` serves from the demo package directory. Using `"assets"` as a relative path allows the runtime to find files copied there, while `AssetMetaCheck::Never` suppresses `.meta` 404 noise.

### Step 2: Fix Script Asset Sync Destination ✅ PRIORITY

Update `scripts/wasm-dev.ps1` to copy assets to the correct location:

```powershell
if (-not $Embed) {
  Write-Section "Syncing assets (non-embedded mode)"
  $dest = Join-Path $root "demos/metaballs_test/assets"  # Changed from embedded_assets
  if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
  Copy-Item (Join-Path $root 'assets') $dest -Recurse
}
```

**Why**: Must align script destination with runtime path configured in Step 1.

### Step 3: Add Pipeline Readiness Watchdog

Create `crates/metaball_renderer/src/compute/watchdog.rs`:

```rust
use bevy::prelude::*;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
use super::pipeline::GpuMetaballPipeline;
use super::pipeline_normals::GpuNormalsPipeline;

#[derive(Resource, Default)]
pub struct PipelineWatchdog {
    pub frames_waiting: u32,
    pub warned_metaballs: bool,
    pub warned_normals: bool,
}

pub fn metaball_pipeline_watchdog(
    mut wd: ResMut<PipelineWatchdog>,
    cache: Res<PipelineCache>,
    metaball_pipeline: Option<Res<GpuMetaballPipeline>>,
    normals_pipeline: Option<Res<GpuNormalsPipeline>>,
) {
    let mut all_ready = true;
    
    if let Some(pipeline) = metaball_pipeline {
        match cache.get_compute_pipeline_state(pipeline.pipeline_id) {
            CachedPipelineState::Ok(_) => {},
            CachedPipelineState::Err(e) => {
                error!("Metaballs compute pipeline failed: {}", e);
                return;
            },
            _ => {
                all_ready = false;
                if !wd.warned_metaballs && wd.frames_waiting >= 120 {
                    warn!("Metaballs compute pipeline still Loading after 120 frames.\nCheck: 1) Network 404s in browser console; 2) Asset path config; 3) Try with -Embed flag");
                    wd.warned_metaballs = true;
                }
            }
        }
    }
    
    if let Some(pipeline) = normals_pipeline {
        match cache.get_compute_pipeline_state(pipeline.pipeline_id) {
            CachedPipelineState::Ok(_) => {},
            CachedPipelineState::Err(e) => {
                error!("Normals compute pipeline failed: {}", e);
                return;
            },
            _ => {
                all_ready = false;
                if !wd.warned_normals && wd.frames_waiting >= 120 {
                    warn!("Normals compute pipeline still Loading after 120 frames.\nCheck: 1) Network 404s in browser console; 2) Asset path config; 3) Try with -Embed flag");
                    wd.warned_normals = true;
                }
            }
        }
    }
    
    if !all_ready {
        wd.frames_waiting += 1;
        
        if cfg!(debug_assertions) && wd.frames_waiting >= 600 {
            panic!("Compute pipelines failed to become Ready in 600 frames (debug build timeout).\nLikely causes:\n- Asset 404s (check Network tab)\n- Incorrect AssetPlugin.file_path for wasm\n- Missing shader files");
        }
    }
}
```

Add to `settings.rs` plugin build:

```rust
render_app.init_resource::<crate::compute::watchdog::PipelineWatchdog>();
render_app.add_systems(Render, crate::compute::watchdog::metaball_pipeline_watchdog);
```

### Step 4: Improve Pipeline Error Messages

Update panic sites in `pipeline.rs` and `pipeline_normals.rs` Node::update methods:

```rust
CachedPipelineState::Err(err) => {
    panic!(
        "Failed to compile metaballs compute pipeline.\n\
        Error: {}\n\
        Shader handle: {:?}\n\
        Troubleshooting:\n\
        - On WASM: Check browser Network tab for 404 errors on .wgsl files\n\
        - Verify AssetPlugin.file_path matches served directory\n\
        - Try embedded mode: cargo run --target wasm32-unknown-unknown --features metaball_renderer/embed_shaders\n\
        - Check that assets/ directory exists and contains shaders/", 
        err, 
        pipeline.pipeline_id
    )
}
```

### Step 5: Extend Embedding to Present Shader (Optional)

If `present_fullscreen.wgsl` also needs embedding, add to `present/mod.rs`:

```rust
#[cfg(feature = "embed_shaders")]
const PRESENT_FULLSCREEN_WGSL: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/shaders/present_fullscreen.wgsl"
));

// In shader load site:
let shader: Handle<Shader> = {
    #[cfg(feature = "embed_shaders")]
    {
        let mut shaders = world.resource_mut::<Assets<Shader>>();
        shaders.add(Shader::from_wgsl(
            PRESENT_FULLSCREEN_WGSL,
            "embedded://present_fullscreen.wgsl",
        ))
    }
    #[cfg(not(feature = "embed_shaders"))]
    {
        asset_server.load("shaders/present_fullscreen.wgsl")
    }
};
```

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

## Phase 2 Checklist (Actionable – Execute Next)

- [ ] **Fix wasm AssetPlugin path**: Add conditional `#[cfg(target_arch = "wasm32")]` block with `file_path: "assets"` and `meta_check: AssetMetaCheck::Never`
- [ ] **Fix wasm-dev.ps1 sync**: Change destination from `embedded_assets/` to `demos/metaballs_test/assets`
- [ ] **Add watchdog module**: Create `crates/metaball_renderer/src/compute/watchdog.rs` with frame counter and warnings
- [ ] **Wire watchdog into plugin**: Add `init_resource` and system in `settings.rs`
- [ ] **Enrich panic messages**: Update `pipeline.rs` and `pipeline_normals.rs` error messages with troubleshooting steps
- [ ] **Extend embedding to present shader**: Add `PRESENT_FULLSCREEN_WGSL` constant and conditional loading in `present/mod.rs`
- [ ] **Test non-embedded mode**: Run `pwsh scripts/wasm-dev.ps1` and verify zero 404s
- [ ] **Test embedded mode**: Run `pwsh scripts/wasm-dev.ps1 -Embed` and verify zero shader network requests
- [ ] **Update FixWebGPU.md**: Mark Phase 2 complete and add Phase 3 planning

## Phase 1 Deliverables (Completed)

- ✅ Updated Cargo / feature configuration (webgpu, embed_shaders)
- ✅ Conditional shader embedding implemented
- ✅ Adapter limits diagnostics added
- ✅ README section: "WebGPU-only & Shader Delivery Modes"
- ✅ Script `-Embed` switch added
- ⚠️ Asset path still needs wasm fix (Phase 2)

## Phase 2 Success Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| 404 Elimination | Zero 404s for `.wgsl` in Network tab (non-embedded) |
| Meta Noise | No `.wgsl.meta` requests when `AssetMetaCheck::Never` |
| Panic Timing | Watchdog warns at 120 frames; panics at 600 (debug only) |
| Embedded Mode | Zero network shader requests; pipeline ready in ≤5 frames |
| Logging | Clear warnings with actionable troubleshooting steps |

## Commit Message Templates

### Phase 1 (Completed)

```text
feat(webgpu): add WebGPU support with hybrid shader delivery

- Add Bevy webgpu feature to workspace dependency
- Introduce embed_shaders feature for deterministic WASM builds
- Add adapter limits diagnostics + assertion
- Update README with WebGPU requirements
- Add -Embed switch to wasm-dev.ps1 script
```

### Phase 2 (To Execute)

```text
fix(webgpu): resolve WASM asset delivery and add pipeline watchdog

- Fix AssetPlugin.file_path for wasm32 target (use "assets")
- Suppress .meta file requests with AssetMetaCheck::Never
- Correct wasm-dev.ps1 asset sync destination
- Add pipeline readiness watchdog with frame counter
- Enrich panic messages with troubleshooting guidance
- Extend embedding to present_fullscreen.wgsl shader
```

## Phase 3 Preview: Performance & Build Hygiene

### Objectives

1. Reduce wasm binary size (169 MB debug is fine, but release + LTO guidelines).
2. Add a headless native smoke test verifying pipeline readiness within a small frame budget.
3. Optional: unify a `web_embed_shaders` workspace feature alias.
4. Introduce structured timing metrics (shader compile duration, first frame to Ready).

### Action Steps

1. **Release Build Guidance & Script Flag**: Add `-Release` + `-Embed` recommendation for CI.
2. **Smoke Test (Native)**: New test in `metaball_renderer/tests/ready.rs` asserting pipeline ready < 240 frames.
3. **Metrics**: Resource timestamps for shader request, pipeline queued, pipeline ready; log derived durations.
4. **Feature Alias**: Optional root `Cargo.toml` feature `web_embed_shaders = ["metaball_renderer/embed_shaders"]`.
5. **Bundle Prep (Exploratory)**: Evaluate moving to `trunk` or `wasm-bindgen-cli` + asset hashing (defer unless needed).

---

> Previous plan retained below for historical context has been superseded by the updated strategy above.
