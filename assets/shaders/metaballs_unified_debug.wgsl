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

// Basic metaball rendering with cluster coloring (dominant cluster only).
// Simplified from production: keeps same kernel & cluster selection logic but
// omits background modes, bevel lighting, surface noise, outlines.
const K_MAX : u32 = 12u; // small working set for dominant cluster selection

struct AccumEntry {
    used: u32,
    indices: array<u32, K_MAX>,
    field: array<f32, K_MAX>,
    grad: array<vec2<f32>, K_MAX>,
};

fn accumulate(p: vec2<f32>, ball_count: u32, radius_scale: f32, radius_multiplier: f32) -> AccumEntry {
    var acc: AccumEntry;
    acc.used = 0u;
    for (var k: u32 = 0u; k < K_MAX; k = k + 1u) {
        acc.indices[k] = 0u;
        acc.field[k] = 0.0;
        acc.grad[k] = vec2<f32>(0.0,0.0);
    }
    for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
        let r = b.z * radius_multiplier;
        if (r <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d,d);
        let scaled_r = r * radius_scale;
        let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2;
            let x2 = x * x;
            let fi = x2 * x; // (1 - d^2/r^2)^3
            let g = (-6.0 / r2) * d * x2; // gradient contribution
            let cluster = u32(b.w + 0.5);
            var found: i32 = -1;
            for (var k: u32 = 0u; k < acc.used; k = k + 1u) {
                if (acc.indices[k] == cluster) { found = i32(k); break; }
            }
            if (found >= 0) {
                let idx = u32(found);
                acc.field[idx] = acc.field[idx] + fi;
                acc.grad[idx] = acc.grad[idx] + g;
            } else if (acc.used < K_MAX) {
                acc.indices[acc.used] = cluster;
                acc.field[acc.used] = fi;
                acc.grad[acc.used] = g;
                acc.used = acc.used + 1u;
            } else {
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var kk: u32 = 0u; kk < acc.used; kk = kk + 1u) {
                    if (acc.field[kk] < smallest) { smallest = acc.field[kk]; smallest_i = kk; }
                }
                if (fi > smallest) {
                    acc.indices[smallest_i] = cluster;
                    acc.field[smallest_i] = fi;
                    acc.grad[smallest_i] = g;
                }
            }
        }
    }
    return acc;
}

fn dominant(acc: AccumEntry) -> u32 {
    var best: u32 = 0u;
    var best_f: f32 = acc.field[0u];
    for (var k: u32 = 1u; k < acc.used; k = k + 1u) {
        if (acc.field[k] > best_f) { best_f = acc.field[k]; best = k; }
    }
    return best;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count        = u32(metaballs.v0.x + 0.5);
    let cluster_color_cnt = u32(metaballs.v0.y + 0.5);
    let radius_scale      = metaballs.v0.z;
    let iso               = metaballs.v0.w;
    let radius_multiplier = metaballs.v2.w;
    let p = in.world_pos;

    if (ball_count == 0u || cluster_color_cnt == 0u) {
        return vec4<f32>(0.05, 0.05, 0.07, 1.0);
    }

    let acc = accumulate(p, ball_count, radius_scale, radius_multiplier);
    if (acc.used == 0u) {
        return vec4<f32>(0.05, 0.05, 0.07, 1.0);
    }
    // Determine dominant cluster (for color) but also accumulate total field
    let dom = dominant(acc);
    let cluster = acc.indices[dom];
    var total_field: f32 = 0.0;
    for (var k: u32 = 0u; k < acc.used; k = k + 1u) { total_field = total_field + acc.field[k]; }
    // Use gradient of dominant for edge smoothing (fallback if tiny)
    let grad = acc.grad[dom];
    var base_col = vec3<f32>(1.0,0.0,1.0); // fallback magenta if out-of-range
    if (cluster < cluster_color_cnt) {
        base_col = metaballs.cluster_colors[cluster].rgb;
    }
    // Derivative-free alpha: smoother across platforms (WebGPU WASM sometimes had derivative issues / extreme values).
    // Use a two-threshold smoothstep: start ramp earlier than iso for feather.
    let ramp_start = iso * 0.6; // earlier ramp makes blobs fatter in debug (easier to see)
    let mask = smoothstep(ramp_start, iso, total_field);
    return vec4<f32>(base_col, mask);
}
