# Agent Spike: Upgrade WASM build to WebGPU **compute** with **storage buffers** (no WebGL2 fallback)

**Objective**
Enable our Bevy WASM build to run compute shaders that read from **storage buffers** by forcing **core WebGPU**. We do **not** care about WebGL2/old GPU compatibility.

**Success Criteria**

- The app boots in Chrome/Edge (WebGPU) and runs the compute pipeline with a `var<storage, read>` buffer bound.
- No `max_storage_buffers_per_shader_stage limit is 0` errors at runtime.
- Console shows `maxStorageBuffersPerShaderStage >= 1` for the active adapter.
- Any prior “uniform buffer fallback” or GL-compat code paths are disabled by default.

---

## Work Plan

### 0) Repo discovery (don’t assume)

- Detect Bevy and wgpu versions.
- Grep for anything that smells like GL-compat:
  - `downlevel_webgl2_defaults`, `bevy_webgl2`, `WGPU_SETTINGS_PRIO`, `webgl`.
- Grep for our compute pipeline code and shader bindings:
  - storage buffer binding in WGSL: `var<storage` and `@binding(…)`.
  - Rust `BindGroupLayoutEntry` for `BufferBindingType::Storage`.

> If the project already forces WebGPU, skip to **Step 4** and wire up diagnostics.

---

### 1) Force **WebGPU** in Cargo features (disable WebGL2)

- In `Cargo.toml`, ensure Bevy compiles with **WebGPU** support and **without** WebGL2.
  - If you see any feature like `bevy_webgl2`, remove it.
  - Prefer enabling a `webgpu` feature if present. Examples (pick what matches our Bevy versioning scheme):

```toml
# Example A: turn off defaults and opt into webgpu explicitly
[dependencies]
bevy = { version = "*", default-features = false, features = ["bevy_winit", "bevy_asset", "bevy_render", "bevy_audio", "png", "webgpu"] }

# Example B: if the repo already uses default features, just add "webgpu" and ensure "webgl2" is not enabled anywhere
[dependencies]
bevy = { version = "*", features = ["webgpu"] }
````

- If there’s a workspace feature gating web targets, add a **`webgpu`** flag and use it in CI/build scripts for `wasm32-unknown-unknown`.

---

### 2) WASM target toolchain + runner

- Ensure target installed:

  - `rustup target add wasm32-unknown-unknown`
- Use one of these runners (pick one used in repo):

  - `wasm-server-runner`: `cargo install wasm-server-runner`
  - `trunk`: `cargo install trunk`
- Update the dev run command (package.json / Makefile / CI) to include **webgpu** feature:

  - wasm-server-runner:

    ```sh
    cargo run --target wasm32-unknown-unknown --features webgpu --release
    ```

  - trunk:

    ```sh
    trunk serve --release
    # and ensure the crate builds with feature "webgpu" via Cargo.toml or Trunk.toml
    ```

> Do **not** set up any WebGL2-specific flags or features.

---

### 3) App init: explicitly request WebGPU backend and sane limits

Add/modify the Bevy init code to force the browser WebGPU backend and request non-downlevel limits (at least one storage buffer per stage).

```rust
// e.g., in main.rs or our renderer bootstrap module
use bevy::prelude::*;
use bevy::render::settings::{WgpuSettings, WgpuFeatures}; // module path may be bevy::render::settings
use wgpu::{Backends, Limits}; // wgpu re-export may be available via bevy; else add wgpu dep in Cargo.toml (same version as bevy uses)

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            // On web, force the WebGPU backend (NOT WebGL2)
            backends: Some(Backends::BROWSER_WEBGPU),
            // Don’t force downlevel/webgl limits. Start with defaults (core WebGPU).
            features: WgpuFeatures::empty(),
            limits: Limits {
                // We need at least one storage buffer per stage for compute
                max_storage_buffers_per_shader_stage: 1,
                ..Limits::default()
            },
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, log_adapter_limits)
        .run();
}

// Optional: log real device limits at startup for sanity
fn log_adapter_limits() {
    // If we already have a RenderDevice handle in our Bevy version, log its limits.
    // Otherwise, skip (some versions don’t expose easily here).
    info!("If you see ‘0’ later, you are still on GL-compat. Fix features/backends/limits.");
}
```

Notes:

- If `bevy::render::settings` paths differ for our version, adapt imports. The intent is: **Backends::BROWSER_WEBGPU**, **Limits::default()** with **max_storage_buffers_per_shader_stage >= 1**.
- Do **not** call or import any `downlevel_webgl2_defaults()`.

---

### 4) Keep the compute pipeline **as storage** (no fallback)

Confirm our bindings stay as storage buffers:

**WGSL (unchanged)**

```wgsl
@group(0) @binding(3) var<storage, read> balls: array<Ball>;
```

**Rust pipeline (unchanged)**

```rust
// BindGroupLayoutEntry for binding=3:
ty: BindingType::Buffer {
    ty: BufferBindingType::Storage { read_only: true },
    has_dynamic_offset: false,
    min_binding_size: None,
}
```

Remove/disable any code paths that convert this to `Uniform` on web. If we keep a fallback for later, gate it behind an explicit cargo feature like `fallback_uniform_on_web` that is **off** by default.

---

### 5) Index.html hard fail if WebGPU missing (optional but honest)

Add a guard so unsupported browsers fail fast with a clear message.

```html
<script type="module">
if (!('gpu' in navigator)) {
  document.body.innerHTML = `
    <h3>WebGPU required</h3>
    <p>Your browser doesn’t support WebGPU. Use Chrome/Edge 113+ (or Firefox Nightly with flags).</p>`;
  throw new Error('WebGPU not available');
}
</script>
```

---

### 6) Verification checklist (manual)

1. Build & run for web with the **webgpu** feature:

   ```sh
   rustup target add wasm32-unknown-unknown
   cargo run --target wasm32-unknown-unknown --features webgpu --release
   ```

2. Open DevTools Console:

   - Confirm **no** `max_storage_buffers_per_shader_stage limit is 0` error.
   - Optional: In the console, check adapter limits directly:

     ```js
     (async () => {
       const adp = await navigator.gpu.requestAdapter();
       console.log('maxStorageBuffersPerShaderStage =', adp.limits.maxStorageBuffersPerShaderStage);
     })();
     ```

     Expect: `>= 1` (commonly `8`).
3. Visual sanity: compute path runs and renders as expected (metaballs animate).
4. Confirm that any GL-compat/uniform fallbacks are **not** active by default.

---

## Edits to make (as diffs where possible)

> Adjust paths to the actual files in the repo.

**Cargo.toml**

- **Remove** any `bevy_webgl2` or `webgl2` feature usage.
- **Add** `webgpu` feature to Bevy, or enable it via our workspace features.
- Ensure `wasm32-unknown-unknown` builds include `--features webgpu`.

**main.rs / renderer bootstrap**

- **Insert** `WgpuSettings` with `backends: Some(Backends::BROWSER_WEBGPU)` and `limits: Limits { max_storage_buffers_per_shader_stage: 1, ..Default::default() }`.

**Compute pipeline & shader**

- **Keep** storage buffer binding for `balls`. Do **not** downgrade to uniform on web.

**README.md**

- **Add** a “WebGPU-only (no WebGL2 fallback)” section:

  - Required browsers.
  - Build commands.
  - Known limitations (older Safari/Firefox may need flags or won’t work).

---

## Guardrails / Gotchas

- If you still see the 0-limit error, you’re either:

  - Accidentally enabling a GL/WebGL2 feature, or
  - Not running under a WebGPU-capable browser.
- Don’t mix `downlevel_webgl2_defaults` anywhere.
- If Bevy’s API changed (module paths for `WgpuSettings`/`Backends`/`Limits`), adapt imports; the goal is the same: **BROWSER_WEBGPU + core limits**.

---

## Deliverables

- ✅ Committed changes (Cargo, init code, docs).
- ✅ A short “why WebGPU-only” note in README.
- ✅ Screenshot or console paste showing `maxStorageBuffersPerShaderStage >= 1`.
- ✅ Confirmation that the compute pipeline initializes without validation errors.

**Commit message template**

```
feat(web): force core WebGPU for WASM; enable compute with storage buffers

- Remove WebGL2/compat features and limits
- Force Backends::BROWSER_WEBGPU and core Limits
- Keep compute pipeline using storage buffers
- Add WebGPU guard and README notes
```
