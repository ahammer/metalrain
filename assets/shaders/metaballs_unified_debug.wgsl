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
    let bg_mode           = u32(metaballs.v1.z + 0.5); // reuse production layout index
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
    // ---------------------------------------------------------------------
    // ALPHA / VISIBILITY NOTE (Debug Fix):
    // The earlier revision used screen-space derivatives (dpdx/dpdy) combined
    // with the gradient magnitude to build an anti-aliased transition width.
    // That worked on native (desktop) but produced an all‑black output on
    // certain WASM / WebGPU paths (observed: mask evaluated ~0 everywhere).
    // Potential causes (implementation dependent):
    //   * Derivative precision / undefined behavior inside dynamic control
    //     flow on some drivers leading to zero or NaN gradient lengths.
    //   * Downlevel / adapter quirk causing large or invalid values that
    //     push the computed mask outside [0,1] before clamp, collapsing alpha.
    //   * Timing of derivative availability for a fullscreen triangle in the
    //     pipeline with our loops (driver specific optimization).
    // For a robust diagnostic view we replace that logic with a derivative‑
    // free formulation: a simple smoothstep against the TOTAL scalar field.
    // This guarantees consistent visibility across native & WASM and keeps
    // the debug shader minimal (no reliance on implicit derivative hardware).
    // If you need edge thickness control, you can reintroduce derivatives
    // once the root cause is isolated, preferably behind a config toggle.
    // ---------------------------------------------------------------------
    // Use a two-threshold smoothstep: start ramp earlier than iso for feather.
    let ramp_start = iso * 0.6; // earlier ramp makes blobs fatter in debug (easier to see)
    let mask = smoothstep(ramp_start, iso, total_field);
    // Background color selection (simple subset of production logic)
    // Modes (debug subset):
    // 0: dark neutral
    // 1: vertical gradient
    // 2: radial gradient
    // 3: cluster hint noise (hash based)
    var bg = vec3<f32>(0.05, 0.05, 0.07);
    if (bg_mode == 1u) {
        let t = 0.5 + 0.5 * (p.y / (metaballs.v2.y * 0.5));
        bg = mix(vec3<f32>(0.06,0.07,0.10), vec3<f32>(0.12,0.14,0.19), clamp(t,0.0,1.0));
    } else if (bg_mode == 2u) {
        let center = vec2<f32>(0.0,0.0);
        let r = length(p - center) / (length(metaballs.v2.xy) * 0.25 + 1e-5);
        let t = clamp(r, 0.0, 1.0);
        bg = mix(vec3<f32>(0.12,0.13,0.18), vec3<f32>(0.02,0.02,0.04), t);
    } else if (bg_mode == 3u) {
        // Tiny hash noise (deterministic) for quick visual contrast
        let h = fract(sin(dot(p, vec2<f32>(12.9898,78.233))) * 43758.5453);
        bg = vec3<f32>(0.04 + 0.04*h, 0.05 + 0.05*h, 0.07 + 0.05*h);
    }

    // Alpha blend metaballs over background
    let out_rgb = mix(bg, base_col, mask);
    return vec4<f32>(out_rgb, 1.0);
}
