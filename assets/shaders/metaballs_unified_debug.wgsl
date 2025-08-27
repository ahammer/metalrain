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

const MAX_BALLS    : u32 = 1024u;
const MAX_CLUSTERS : u32 =  256u;

struct MetaballsData {
    v0: vec4<f32>, // (ball_count, cluster_color_count, radius_scale, iso)
    v1: vec4<f32>, // (normal_z_scale, fg_mode, bg_mode, debug_view)
    v2: vec4<f32>, // (viewport_w, viewport_h, time_seconds, radius_multiplier)
    balls:          array<vec4<f32>, MAX_BALLS>,
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>,
};
@group(2) @binding(0)
var<uniform> metaballs: MetaballsData;

// Noise + surface noise uniform placeholders (layouts preserved)
struct NoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<u32> };
@group(2) @binding(1)
var<uniform> noise_params: NoiseParamsStd140;

struct SurfaceNoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<f32>, v3: vec4<u32> };
@group(2) @binding(2)
var<uniform> surface_noise: SurfaceNoiseParamsStd140;

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
    // Solid red diagnostic. Alpha 1.0 to make sure compositing visibly succeeds.
    // If this shows up the pipeline / bind group layouts are correct and the
    // black screen issue originates elsewhere (e.g., upstream pass failure or
    // unsupported texture usage in another node).
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
