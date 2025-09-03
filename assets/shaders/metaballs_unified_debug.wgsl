// DEBUG Metaballs Unified Shader
// -----------------------------------------------------------------------------
// Purpose: Minimal diagnostic variant that preserves the binding / interface
// contract of the production `metaballs_unified.wgsl` while rendering a solid
// red output. This helps isolate pipeline / texture / uniform layout issues
// (e.g. storage format capability failures) from higher level logic.
//
// Keep: identical @group/@binding layout and entry point signatures so that the
// existing Rust material / bind group layout remains valid.
// -----------------------------------------------------------------------------

// Mirror production layout (storage palette only)
// v0: (ball_count_exposed, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, fg_mode, bg_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)
// v4: (enable_early_exit, needs_gradient, metadata_v2_flag, enable_adaptive_mask)
// v5: (reserved0, cluster_color_count, reserved1, reserved2)
struct MetaballsData {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    v5: vec4<f32>,
};
@group(2) @binding(0) var<uniform> metaballs: MetaballsData;

// Noise + surface noise uniform placeholders (layouts preserved)
struct NoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<u32> };
@group(2) @binding(1)
var<uniform> noise_params: NoiseParamsStd140;

struct SurfaceNoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<f32>, v3: vec4<u32> };
@group(2) @binding(2) var<uniform> surface_noise: SurfaceNoiseParamsStd140;

// Storage buffers (not used in debug path, but declared for layout compatibility)
struct GpuBall { data: vec4<f32> };
struct TileHeader { offset: u32, count: u32, _pad0: u32, _pad1: u32 };
@group(2) @binding(3) var<storage, read> balls: array<GpuBall>;
@group(2) @binding(4) var<storage, read> tile_headers: array<TileHeader>;
@group(2) @binding(5) var<storage, read> tile_ball_indices: array<u32>;
struct ClusterColor { value: vec4<f32> };
@group(2) @binding(6) var<storage, read> cluster_palette: array<ClusterColor>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       world_pos: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    let half_size = metaballs.v2.xy * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

@fragment
fn fragment(_in: VertexOutput) -> @location(0) vec4<f32> {
    // Minimal diagnostic: render solid color modulated by early-exit & metadata flags.
    let ee = metaballs.v4.x;
    let grad = metaballs.v4.y;
    let meta_v2 = metaballs.v4.z;
    // Encode flags into RGB for quick visual verification.
    return vec4<f32>(ee, grad * 0.5, meta_v2, 1.0);
}
