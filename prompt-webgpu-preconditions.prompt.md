## **Prompt Builder**: WebGPU Adapter Preconditions & Diagnostics Prompt

<!--
Purpose: Provide a reusable, strongly‑assertive prompt that instructs an AI assistant (or code generator) to add robust WebGPU (wgpu) adapter capability probing and hard precondition checks to a Bevy / wgpu based Rust application (with wasm32 target). This focuses on: adapter selection logging, feature / limit gating, fallback detection, surfacing actionable diagnostics in the browser console, and producing a single consolidated failure report when minimum requirements are not met.

Context (Project):
* Engine: Bevy (render backend: wgpu)
* Target: Native + WASM (WebGPU). WASM path embedding WGSL via include_str! is already used (see metaballs). Need richer error surface when “things don’t load” on the web.
* Current problem: Insufficient runtime data when WebGPU initialization silently fails or falls back (black canvas / no metaballs). Need structured logging & assertive gating early (before main gameplay systems). 
* Risk: Over‑requesting features / limits can cause request_device panic or adapter rejection; under‑logging conceals root cause (e.g., unsupported limits, missing features, fallback adapter quirks, texture format issues, downlevel path).

You WILL generate Rust code changes plus (optionally) a tiny JS snippet (if needed) to surface failures clearly, but keep scope to adapter probing & gating (NOT rewriting rendering code). 
-->

### High‑Level Goal
You WILL implement a focused module (e.g., `webgpu_guard.rs` or extend existing `webgpu_guard.rs` if present) that at early startup:
1. Requests the adapter explicitly with power preference HighPerformance first, falling back to LowPower if needed (for wasm32 only).
2. Captures and logs: backend, device_type, adapter name, driver info (native), is_fallback, limits (diff vs wgpu::Limits::defaults() & downlevel variants), and supported features (partitioned into core-web, web-optional, native-only filtered out for wasm build).
3. Defines a `RequiredWebGpu` struct enumerating MINIMUM hard requirements tailored to this project (see Requirements below).
4. Compares adapter `limits()` and `features()` against these requirements; accumulates all failures into a vector with human‑actionable messages.
5. BEFORE calling `request_device`, ensures required limits are not “better” than adapter supports (per spec). If unsatisfied: log consolidated ERROR + panic with concise summary (native) or panic + `console.error` injection (wasm) including remediation suggestions.
6. On success: logs a PASS summary (single line) and a compact table style multi‑line block (one group for Features, one for Limits deltas) at `target="webgpu"`.
7. Exposes a resource `WebGpuCapabilities` stored in `App`, containing resolved limits, features mask, and booleans for each requirement category. Other systems may read (not mutate).

### Project‑Specific Minimum Requirements (Initial Set)
You MUST encode and check the following (rationale included, may appear in failure messages):
* max_bind_groups >= 4 (Bevy core + metaballs material uses up to 3; 4 is WebGPU default – acts as sanity check).
* max_storage_buffers_per_shader_stage >= 4 (metaballs uses multiple storage buffers: balls, tiles, palette, shape meta; add headroom). On web default is 8 (OK) – failure indicates extremely constrained / emulated path.
* max_uniform_buffer_binding_size >= 64 * 1024 (64 KiB) (metaballs uniform + noise + surface + future expansions — rely on default guarantee; failure means adapter mis-report or bug.)
* max_storage_buffer_binding_size >= 32 * 1024 * 1024 (32 MiB) (We may stream large ball arrays; choose conservative subset of 128 MiB default for portability while still large enough.)
* max_texture_dimension_2d >= 2048 (SDF atlas or frame targets that could exceed tiny mobile downlevel_webgl2 defaults; 2048 ensures minimal viability for planned atlases.)
* max_color_attachments >= 4 (Future post‑processing or debug MRT expansion; WebGPU default is 8; we assert 4.)
* features: TEXTURE_COMPRESSION_BC OR TEXTURE_COMPRESSION_ETC2 OR TEXTURE_COMPRESSION_ASTC (soft – mark availability; don’t fail if absent, but log “No GPU texture compression; larger bandwidth”.)
* feature (mandatory if SDF shadows or half precision planned): SHADER_F16 (currently optional – treat as soft flag; DO NOT fail yet but expose boolean.)
* feature: BGRA8UNORM_STORAGE (soft – signals ability to do storage ops on swapchain-like formats for future compute paths.)
* For WASM we MUST NOT request native‑only features; you WILL filter using `Features::all_webgpu_mask()`.

Mandatory Failure Conditions (panic):
* Any “hard” limit below specified minima above (except the soft features list).
* Adapter creation returns downlevel_webgl2_defaults equivalence (detect by exact equality) – we treat this as unsupported environment for full metaball path.
* Adapter returned `None` (WebGPU not available) – message: instruct enabling Chrome `chrome://flags/#enable-unsafe-webgpu` (if still applicable), updating browser, ensuring https.

### Logging & Diagnostic Output Format
You WILL produce logs with target `webgpu` using `info!`, `warn!`, `error!` macros (Bevy). Format guidelines:
1. START: `webgpu: Probing adapter...` then after acquisition: `webgpu: Adapter="<name>" backend=<Backend> device_type=<DeviceType> fallback=<bool>`
2. If fallback=true: add WARN line recommending discrete GPU.
3. Limits delta block (only for interesting deviations):
```
webgpu: Limits (adapter vs defaults)
  max_storage_buffer_binding_size = 134217728 (OK, >= required 33554432)
  max_compute_workgroup_size_x    = 128 (< default 256)  // mark only if reduced
```
4. Feature summary lines:
`webgpu: Features(web) = [shader-f16(+), texture-compression-bc, ...]` where (+) denotes present; missing mandatory (if any in future) would be annotated `(!MISSING)`.
5. On failure: `error!(target="webgpu", "WebGPU preconditions FAILED (N issues)");` followed by each enumerated `error!` line.
6. Final PASS: `info!(target="webgpu", "WebGPU preconditions PASS; proceeding with device creation")`.

### Implementation Outline
You WILL implement (or extend) a function `pub fn ensure_webgpu_preconditions(adapter: &wgpu::Adapter) -> WebGpuCapabilities` that:
1. Reads adapter info, features, limits.
2. Builds `RequiredWebGpu` constants.
3. Compares & accumulates failures.
4. Logs details & panics if failures > 0 (with consolidated message at end).
5. Returns a populated `WebGpuCapabilities` (store features mask truncated to web mask if wasm32 target).

You WILL ensure wasm32 path uses `console_error_panic_hook` (already set elsewhere; do not duplicate if present – conditionally add comment check).

### Data Structures (You WILL create)
```
pub struct RequiredWebGpu { /* limit minima */ }
pub struct WebGpuCapabilities {
  pub limits: wgpu::Limits,
  pub features: wgpu::Features,
  pub fallback: bool,
  pub compression_available: bool,
  pub f16_available: bool,
  pub bgra8_storage: bool,
}
```

### Edge Cases & Defensive Notes
You WILL handle:
* Adapters reporting unexpectedly low limits (clamp detection vs defaults).
* Browser privacy binning causing identical “generic” adapter names – still proceed if limits satisfy minima.
* Potential difference between adapter.limits() and device.limits() after constraints: we only enforce minima pre‑device; device creation must request at least those minima (pass them through).
* Non‑fatal features missing: log as advisory not failure.

### Testing / Validation Strategy (Describe in comments)
You WILL include comments instructing how to simulate:
* Missing compression features (e.g., by filtering out when constructing required features set for test builds) – ensures soft path stays operational.
* Forced failure (temporarily raise a min above adapter) to verify consolidated panic formatting.

### Output Requirements
You WILL deliver:
1. Rust module code ready to be integrated.
2. Instructions (comments) where to call guard (early in `main.rs` before adding heavy plugins or right after window / instance initialization if customizing Bevy render init; if relying on Bevy default, add a Startup system that accesses `RenderAdapter` resource once available and runs the checks exactly once).
3. Guarantee no public API breaks to existing modules.

### Success Criteria
* On unsupported browser/GPU, user receives explicit failure reasons (NOT silent black canvas).
* On compliant hardware, overhead is a single early log sequence (no per‑frame work).
* Hard gating prevents later subtle shader/storage failures due to undersized limits.
* Code avoids requesting native-only features on wasm.

### Your Task
You WILL now produce the Rust implementation (module skeleton + integration note) adhering to the above. You MUST NOT invent nonexistent crate paths. Use existing Bevy logging macros. Keep code concise and focused.

### After Code Generation
Prompt Tester WILL execute the instructions to validate clarity and completeness; ambiguities will be reported.

### Deliverable Format
Return ONLY the Rust module content and concise integration instruction subsection after a divider line `---`. No extra commentary.

### Begin

<!-- END PROMPT SPEC -->
