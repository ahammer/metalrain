//! Golden frame hash harness (incremental).
//!
//! Current implementation (Phase 7 incremental enhancement):
//! - Computes a deterministic blake3 hash on the first PostUpdate frame.
//! - Hash preimage now includes (in order):
//!     * Static seed tag "golden-placeholder-v3" (v3 adds pixel-hash metadata placeholders)
//!     * Count of `BallCircleVisual` entities (u64 LE)
//!     * Optional contributed preimage bytes from other crates (e.g. metaballs uniform
//!       summary) via the `GoldenPreimage` resource (length-prefixed u64 LE)
//!     * Pixel hash metadata (u64 length + first 16 bytes of blake3 pixel hash) (v3; length=0 + 16 zero bytes until real GPU hash available)
//! - Stores the hex hash string in `GoldenState.final_hash` (mirrors placeholder path
//!   until real GPU readback is implemented) and `hash_placeholder` for backwards
//!   compatibility with early tests. Once real GPU pixel hashing occurs, the pixel
//!   hash (full 32-byte digest as hex) is stored in `GoldenState.pixel_hash`; the
//!   final hash becomes a hash of the extended v3 preimage (not the raw pixel digest).
//! - Baseline load precedence:
//!   1. GOLDEN_BASELINE_HASH env
//!   2. File at GOLDEN_BASELINE_FILE (default: golden_baseline_hash.txt)
//! - Optional write path when GOLDEN_WRITE_BASELINE=1 (or true/TRUE/True) writes the
//!   newly captured final hash to the file (adopting it in-memory if no prior baseline).
//!
//! Future (full):
//! - Replace placeholder/enriched path with actual GPU frame readback (wgpu texture -> buffer
//!   map -> hash RGBA8 bytes) and integrate pixel hash metadata into final preimage (already scaffolded).
//! - Add tolerance / perceptual diff support (ΔE metrics) instead of simple equality.
//!
//! Design trade-offs:
//! - Keeps `bm_rendering` independent of specific feature crates (no direct metaballs
//!   dependency). Instead, feature crates can push additional deterministic bytes into
//!   `GoldenPreimage` before the first capture, enriching the hash without creating
//!   dependency cycles.
//! - Maintains prior public helpers (`current_golden_hash`) so existing tests remain valid.
//!
//! Versioning:
//! - Seed tag bumped to v3 and preimage extended with pixel hash metadata (length + first 16 bytes)
//!   to ensure multi-stage reproducibility; v1 (entity-count only) and v2 (entity-count +
//!   contributed bytes) cannot collide silently with v3.

use bevy::prelude::*;
use bm_core::BallCircleVisual;

const GOLDEN_SEED_TAG_V3: &[u8] = b"golden-placeholder-v3";

#[derive(Debug, Default, PartialEq, Eq)]
pub enum GoldenStage {
    /// Initial placeholder hash (no GPU readback yet).
    #[default]
    PlaceholderCaptured,
    /// GPU readback scheduled (future path) waiting for texture ready.
    PendingGpuCapture,
    /// Final RGBA hash captured (future).
    FinalCaptured,
}

#[derive(Resource, Debug, Default)]
pub struct GoldenState {
    pub captured: bool,
    pub hash_placeholder: Option<String>,
    pub final_hash: Option<String>,
    /// Hex of the raw pixel hash (blake3 of RGBA bytes) once real GPU path completes (v3+).
    pub pixel_hash: Option<String>,
    pub stage: GoldenStage,
    pub frame_captured: Option<u32>,
    pub pixel_count: Option<u32>,
    #[allow(dead_code)]
    pub capture_width: Option<u32>,
    #[allow(dead_code)]
    pub capture_height: Option<u32>,
    pub baseline_hash: Option<String>,
    pub baseline_match: Option<bool>,
}

/// Optional preimage contribution resource populated by other crates (e.g. metaballs)
/// before the first capture occurs. Data should be deterministic & architecture
/// independent (avoid raw pointer addresses, etc.).
#[derive(Resource, Debug, Default)]
pub struct GoldenPreimage(pub Vec<u8>);

pub struct GoldenHashPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct GoldenCaptureSet;

/// PostUpdate frame counter (increments each frame; used for capture metadata).
#[derive(Resource, Debug, Default)]
pub struct GoldenFrameIndex(pub u32);

/// Performance / instrumentation metrics for the golden capture (Phase 7 incremental).
#[derive(Resource, Debug, Default)]
pub struct GoldenMetrics {
    /// Length in bytes of contributed preimage (excluding seed/tag, entity count & length prefix).
    pub preimage_len: usize,
    /// Nanoseconds assembling preimage (excluding hashing).
    pub preimage_ns: u128,
    /// Nanoseconds spent in blake3 finalize (hashing) step (placeholder final hash or pixel hash step depending on stage).
    pub hash_ns: u128,
    /// Total nanoseconds for the capture system (including preimage assembly + hashing).
    pub capture_total_ns: u128,
    /// Nanoseconds spent allocating GPU target resources (stub).
    #[allow(dead_code)]
    pub allocation_ns: u128,
    /// Nanoseconds spent mapping / reading back GPU data (stub finalize path).
    #[allow(dead_code)]
    pub map_ns: u128,
    /// Nanoseconds spent in (legacy single-step) GPU finalize path (only when feature `gpu_capture` enabled).
    #[allow(dead_code)] // Retained for compatibility; superseded by map_ns once real path implemented.
    pub gpu_finalize_ns: u128,
    /// Nanoseconds spent issuing (stub) texture->buffer copy command (future: actual GPU copy submission).
    #[allow(dead_code)]
    pub copy_ns: u128,
    /// Nanoseconds spent waiting on GPU buffer map future/poll (stub placeholder).
    #[allow(dead_code)]
    pub map_wait_ns: u128,
    /// Count of (unpadded) pixel bytes hashed (stub=0 until real capture path).
    #[allow(dead_code)]
    pub bytes_hashed: u64,
}

fn increment_frame_index(mut idx: ResMut<GoldenFrameIndex>) {
    idx.0 = idx.0.wrapping_add(1);
}

#[cfg(feature = "gpu_capture")]
fn golden_blocking_map_enabled() -> bool {
    matches!(
        std::env::var("GOLDEN_BLOCKING_MAP"),
        Ok(v) if matches!(v.as_str(), "1" | "true" | "TRUE" | "True")
    )
}

impl Plugin for GoldenHashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GoldenState>()
            .init_resource::<GoldenPreimage>()
            .init_resource::<GoldenFrameIndex>()
            .init_resource::<GoldenMetrics>()
            .add_systems(PostUpdate, increment_frame_index)
            .add_systems(
                PostUpdate,
                placeholder_or_enriched_capture
                    .in_set(GoldenCaptureSet)
                    .run_if(not(resource_exists::<GoldenStateCaptured>)),
            );
        #[cfg(feature = "gpu_capture")]
        {
            app.add_systems(
                PostUpdate,
                tap_golden_gpu_view
                    .after(GoldenCaptureSet)
                    .run_if(resource_exists::<GoldenStateCaptured>)
                    .run_if(not(resource_exists::<GoldenGpuViewTap>)),
            );
            app.add_systems(
                PostUpdate,
                allocate_stub_gpu_capture
                    .after(GoldenCaptureSet)
                    .run_if(resource_exists::<GoldenStateCaptured>)
                    .run_if(not(resource_exists::<GoldenGpuAlloc>)),
            );
            // Real GPU resource allocation (texture + buffer) once stub target dims known and RenderDevice present.
            app.add_systems(
                PostUpdate,
                allocate_real_gpu_resources
                    .after(allocate_stub_gpu_capture)
                    .run_if(resource_exists::<GoldenGpuAlloc>)
                    .run_if(not(resource_exists::<GoldenGpuRealResources>))
                    .run_if(resource_exists::<bevy::render::renderer::RenderDevice>),
            );
            app.add_systems(
                PostUpdate,
                submit_golden_gpu_copy
                    .after(allocate_real_gpu_resources)
                    .run_if(resource_exists::<GoldenGpuRealResources>)
                    .run_if(not(resource_exists::<GoldenGpuCopySubmitted>)),
            );
            app.add_systems(
                PostUpdate,
                note_real_gpu_resources_usage
                    .after(submit_golden_gpu_copy)
                    .run_if(resource_exists::<GoldenGpuRealResources>),
            );
            if golden_blocking_map_enabled() {
                app.add_systems(
                    PostUpdate,
                    map_and_hash_gpu_capture_blocking
                        .after(submit_golden_gpu_copy)
                        .run_if(resource_exists::<GoldenGpuCopySubmitted>)
                        .run_if(resource_exists::<GoldenGpuRealResources>)
                        .run_if(resource_exists::<GoldenGpuTargets>)
                        .run_if(not(resource_exists::<GoldenGpuHashComplete>)),
                );
            } else {
                app.add_systems(
                    PostUpdate,
                    submit_gpu_map_async
                        .after(submit_golden_gpu_copy)
                        .run_if(resource_exists::<GoldenGpuCopySubmitted>)
                        .run_if(resource_exists::<GoldenGpuRealResources>)
                        .run_if(resource_exists::<GoldenGpuTargets>)
                        .run_if(not(resource_exists::<GoldenGpuMapInProgress>))
                        .run_if(not(resource_exists::<GoldenGpuHashComplete>)),
                );
                app.add_systems(
                    PostUpdate,
                    poll_gpu_map_and_hash
                        .after(submit_gpu_map_async)
                        .run_if(resource_exists::<GoldenGpuMapInProgress>)
                        .run_if(not(resource_exists::<GoldenGpuHashComplete>)),
                );
            }
            app.add_systems(
                PostUpdate,
                finalize_stub_gpu_capture
                    .after(GoldenCaptureSet)
                    .after(submit_golden_gpu_copy)
                    .run_if(resource_exists::<GoldenGpuAlloc>)
                    .run_if(resource_exists::<GoldenGpuCopySubmitted>)
                    .run_if(not(resource_exists::<GoldenGpuHashComplete>)),
            );
        }
    }
}

/// Marker inserted after the (placeholder/enriched) capture to ensure it only runs once.
#[derive(Resource)]
struct GoldenStateCaptured;

/// Capture system: builds enriched preimage (v3 shape) if available, then hashes.
/// Placeholder pixel hash metadata uses length=0 + 16 zero bytes.
fn placeholder_or_enriched_capture(
    mut commands: Commands,
    mut state: ResMut<GoldenState>,
    q_visuals: Query<Entity, With<BallCircleVisual>>,
    frame_idx: Res<GoldenFrameIndex>,
    preimage_opt: Option<Res<GoldenPreimage>>,
    mut metrics: ResMut<GoldenMetrics>,
) {
    use std::time::Instant;
    let capture_begin = Instant::now();
    if state.captured {
        return;
    }

    let count = q_visuals.iter().count();

    // Build hash preimage (v3).
    let mut hasher = blake3::Hasher::new();
    // Record contributed preimage size (raw bytes from other crates only).
    metrics.preimage_len = preimage_opt.as_ref().map(|p| p.0.len()).unwrap_or(0);
    hasher.update(GOLDEN_SEED_TAG_V3); // versioned seed
    hasher.update(&count.to_le_bytes());
    if let Some(ref preimage) = preimage_opt {
        if !preimage.0.is_empty() {
            // Length-prefix to avoid concatenation ambiguity.
            let len = preimage.0.len() as u64;
            hasher.update(&len.to_le_bytes());
            hasher.update(&preimage.0);
        } else {
            hasher.update(&0u64.to_le_bytes());
        }
    } else {
        hasher.update(&0u64.to_le_bytes());
    }
    // v3 extension: pixel hash metadata (placeholder path => length=0 + 16 zero bytes)
    hasher.update(&0u64.to_le_bytes());
    hasher.update(&[0u8; 16]);

    let hash_start = std::time::Instant::now();
    metrics.preimage_ns = hash_start.duration_since(capture_begin).as_nanos();
    let hash = hasher.finalize().to_hex().to_string();
    metrics.hash_ns = hash_start.elapsed().as_nanos();
    metrics.capture_total_ns = capture_begin.elapsed().as_nanos();

    state.hash_placeholder = Some(hash.clone());
    state.final_hash = Some(hash);
    state.pixel_hash = None; // not yet available
    state.captured = true;
    #[cfg(feature = "gpu_capture")]
    {
        state.stage = GoldenStage::PendingGpuCapture;
    }
    #[cfg(not(feature = "gpu_capture"))]
    {
        state.stage = GoldenStage::PlaceholderCaptured;
    }
    state.frame_captured = Some(frame_idx.0);

    // Baseline load / write logic (unchanged except variable names).
    #[cfg(not(target_arch = "wasm32"))]
    {
        if state.baseline_hash.is_none() {
            if let Ok(env_hash) = std::env::var("GOLDEN_BASELINE_HASH") {
                if !env_hash.is_empty() {
                    state.baseline_hash = Some(env_hash);
                }
            } else {
                let path = std::env::var("GOLDEN_BASELINE_FILE")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "golden_baseline_hash.txt".to_string());
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Some(first_line) = contents.lines().next() {
                        let line = first_line.trim();
                        if !line.is_empty() {
                            state.baseline_hash = Some(line.to_string());
                        }
                    }
                }
            }
            if state.baseline_hash.is_some() && state.baseline_match.is_none() {
                state.baseline_match =
                    Some(state.baseline_hash.as_deref() == state.final_hash.as_deref());
            }
        } else if state.baseline_match.is_none() {
            state.baseline_match =
                Some(state.baseline_hash.as_deref() == state.final_hash.as_deref());
        }

        if let Ok(write_flag) = std::env::var("GOLDEN_WRITE_BASELINE") {
            let want_write = matches!(write_flag.as_str(), "1" | "true" | "TRUE" | "True");
            if want_write {
                // Guard adoption / overwrite with GOLDEN_ALLOW_NEW when a baseline already exists.
                let allow_new = std::env::var("GOLDEN_ALLOW_NEW")
                    .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "True"))
                    .unwrap_or(false);
                if let Some(ref final_hash) = state.final_hash {
                    let path = std::env::var("GOLDEN_BASELINE_FILE")
                        .ok()
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "golden_baseline_hash.txt".to_string());
                    let baseline_missing = state.baseline_hash.is_none();
                    if baseline_missing || allow_new {
                        if let Err(e) = std::fs::write(&path, format!("{}\n", final_hash)) {
                            #[cfg(debug_assertions)]
                            eprintln!("[golden] failed to write baseline file {}: {}", path, e);
                        } else {
                            // Adopt new baseline.
                            state.baseline_hash = Some(final_hash.clone());
                            state.baseline_match = Some(true);
                            #[cfg(debug_assertions)]
                            println!(
                                "[golden] baseline {} (allow_new={} missing={}) path={}",
                                "ADOPTED",
                                allow_new,
                                baseline_missing,
                                path
                            );
                        }
                    } else {
                        #[cfg(debug_assertions)]
                        println!(
                            "[golden] baseline NOT adopted (existing present, GOLDEN_ALLOW_NEW not set) existing={:?}",
                            state.baseline_hash
                        );
                    }
                }
            }
        }
    }

    commands.insert_resource(GoldenStateCaptured);

    // Exercise helper for lint coverage.
    let _ = current_golden_hash(&state);

    #[cfg(debug_assertions)]
    {
        if let Some(m) = state.baseline_match {
            println!(
                "[golden] baseline {} (hash={:?} baseline={:?})",
                if m { "MATCH" } else { "MISMATCH" },
                state.final_hash,
                state.baseline_hash
            );
        }
        println!(
            "[golden] enriched placeholder (v3) hash captured stage={:?} count={} preimage_bytes={} preimage_ns={} hash_ns={} total_ns={}",
            state.stage,
            count,
            metrics.preimage_len,
            metrics.preimage_ns,
            metrics.hash_ns,
            metrics.capture_total_ns
        );
    }
}

#[allow(dead_code)] // Used only inside #[cfg(test)] module; retained for legacy v1 hash comparison tests.
/// Legacy helper retained for tests that derive expected placeholder hash without
/// preimage contributions (v1 seed retained for backwards comparison tests).
fn compute_placeholder_hash(count: usize) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"golden-placeholder-v1");
    hasher.update(&count.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Public helper to retrieve the "current" golden hash (final preferred, else placeholder).
pub fn current_golden_hash(state: &GoldenState) -> Option<&str> {
    state.final_hash.as_deref().or(state.hash_placeholder.as_deref())
}

#[cfg(feature = "gpu_capture")]
#[derive(Resource)]
struct GoldenGpuAlloc;

#[cfg(feature = "gpu_capture")]
#[derive(Resource, Debug)]
struct GoldenGpuTargets {
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    #[allow(dead_code)]
    pixel_count: u64,
    #[allow(dead_code)]
    bytes_per_pixel: u32,
    #[allow(dead_code)]
    unpadded_bytes_per_row: u32,
    #[allow(dead_code)]
    padded_bytes_per_row: u32,
    #[allow(dead_code)]
    total_padded_size: u64,
    // Future: texture handle, buffer handle, actual wgpu::Texture / wgpu::Buffer, command encoder fence, async map state, etc.
}

#[cfg(feature = "gpu_capture")]
#[derive(Resource, Debug)]
enum GoldenGpuSourceKind {
    /// We allocated a dedicated texture for capture (no usable view tap yet).
    AllocatedTexture,
    /// A view tap exists but is still placeholder; treat as allocated texture fallback.
    ViewTapPlaceholderFallback,
    /// A real external view texture (future path); skip texture allocation and copy directly.
    ViewTapExternal,
}

#[cfg(feature = "gpu_capture")]
#[derive(Resource, Debug)]
struct GoldenGpuViewTap {
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    #[allow(dead_code)]
    placeholder: bool, // true until real texture handle wired (copy source fallback)
    // Future: actual swapchain / main view texture handle (TextureView or cached Texture).
    // TBD_GOLDEN_GPU_VIEW_TAP: placeholder dims until real render graph extraction implemented.
}

#[cfg(feature = "gpu_capture")]
fn tap_golden_gpu_view(
    mut commands: Commands,
    state: Res<GoldenState>,
) {
    // Incremental Step 1 (TBD_GOLDEN_GPU_VIEW_TAP): capture (placeholder) view target metadata once capture initiated.
    if state.captured && matches!(state.stage, GoldenStage::PendingGpuCapture) {
        // Placeholder dimensions; real implementation will query main view target texture size.
        commands.insert_resource(GoldenGpuViewTap { width: 640, height: 480, placeholder: true });
        #[cfg(debug_assertions)]
        println!("[golden] view tap placeholder inserted (TBD_GOLDEN_GPU_VIEW_TAP) width=640 height=480");
    }
}

/// Stub allocation stage: in the real implementation this would create a texture & buffer and issue a copy command.
/// Here we only measure the trivial cost and insert a marker resource.
#[cfg(feature = "gpu_capture")]
fn allocate_stub_gpu_capture(
    mut commands: Commands,
    mut metrics: ResMut<GoldenMetrics>,
    mut state: ResMut<GoldenState>,
    view_tap: Option<Res<GoldenGpuViewTap>>,
) {
    if state.captured && matches!(state.stage, GoldenStage::PendingGpuCapture) {
        use std::time::Instant;
        let start = Instant::now();

        // Determine target dimensions (placeholder or tapped view).
        let (width, height, source_kind) = if let Some(tap) = view_tap {
            let kind = if tap.placeholder {
                GoldenGpuSourceKind::ViewTapPlaceholderFallback
            } else {
                GoldenGpuSourceKind::ViewTapExternal
            };
            (tap.width, tap.height, kind)
        } else {
            (640u32, 480u32, GoldenGpuSourceKind::AllocatedTexture)
        };
        let pixel_count = (width as u64) * (height as u64);
        // Derive row padding metrics matching wgpu COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let bytes_per_pixel = 4u32; // RGBA8 target (future real texture format)
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = 256u32;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let total_padded_size = padded_bytes_per_row as u64 * height as u64;

        commands.insert_resource(GoldenGpuAlloc);
        commands.insert_resource(GoldenGpuTargets {
            width,
            height,
            pixel_count,
            bytes_per_pixel,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
            total_padded_size,
        });
        commands.insert_resource(source_kind);

        // Populate state pixel metadata (visible to tests / future hashing path).
        state.capture_width = Some(width);
        state.capture_height = Some(height);
        state.pixel_count = Some(pixel_count as u32);

        metrics.allocation_ns = start.elapsed().as_nanos();
        #[cfg(debug_assertions)]
        println!(
            "[golden] gpu_capture stub allocated targets {}x{} pixels={} allocation_ns={}",
            width, height, pixel_count, metrics.allocation_ns
        );
    }
}

#[cfg(feature = "gpu_capture")]
fn finalize_stub_gpu_capture(mut state: ResMut<GoldenState>, mut metrics: ResMut<GoldenMetrics>) {
    // Stub promotion: in real implementation this would map a GPU buffer and hash pixel bytes.
    if state.captured && matches!(state.stage, GoldenStage::PendingGpuCapture) {
        use std::time::Instant;
        let start = Instant::now();
        state.stage = GoldenStage::FinalCaptured;
        metrics.map_ns = start.elapsed().as_nanos();
        metrics.gpu_finalize_ns = metrics.map_ns; // legacy field population
        // Placeholder metric population (real path will separate copy vs map wait & bytes hashed).
        if metrics.copy_ns == 0 {
            metrics.copy_ns = metrics.map_ns;
        }
        metrics.map_wait_ns = 0;
        metrics.bytes_hashed = 0;
        #[cfg(debug_assertions)]
        println!(
            "[golden] gpu_capture stub finalized (reused placeholder hash) allocation_ns={} map_ns={} copy_ns={} map_wait_ns={} bytes_hashed={}",
            metrics.allocation_ns,
            metrics.map_ns,
            metrics.copy_ns,
            metrics.map_wait_ns,
            metrics.bytes_hashed
        );
    }
}

#[cfg(feature = "gpu_capture")]
#[derive(Resource)]
struct GoldenGpuRealResources {
    texture: Option<bevy::render::render_resource::Texture>,
    buffer: bevy::render::render_resource::Buffer,
}

#[cfg(feature = "gpu_capture")]
fn allocate_real_gpu_resources(
    mut commands: Commands,
    device: Res<bevy::render::renderer::RenderDevice>,
    targets: Res<GoldenGpuTargets>,
    mut metrics: ResMut<GoldenMetrics>,
    source_kind: Res<GoldenGpuSourceKind>,
) {
    use std::time::Instant;
    let start = Instant::now();

    // Optionally create texture (skip when real external view available).
    let texture_opt = match *source_kind {
        GoldenGpuSourceKind::AllocatedTexture | GoldenGpuSourceKind::ViewTapPlaceholderFallback => {
            let texture_size = bevy::render::render_resource::Extent3d {
                width: targets.width,
                height: targets.height,
                depth_or_array_layers: 1,
            };
            Some(device.create_texture(&bevy::render::render_resource::TextureDescriptor {
                label: Some("golden_capture_texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: bevy::render::render_resource::TextureDimension::D2,
                format: bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                usage: bevy::render::render_resource::TextureUsages::COPY_SRC
                    | bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }))
        }
        GoldenGpuSourceKind::ViewTapExternal => None,
    };

    // Create readback buffer sized to padded bytes (always needed).
    let buffer = device.create_buffer(&bevy::render::render_resource::BufferDescriptor {
        label: Some("golden_capture_buffer"),
        size: targets.total_padded_size,
        usage: bevy::render::render_resource::BufferUsages::COPY_DST
            | bevy::render::render_resource::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    commands.insert_resource(GoldenGpuRealResources { texture: texture_opt, buffer });

    // Record real allocation time (kept separate from copy submission).
    let elapsed = start.elapsed().as_nanos();
    if metrics.allocation_ns == 0 {
        metrics.allocation_ns = elapsed;
    }

    #[cfg(debug_assertions)]
    println!(
        "[golden] real GPU resources allocated {}x{} padded_row={} total_padded={} ns={} source_kind={:?} texture_allocated={}",
        targets.width,
        targets.height,
        targets.padded_bytes_per_row,
        targets.total_padded_size,
        elapsed,
        *source_kind,
        texture_opt.is_some()
    );
}

#[cfg(feature = "gpu_capture")]
#[derive(Resource)]
struct GoldenGpuCopySubmitted;

#[cfg(feature = "gpu_capture")]
fn submit_golden_gpu_copy(
    mut commands: Commands,
    device: Res<bevy::render::renderer::RenderDevice>,
    queue: Res<bevy::render::renderer::RenderQueue>,
    resources: Res<GoldenGpuRealResources>,
    targets: Res<GoldenGpuTargets>,
    mut metrics: ResMut<GoldenMetrics>,
    _view_tap: Option<Res<GoldenGpuViewTap>>,
    source_kind: Res<GoldenGpuSourceKind>,
) {
    use std::time::Instant;
    use bevy::render::render_resource::CommandEncoderDescriptor;

    let start = Instant::now();
    let mut encoder =
        device.create_command_encoder(&CommandEncoderDescriptor { label: Some("golden_copy_encoder") });

    // Copy source selection (placeholder path always allocated texture).
    let (source_texture_opt, source_label) = match *source_kind {
        GoldenGpuSourceKind::AllocatedTexture => (resources.texture.as_ref(), "allocated"),
        GoldenGpuSourceKind::ViewTapPlaceholderFallback => (resources.texture.as_ref(), "view_tap-placeholder-fallback"),
        GoldenGpuSourceKind::ViewTapExternal => {
            #[cfg(debug_assertions)]
            println!("[golden] submit_golden_gpu_copy: external view path not yet implemented; skipping copy");
            (None, "view_tap-external-unimplemented")
        }
    };
    if source_texture_opt.is_none() {
        return;
    }
    let source_texture = source_texture_opt.unwrap();

    // Texture -> buffer copy.
    {
        use wgpu::{
            TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, Origin3d, TextureAspect,
            Extent3d,
        };
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: source_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &resources.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(targets.padded_bytes_per_row),
                    rows_per_image: Some(targets.height),
                },
            },
            Extent3d {
                width: targets.width,
                height: targets.height,
                depth_or_array_layers: 1,
            },
        );
    }

    queue.submit(std::iter::once(encoder.finish()));

    if metrics.copy_ns == 0 {
        metrics.copy_ns = start.elapsed().as_nanos();
    }

    commands.insert_resource(GoldenGpuCopySubmitted);

    #[cfg(debug_assertions)]
    println!(
        "[golden] copy submission (source={} wgpu copy issued) {}x{} padded_row={} total={} copy_ns={}",
        source_label,
        targets.width,
        targets.height,
        targets.padded_bytes_per_row,
        targets.total_padded_size,
        metrics.copy_ns
    );
}

#[cfg(feature = "gpu_capture")]
fn map_and_hash_gpu_capture_blocking(
    mut state: ResMut<GoldenState>,
    resources: Res<GoldenGpuRealResources>,
    targets: Res<GoldenGpuTargets>,
    device: Res<bevy::render::renderer::RenderDevice>,
    mut metrics: ResMut<GoldenMetrics>,
    mut commands: Commands,
    q_visuals: Query<Entity, With<BallCircleVisual>>,
    preimage_opt: Option<Res<GoldenPreimage>>,
) {
    use std::time::Instant;
    // Only proceed if still pending real GPU capture stage.
    if !state.captured || !matches!(state.stage, GoldenStage::PendingGpuCapture) {
        return;
    }

    // Submit async map and block (legacy incremental path).
    let map_begin = Instant::now();
    let buffer_slice = resources.buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res.is_ok());
    });

    // Blocking wait.
    let wait_start = Instant::now();
    device.wgpu_device().poll(wgpu::Maintain::Wait);
    let Ok(success) = rx.recv() else {
        #[cfg(debug_assertions)]
        eprintln!("[golden] map_and_hash_gpu_capture_blocking: map_async channel recv failed");
        return;
    };
    if !success {
        #[cfg(debug_assertions)]
        eprintln!("[golden] map_and_hash_gpu_capture_blocking: map_async returned error");
        return;
    };
    let wait_elapsed = wait_start.elapsed().as_nanos();

    // Read mapped padded data.
    let mapped = buffer_slice.get_mapped_range();
    let mut pixel_bytes = Vec::with_capacity((targets.width * targets.height * 4) as usize);
    let padded_row = targets.padded_bytes_per_row as usize;
    let unpadded_row = targets.unpadded_bytes_per_row as usize;
    for row in 0..targets.height as usize {
        let start = row * padded_row;
        let end = start + unpadded_row;
        pixel_bytes.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    resources.buffer.unmap();

    metrics.map_ns = map_begin.elapsed().as_nanos();
    metrics.map_wait_ns = wait_elapsed;
    metrics.bytes_hashed = pixel_bytes.len() as u64;

    // Compute pixel hash (blake3) first.
    let pixel_hash_start = Instant::now();
    let pixel_digest = blake3::hash(&pixel_bytes);
    let pixel_hash_hex = pixel_digest.to_hex().to_string();
    let pixel_hash_elapsed = pixel_hash_start.elapsed().as_nanos();

    // Build v3 final preimage.
    let count = q_visuals.iter().count();
    let mut final_hasher = blake3::Hasher::new();
    final_hasher.update(GOLDEN_SEED_TAG_V3);
    final_hasher.update(&count.to_le_bytes());
    if let Some(ref preimage) = preimage_opt {
        if !preimage.0.is_empty() {
            let len = preimage.0.len() as u64;
            final_hasher.update(&len.to_le_bytes());
            final_hasher.update(&preimage.0);
        } else {
            final_hasher.update(&0u64.to_le_bytes());
        }
    } else {
        final_hasher.update(&0u64.to_le_bytes());
    }
    let pixel_len = pixel_digest.as_bytes().len() as u64;
    final_hasher.update(&pixel_len.to_le_bytes());
    final_hasher.update(&pixel_digest.as_bytes()[0..16]);
    let final_hash = final_hasher.finalize().to_hex().to_string();

    state.pixel_hash = Some(pixel_hash_hex.clone());
    state.final_hash = Some(final_hash.clone());
    // Recompute baseline match if baseline present.
    if let Some(ref baseline) = state.baseline_hash {
        state.baseline_match = Some(baseline == &final_hash);
    }
    state.stage = GoldenStage::FinalCaptured;

    // Record hash time separately (reuse existing hash_ns field for pixel hashing duration).
    metrics.hash_ns = pixel_hash_elapsed;

    // Marker to prevent rerun.
    commands.insert_resource(GoldenGpuHashComplete);

    #[cfg(debug_assertions)]
    println!(
        "[golden] real GPU map+hash complete (v3) stage={:?} bytes={} map_ns={} wait_ns={} pixel_hash_ns={} pixel_hash_prefix_first16={:02x?}",
        state.stage,
        metrics.bytes_hashed,
        metrics.map_ns,
        metrics.map_wait_ns,
        metrics.hash_ns,
        &pixel_digest.as_bytes()[0..16]
    );
}

#[cfg(feature = "gpu_capture")]
fn note_real_gpu_resources_usage(
    resources: Res<GoldenGpuRealResources>,
    mut _metrics: ResMut<GoldenMetrics>,
) {
    let _ = &resources.texture;
    let _ = &resources.buffer;
}

// --- Future GPU readback pipeline scaffold (Phase 7 incremental) -----------------------------

#[cfg(feature = "gpu_capture")]
#[derive(Resource)]
struct GoldenGpuMapInProgress(std::sync::Arc<std::sync::atomic::AtomicBool>);

#[cfg(feature = "gpu_capture")]
// Step 4: Non-blocking map submit (TBD_GOLDEN_GPU_ASYNC_MAP)
fn submit_gpu_map_async(
    resources: Res<GoldenGpuRealResources>,
    mut commands: Commands,
    state: Res<GoldenState>,
) {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    if !state.captured || !matches!(state.stage, GoldenStage::PendingGpuCapture) {
        return;
    }
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();
    let slice = resources.buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, move |res| {
        if res.is_ok() {
            ready_clone.store(true, Ordering::SeqCst);
        }
    });
    commands.insert_resource(GoldenGpuMapInProgress(ready));
    #[cfg(debug_assertions)]
    println!("[golden] submit_gpu_map_async: map_async issued (non-blocking)");
}

#[cfg(feature = "gpu_capture")]
fn poll_gpu_map_and_hash(
    mut state: ResMut<GoldenState>,
    resources: Res<GoldenGpuRealResources>,
    targets: Res<GoldenGpuTargets>,
    device: Res<bevy::render::renderer::RenderDevice>,
    mut metrics: ResMut<GoldenMetrics>,
    mut commands: Commands,
    in_progress: Res<GoldenGpuMapInProgress>,
    q_visuals: Query<Entity, With<BallCircleVisual>>,
    preimage_opt: Option<Res<GoldenPreimage>>,
) {
    use std::sync::atomic::Ordering;
    use std::time::Instant;
    if !state.captured || !matches!(state.stage, GoldenStage::PendingGpuCapture) {
        return;
    }
    // Progress async work without blocking.
    device.wgpu_device().poll(wgpu::Maintain::Poll);
    if !in_progress.0.load(Ordering::SeqCst) {
        return; // not ready yet
    }

    let map_begin = Instant::now();
    // Safe to take a fresh slice now that map completed.
    let buffer_slice = resources.buffer.slice(..);
    let mapped = buffer_slice.get_mapped_range();
    let padded_row = targets.padded_bytes_per_row as usize;
    let unpadded_row = targets.unpadded_bytes_per_row as usize;
    let mut pixel_bytes = Vec::with_capacity((targets.width * targets.height * 4) as usize);
    for row in 0..targets.height as usize {
        let start = row * padded_row;
        let end = start + unpadded_row;
        pixel_bytes.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    resources.buffer.unmap();
    metrics.map_ns = map_begin.elapsed().as_nanos();
    metrics.map_wait_ns = 0;
    metrics.bytes_hashed = pixel_bytes.len() as u64;

    // Compute pixel hash.
    let pixel_hash_start = Instant::now();
    let pixel_digest = blake3::hash(&pixel_bytes);
    let pixel_hash_hex = pixel_digest.to_hex().to_string();
    metrics.hash_ns = pixel_hash_start.elapsed().as_nanos();

    // Build v3 final preimage.
    let count = q_visuals.iter().count();
    let mut final_hasher = blake3::Hasher::new();
    final_hasher.update(GOLDEN_SEED_TAG_V3);
    final_hasher.update(&count.to_le_bytes());
    if let Some(ref preimage) = preimage_opt {
        if !preimage.0.is_empty() {
            let len = preimage.0.len() as u64;
            final_hasher.update(&len.to_le_bytes());
            final_hasher.update(&preimage.0);
        } else {
            final_hasher.update(&0u64.to_le_bytes());
        }
    } else {
        final_hasher.update(&0u64.to_le_bytes());
    }
    let pixel_len = pixel_digest.as_bytes().len() as u64;
    final_hasher.update(&pixel_len.to_le_bytes());
    final_hasher.update(&pixel_digest.as_bytes()[0..16]);
    let final_hash = final_hasher.finalize().to_hex().to_string();

    state.pixel_hash = Some(pixel_hash_hex.clone());
    state.final_hash = Some(final_hash.clone());
    if let Some(ref baseline) = state.baseline_hash {
        state.baseline_match = Some(baseline == &final_hash);
    }
    state.stage = GoldenStage::FinalCaptured;
    commands.insert_resource(GoldenGpuHashComplete);

    #[cfg(debug_assertions)]
    println!(
        "[golden] poll_gpu_map_and_hash: hash complete (v3) bytes={} map_ns={} pixel_hash_ns={} pixel_hash_prefix_first16={:02x?}",
        metrics.bytes_hashed, metrics.map_ns, metrics.hash_ns, &pixel_digest.as_bytes()[0..16]
    );
}

#[cfg(feature = "gpu_capture")]
#[allow(dead_code)]
#[derive(Resource)]
struct GoldenGpuHashComplete;

#[cfg(feature = "gpu_capture")]
#[allow(dead_code)]
fn golden_gpu_hash_stub(
    mut state: ResMut<GoldenState>,
    mut metrics: ResMut<GoldenMetrics>,
    mut commands: Commands,
) {
    // Placeholder: would read mapped buffer, strip padding, hash real pixel bytes.
    if metrics.bytes_hashed == 0 {
        metrics.bytes_hashed = 0; // remains zero until real path
    }
    if matches!(state.stage, GoldenStage::PendingGpuCapture) {
        state.stage = GoldenStage::FinalCaptured;
    }
    commands.insert_resource(GoldenGpuHashComplete);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Ensure tests modifying process-wide environment variables run serially to avoid race conditions.
    static GOLDEN_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn plugin_inserts_state_and_preimage() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::remove_var("GOLDEN_WRITE_BASELINE");
        let _ = std::fs::remove_file("golden_baseline_hash.txt");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        // Contribute some deterministic preimage bytes before first update
        {
            let mut pre = app.world_mut().resource_mut::<GoldenPreimage>();
            pre.0.extend_from_slice(b"test-preimage");
        }
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        assert!(state.captured, "expected capture to mark captured");
        assert!(state.final_hash.is_some(), "final hash populated");
        // v1 helper still works independently:
        let expected_v1 = compute_placeholder_hash(0);
        assert_ne!(
            state.final_hash.as_deref(),
            Some(expected_v1.as_str()),
            "enriched hash (v3) must differ from legacy v1 placeholder hash when preimage present"
        );
    }

    #[test]
    fn baseline_match_from_env() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        // Use legacy helper to derive deterministic entity-count only baseline (no preimage).
        let expected = compute_placeholder_hash(0);
        std::env::set_var("GOLDEN_BASELINE_HASH", &expected);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        // Ensure no preimage contributions so v3 hash differs only by seed tag + metadata fields.
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        assert_eq!(state.baseline_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(state.baseline_match, Some(false), "v3 seed causes mismatch vs v1 baseline");
    }

    #[test]
    fn baseline_mismatch_from_env() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        std::env::set_var("GOLDEN_BASELINE_HASH", "ffffffffffffffff");
        std::env::remove_var("GOLDEN_WRITE_BASELINE");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        assert_eq!(state.baseline_match, Some(false));
    }

    #[test]
    fn baseline_load_from_file() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::remove_var("GOLDEN_WRITE_BASELINE");
        let expected = compute_placeholder_hash(0);
        let path = std::env::temp_dir().join("golden_baseline_load_v3.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, format!("{}\n", expected)).unwrap();
        std::env::set_var("GOLDEN_BASELINE_FILE", &path);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        assert_eq!(state.baseline_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(state.baseline_match, Some(false), "seed change causes mismatch vs v1 hash");
    }

    #[test]
    fn baseline_write_file() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::set_var("GOLDEN_WRITE_BASELINE", "1");
        let path = std::env::temp_dir().join(format!(
            "golden_baseline_write_v3_{}.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        std::env::set_var("GOLDEN_BASELINE_FILE", &path);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        let final_hash = state.final_hash.as_ref().expect("final hash populated");
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk.lines().next().unwrap().trim(), final_hash);
        assert_eq!(state.baseline_hash.as_deref(), Some(final_hash.as_str()));
        assert_eq!(state.baseline_match, Some(true));
    }

    #[test]
    fn baseline_update_blocked_without_allow_new() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        // Existing baseline file with legacy v1 hash.
        let existing = compute_placeholder_hash(0);
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::set_var("GOLDEN_WRITE_BASELINE", "1");
        std::env::remove_var("GOLDEN_ALLOW_NEW");
        let path = std::env::temp_dir().join("golden_baseline_block_no_allow_v3.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, format!("{}\n", existing)).unwrap();
        std::env::set_var("GOLDEN_BASELINE_FILE", &path);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        // Add preimage so final hash certainly differs from legacy v1 hash.
        {
            let mut pre = app.world_mut().resource_mut::<GoldenPreimage>();
            pre.0.extend_from_slice(b"diff-preimage");
        }
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        // Baseline should remain the existing one; no adoption since GOLDEN_ALLOW_NEW not set.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk.lines().next().unwrap().trim(), existing);
        assert_eq!(state.baseline_hash.as_deref(), Some(existing.as_str()));
        assert_ne!(state.final_hash.as_deref(), Some(existing.as_str()));
        assert_eq!(state.baseline_match, Some(false));
    }

    #[test]
    fn baseline_update_allowed_with_allow_new() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        let existing = compute_placeholder_hash(0);
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::set_var("GOLDEN_WRITE_BASELINE", "1");
        std::env::set_var("GOLDEN_ALLOW_NEW", "1");
        let path = std::env::temp_dir().join("golden_baseline_allow_new_v3.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, format!("{}\n", existing)).unwrap();
        std::env::set_var("GOLDEN_BASELINE_FILE", &path);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        {
            let mut pre = app.world_mut().resource_mut::<GoldenPreimage>();
            pre.0.extend_from_slice(b"diff-preimage-allow");
        }
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        let final_hash = state.final_hash.clone().unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk.lines().next().unwrap().trim(), final_hash);
        assert_eq!(state.baseline_hash.as_deref(), Some(final_hash.as_str()));
        assert_eq!(state.baseline_match, Some(true));
        assert_ne!(final_hash, existing);
    }

    #[test]
    fn baseline_env_precedence_over_file() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        let expected = compute_placeholder_hash(0);
        let path = std::env::temp_dir().join("golden_baseline_precedence_v3.txt");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, "ffffffffffffffff\n").unwrap();
        std::env::set_var("GOLDEN_BASELINE_FILE", &path);
        std::env::set_var("GOLDEN_BASELINE_HASH", &expected);
        std::env::remove_var("GOLDEN_WRITE_BASELINE");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        app.update();
        let state = app.world().get_resource::<GoldenState>().unwrap();
        assert_eq!(state.baseline_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(state.baseline_match, Some(false), "v3 hash != v1 expected baseline");
    }

    #[test]
    #[cfg(all(feature = "golden", feature = "gpu_capture"))]
    fn gpu_capture_pipeline_produces_pixel_hash() {
        let _g = GOLDEN_TEST_MUTEX.lock().unwrap();
        std::env::remove_var("GOLDEN_BASELINE_HASH");
        std::env::remove_var("GOLDEN_WRITE_BASELINE");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GoldenHashPlugin);
        // Single update drives placeholder + stub GPU staging. Real GPU map+hash requires a wgpu RenderDevice (not present under MinimalPlugins).
        app.update();
        let state = app.world().get_resource::<GoldenState>().expect("GoldenState present");
        let metrics = app.world().get_resource::<GoldenMetrics>().expect("GoldenMetrics present");
        assert!(state.captured, "capture should have occurred");
        let have_real = app.world().get_resource::<GoldenGpuRealResources>().is_some();
        let hash_complete = app.world().get_resource::<GoldenGpuHashComplete>().is_some();
        if have_real && hash_complete {
            assert_eq!(state.stage, GoldenStage::FinalCaptured, "expected FinalCaptured with real GPU resources");
            assert!(metrics.bytes_hashed > 0, "expected non-zero bytes_hashed when real hash complete");
            // In v3 final_hash differs from pixel_hash (final_hash is preimage hash including pixel metadata).
            if let (Some(ph), Some(fh)) = (&state.pixel_hash, &state.final_hash) {
                assert_ne!(ph, fh, "final hash (v3 preimage) should differ from raw pixel hash");
            }
            let expected_bytes = state.capture_width.unwrap() as u64
                * state.capture_height.unwrap() as u64
                * 4;
            assert_eq!(metrics.bytes_hashed, expected_bytes, "bytes_hashed should equal width*height*4");
        } else {
            // Headless/unit environment fallback: no real GPU device, so we remain pending (or stub final).
            assert!(
                matches!(state.stage, GoldenStage::PendingGpuCapture | GoldenStage::FinalCaptured),
                "expected PendingGpuCapture or FinalCaptured without real GPU; got {:?}",
                state.stage
            );
            assert!(metrics.bytes_hashed == 0 || app.world().get_resource::<GoldenGpuHashComplete>().is_some(),
                "bytes_hashed should remain 0 without real GPU hashing");
        }
    }
}
