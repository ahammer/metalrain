//! WebGPU adapter / device precondition probing & gating.
//!
//! Goals:
//! * Provide an early, consolidated diagnostic when an adapter does not meet
//!   minimum project requirements (limits, essential capabilities).
//! * Avoid silent black canvas scenarios on the web by panicking *before* heavy
//!   plugin initialization when requirements cannot be met.
//! * Expose a lightweight `WebGpuCapabilities` resource other systems can read.
//!
//! Integration:
//! * `assert_webgpu_available` (wasm) still runs very early in `main` to fail fast
//!   if `navigator.gpu` is missing.
//! * `WebGpuGuardPlugin` adds a startup system that waits for `RenderAdapter`
//!   (inserted by `RenderPlugin`) then performs checks exactly once.
//! * Native + wasm share the same evaluation code path. Wasm path filters out
//!   native-only features automatically using `Features::all_webgpu_mask()`.
//!
//! Testing suggestions (manual):
//! * Force a failure: temporarily raise `REQUIRED.max_texture_dimension_2d` to an
//!   unrealistically high value (e.g. 16384) and verify a consolidated panic with
//!   enumerated issues is produced (target="webgpu").
//! * Simulate missing optional compression feature messages by observing output
//!   on hardware without BC/ETC2/ASTC. These should *not* fail but log advisory.
//! * Observe a PASS log block on successful start: includes adapter summary,
//!   limits deltas, and feature flags.
//!
//! NOTE: We purposefully do not request a superset of adapter limits when creating
//! the device—Bevy handles device creation. We validate that adapter-provided
//! limits meet minima; failing early if not.

use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapter, RenderAdapterInfo};
use wgpu::{Adapter, Features, Limits};

// --------------------------------------------------------------------------------
// Early WASM availability guard (unchanged behavior, just moved down for docs).
// --------------------------------------------------------------------------------
/// WASM-only early guard to fail fast if WebGPU is unavailable.
/// We intentionally DO NOT offer a WebGL fallback: build requires `navigator.gpu`.
#[cfg(target_arch = "wasm32")]
pub fn assert_webgpu_available() {
    let win = web_sys::window().expect("no window");
    let nav = win.navigator();
    let key = wasm_bindgen::JsValue::from_str("gpu");
    let has_gpu = js_sys::Reflect::get(&nav, &key)
        .map(|v| !v.is_undefined())
        .unwrap_or(false);
    if !has_gpu {
        panic!("WebGPU (navigator.gpu) is required. Use a WebGPU-enabled browser (Chrome, Edge, Firefox Nightly w/ flag, or Safari Technology Preview). WebGL fallback intentionally disabled.");
    }
}

// --------------------------------------------------------------------------------
// Capability Data Structures
// --------------------------------------------------------------------------------
/// Hard minimum WebGPU limits for this project (see design rationale in prompt).
#[derive(Debug, Clone)]
pub struct RequiredWebGpu {
    pub max_bind_groups: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_uniform_buffer_binding_size: u32,
    pub max_storage_buffer_binding_size: u64,
    pub max_texture_dimension_2d: u32,
    pub max_color_attachments: u32,
}

impl RequiredWebGpu {
    pub const fn new() -> Self {
        Self {
            max_bind_groups: 4,
            max_storage_buffers_per_shader_stage: 4,
            max_uniform_buffer_binding_size: 64 * 1024,        // 64 KiB
            max_storage_buffer_binding_size: 32 * 1024 * 1024, // 32 MiB
            max_texture_dimension_2d: 2048,
            max_color_attachments: 4,
        }
    }
}

/// Captured and exposed capabilities after validation.
#[derive(Resource, Debug, Clone)]
pub struct WebGpuCapabilities {
    pub limits: Limits,
    pub features: Features,
    pub fallback: bool,
    pub compression_available: bool,
    pub f16_available: bool,
    pub bgra8_storage: bool,
}

// --------------------------------------------------------------------------------
// Core check function
// --------------------------------------------------------------------------------
/// Perform precondition checks against an already selected adapter.
/// On failure: logs all issues (`target="webgpu"`) then panics with summary.
pub fn ensure_webgpu_preconditions(adapter: &Adapter, adapter_info: &wgpu::AdapterInfo) -> WebGpuCapabilities {
    let required = RequiredWebGpu::new();
    info!(target: "webgpu", "Probing adapter...");

    // Basic adapter info
    let name = adapter_info.name.clone();
    let backend = format!("{:?}", adapter_info.backend); // wgpu::Backend variant
    let device_type = format!("{:?}", adapter_info.device_type);
    // wgpu 0.24 AdapterInfo no longer exposes `is_fallback`; approximate: treat CPU adapter as fallback.
    let fallback = matches!(adapter_info.device_type, wgpu::DeviceType::Cpu | wgpu::DeviceType::Other);

    info!(target: "webgpu", "Adapter=\"{name}\" backend={backend} device_type={device_type} fallback={fallback}");
    if fallback {
        warn!(target: "webgpu", "Fallback adapter in use; performance & limits may be reduced. Prefer a discrete GPU if available.");
    }

    // Gather features + limits
    let mut features = adapter.features();
    let limits = adapter.limits();
    let defaults = Limits::default();
    let web_mask = Features::all_webgpu_mask();

    // Filter out non-web features when compiling to wasm to avoid accidental over-request.
    #[cfg(target_arch = "wasm32")]
    {
        features &= web_mask;
    }

    // Soft feature availability flags.
    let compression_available = features.intersects(
        Features::TEXTURE_COMPRESSION_BC
            | Features::TEXTURE_COMPRESSION_ETC2
            | Features::TEXTURE_COMPRESSION_ASTC,
    );
    let f16_available = features.contains(Features::SHADER_F16);
    let bgra8_storage = features.contains(Features::BGRA8UNORM_STORAGE);

    // Accumulate failures with actionable messaging.
    let mut failures: Vec<String> = Vec::new();

    macro_rules! check_limit_u32 { ($field:ident) => {{
        if limits.$field < required.$field {
            failures.push(format!(
                "Limit {}={} below required {} (adapter insufficient for metaball pipeline)",
                stringify!($field), limits.$field, required.$field
            ));
        }
    }}; }
    macro_rules! check_limit_u64 { ($field:ident) => {{
        if u64::from(limits.$field) < required.$field {
            failures.push(format!(
                "Limit {}={} below required {} (insufficient buffer sizing headroom)",
                stringify!($field), u64::from(limits.$field), required.$field
            ));
        }
    }}; }

    check_limit_u32!(max_bind_groups);
    check_limit_u32!(max_storage_buffers_per_shader_stage);
    check_limit_u32!(max_uniform_buffer_binding_size);
    check_limit_u64!(max_storage_buffer_binding_size);
    check_limit_u32!(max_texture_dimension_2d);
    check_limit_u32!(max_color_attachments);

    // Detect exact downlevel_webgl2 defaults equivalence (treat as unsupported environment).
    if limits == wgpu::Limits::downlevel_webgl2_defaults() {
        failures.push("Adapter limits match downlevel_webgl2_defaults (environment too constrained)".to_string());
    }

    // Log limits delta (only highlight those either reduced vs default or those we required and OK)
    info!(target: "webgpu", "Limits (adapter vs defaults)");
    macro_rules! log_limit { ($field:ident, $required:expr) => {{
        let val = limits.$field; let def = defaults.$field; let req_u64 = $required as u64; let val_u64 = u64::from(val);
        let fail = val_u64 < req_u64;
        let status = if fail { "FAIL" } else { "OK" };
        let delta_note = if val < def { format!(" (< default {def})") } else { String::new() };
        info!(target: "webgpu", "  {:30} = {} ({}{}{}{})", stringify!($field), val, status, if status=="OK" {", >= required "} else {""}, if status=="OK" {($required).to_string()} else {String::new()}, delta_note);
    }}; }
    log_limit!(max_bind_groups, required.max_bind_groups);
    log_limit!(max_storage_buffers_per_shader_stage, required.max_storage_buffers_per_shader_stage);
    log_limit!(max_uniform_buffer_binding_size, required.max_uniform_buffer_binding_size);
    log_limit!(max_storage_buffer_binding_size, required.max_storage_buffer_binding_size);
    log_limit!(max_texture_dimension_2d, required.max_texture_dimension_2d);
    log_limit!(max_color_attachments, required.max_color_attachments);

    // Features summary formatting
    let mut feature_lines: Vec<&'static str> = Vec::new();
    if features.contains(Features::SHADER_F16) { feature_lines.push("shader-f16(+)"); } else { feature_lines.push("shader-f16(-)"); }
    if features.contains(Features::TEXTURE_COMPRESSION_BC) { feature_lines.push("texture-compression-bc"); }
    if features.contains(Features::TEXTURE_COMPRESSION_ETC2) { feature_lines.push("texture-compression-etc2"); }
    if features.contains(Features::TEXTURE_COMPRESSION_ASTC) { feature_lines.push("texture-compression-astc"); }
    if features.contains(Features::BGRA8UNORM_STORAGE) { feature_lines.push("bgra8unorm-storage(+)"); } else { feature_lines.push("bgra8unorm-storage(-)"); }
    info!(target: "webgpu", "Features(web) = [{}]", feature_lines.join(", "));
    if !compression_available {
        warn!(target: "webgpu", "No GPU texture compression feature available (BC/ETC2/ASTC). Bandwidth may be higher.");
    }

    if failures.is_empty() {
        info!(target: "webgpu", "WebGPU preconditions PASS; proceeding with device creation");
    } else {
        error!(target: "webgpu", "WebGPU preconditions FAILED ({} issues)", failures.len());
        for f in &failures { error!(target: "webgpu", " - {f}"); }
        panic!("WebGPU preconditions failed: {} issues (see log for details)", failures.len());
    }

    WebGpuCapabilities { limits, features, fallback, compression_available, f16_available, bgra8_storage }
}

// --------------------------------------------------------------------------------
// Bevy integration: one-shot startup system & plugin.
// --------------------------------------------------------------------------------
fn system_run_webgpu_guard(
    adapter: Res<RenderAdapter>,
    info: Res<RenderAdapterInfo>,
    mut commands: Commands,
    already: Option<Res<StateFlag>>, // ensure idempotence if somehow scheduled twice
) {
    if already.is_some() { return; }
    let caps = ensure_webgpu_preconditions(&adapter.0, &info.0);
    commands.insert_resource(caps);
    commands.insert_resource(StateFlag);
}

#[derive(Resource, Debug)]
struct StateFlag; // Marker to ensure guard runs only once.

/// Plugin registering the WebGPU precondition guard. Must be added *after* `RenderPlugin`.
pub struct WebGpuGuardPlugin;
impl Plugin for WebGpuGuardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, system_run_webgpu_guard);
    }
}

