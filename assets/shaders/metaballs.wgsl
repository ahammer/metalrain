// Per-ball metaballs using a Wyvill-style bounded kernel f = (1 - (d/R)^2)^3 for d<R.
// Accumulates field & gradient for analytic normal, applies smooth AA around iso threshold.
// Single full-screen pass (vertex passthrough) using Material2d bind group (index 2 in Bevy 0.14).

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

struct MetaballsData {
    ball_count: u32,
    cluster_color_count: u32,
    radius_scale: f32,
    _pad1: u32,
    window_size: vec2<f32>,
    iso: f32,
    normal_z_scale: f32,
    // (header now 32 bytes; arrays follow aligned to 16)
    balls: array<vec4<f32>, MAX_BALLS>,          // (x, y, radius, cluster_index as float)
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>, // (r,g,b,_)
};

// Material2d bind group index is 2 (0=view,1=mesh,2=material) in Bevy 0.14; use group(2).
@group(2) @binding(0)
var<uniform> metaballs: MetaballsData;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    let half_size = metaballs.window_size * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (metaballs.ball_count == 0u) { return vec4<f32>(0.0); }
    let p = in.world_pos;
    var field: f32 = 0.0;
    var grad: vec2<f32> = vec2<f32>(0.0);
    var accum_col: vec3<f32> = vec3<f32>(0.0);
    // Accumulate contributions
    for (var i: u32 = 0u; i < metaballs.ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
        let radius = b.z;
        let cluster_index = u32(b.w + 0.5);
        if (radius <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d, d);
    let scaled_r = radius * metaballs.radius_scale;
    let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2; // in [0,1]
            let x2 = x * x;
            let fi = x2 * x; // (1 - (d/R)^2)^3
            field = field + fi;
            // grad f_i = -6 * (p-c)/R^2 * (1 - d^2/R^2)^2
            grad = grad + (-6.0 / r2) * d * x2;
            if (cluster_index < metaballs.cluster_color_count) {
                let col = metaballs.cluster_colors[cluster_index].rgb;
                accum_col = accum_col + col * fi;
            }
        }
    }
    if (field <= 0.0001) { return vec4<f32>(0.0); }
    // Normalize accumulated color by field to avoid bleaching when many overlaps
    let base_col = accum_col / max(field, 1e-5);
    // Signed distance approximation (field - iso) / |grad|
    let grad_len = max(length(grad), 1e-5);
    let s = (field - metaballs.iso) / grad_len;
    // Screen-space derivative for smooth AA band
    let px = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y))); // world-space derivative magnitude
    let aa = 1.5 * px;
    let mask = clamp(0.5 + 0.5 * s / aa, 0.0, 1.0);
    // Pseudo-normal for simple lighting (Z from param)
    let n = normalize(vec3<f32>(grad, metaballs.normal_z_scale));
    let L = normalize(vec3<f32>(0.6, 0.5, 1.0));
    let ndotl = max(dot(n, L), 0.0);
    let hemi = 0.5 + 0.5 * n.z;
    let lit = base_col * (0.25 + 0.6 * ndotl + 0.15 * hemi);
    let out_col = lit;
    return vec4<f32>(out_col, mask);
}
