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

// Simple field visualization:
// Accumulates the same kernel used in production ( (1 - d^2/r^2)^3 ) and then
// visualizes (field - iso). Inside = warm, outside = cool, iso ≈ near neutral.
// This ignores clustering & gradient lighting for clarity.
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count        = u32(metaballs.v0.x + 0.5);
    let radius_scale      = metaballs.v0.z;      // derived in Rust
    let iso               = metaballs.v0.w;      // threshold
    let radius_multiplier = metaballs.v2.w;      // per-frame multiplier
    let p = in.world_pos;

    if (ball_count == 0u) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    var field: f32 = 0.0;
    for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
        let r = b.z * radius_multiplier;
        if (r <= 0.0) { continue; }
        let scaled_r = r * radius_scale;
        let d = p - center;
        let d2 = dot(d, d);
        let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2;
            let fi = x * x * x; // (1 - d^2/r^2)^3
            field = field + fi;
        }
    }

    let is_signed = field - iso; // positive inside
    let norm = clamp(abs(is_signed) / (iso + 1e-5), 0.0, 1.0);
    // Bi-color ramp: inside -> reds, outside -> blues, fade to white near iso
    var col: vec3<f32>;
    if (is_signed >= 0.0) {
        col = mix(vec3<f32>(1.0, 0.95, 0.9), vec3<f32>(0.95, 0.15, 0.05), norm);
    } else {
        col = mix(vec3<f32>(0.9, 0.95, 1.0), vec3<f32>(0.1, 0.3, 0.9), norm);
    }
    // Encode alpha as a soft mask approximation for quick visual check
    let mask = clamp(field / (iso + iso), 0.0, 1.0);
    return vec4<f32>(col, mask);
}
