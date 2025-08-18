// Per-ball metaballs using a Wyvill-style bounded kernel f = (1 - (d/R)^2)^3 for d<R.
// Accumulates field & gradient for analytic normal, applies smooth AA around iso threshold.
// Single full-screen pass (vertex passthrough) using Material2d bind group (index 2 in Bevy 0.16).

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

// Mirrors the Rust `MetaballsUniform` layout exactly. Remove fields cautiously & in lockstep.
struct MetaballsData {
    ball_count: u32,
    cluster_color_count: u32,
    radius_scale: f32,
    _pad1: u32,
    window_size: vec2<f32>,
    iso: f32,
    normal_z_scale: f32,
    color_blend_exponent: f32,
    radius_multiplier: f32, // user visual expansion factor (applied before radius_scale)
    debug_view: u32,        // 0=Normal,1=Heightfield,2=ColorInfo
    _pad2: vec2<f32>,       // matches Rust padding
    balls: array<vec4<f32>, MAX_BALLS>,             // (x, y, radius, cluster_index as float)
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

// Simplified single-path fragment: per-cluster field accumulation (sparse) and flat color output.
// We aggregate contributions for up to K_MAX clusters influencing this pixel, pick the cluster with
// the largest field value, then use that cluster's color and analytic gradient (for AA mask only).
// All previous lighting / blending modes removed for clarity & performance.
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (metaballs.ball_count == 0u) { return vec4<f32>(0.0); }
    let p = in.world_pos;

    // Sparse per-pixel cluster accumulation (top-K style). K kept small for ALU efficiency.
    const K_MAX : u32 = 12u;
    var k_indices: array<u32, 12>; // uninitialized entries only valid up to used count
    var k_field: array<f32, 12>;
    var k_grad: array<vec2<f32>, 12>;
    var used: u32 = 0u;

    for (var i: u32 = 0u; i < metaballs.ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
        let radius = b.z * metaballs.radius_multiplier;
        if (radius <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d, d);
        let scaled_r = radius * metaballs.radius_scale;
        let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2; // [0,1]
            let x2 = x * x;
            let fi = x2 * x; // field contribution
            let g = (-6.0 / r2) * d * x2; // gradient contribution (2D)
            let cluster = u32(b.w + 0.5);
            if (cluster >= metaballs.cluster_color_count) { continue; }
            // Find existing slot
            var found: i32 = -1;
            for (var k: u32 = 0u; k < used; k = k + 1u) {
                if (k_indices[k] == cluster) { found = i32(k); break; }
            }
            if (found >= 0) {
                let idx = u32(found);
                k_field[idx] = k_field[idx] + fi;
                k_grad[idx] = k_grad[idx] + g;
            } else if (used < K_MAX) {
                k_indices[used] = cluster;
                k_field[used] = fi;
                k_grad[used] = g;
                used = used + 1u;
            } else {
                // Optional replacement policy: keep if this contribution beats smallest current field.
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var k: u32 = 0u; k < K_MAX; k = k + 1u) {
                    if (k_field[k] < smallest) { smallest = k_field[k]; smallest_i = k; }
                }
                if (fi > smallest) {
                    k_indices[smallest_i] = cluster;
                    k_field[smallest_i] = fi;
                    k_grad[smallest_i] = g;
                }
            }
        }
    }

    if (used == 0u) { return vec4<f32>(0.0); }
    // Aggregate total field & gradient (sum of contributions) for iso surface.
    var total_field: f32 = 0.0;
    var total_grad: vec2<f32> = vec2<f32>(0.0, 0.0);
    for (var k: u32 = 0u; k < used; k = k + 1u) {
        total_field = total_field + k_field[k];
        total_grad = total_grad + k_grad[k];
    }

    // Heightfield view: show scalar field (pre-iso) as grayscale.
    if (metaballs.debug_view == 1u) {
        let gray = clamp(total_field / metaballs.iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    // Determine dominant cluster (for ColorInfo mode) & also compute blended color for Normal.
    var best_i: u32 = 0u;
    var best_field: f32 = k_field[0u];
    for (var k: u32 = 1u; k < used; k = k + 1u) {
        if (k_field[k] > best_field) { best_field = k_field[k]; best_i = k; }
    }

    // Blended color based on field^exponent weights.
    var blend_col: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var blend_w_sum: f32 = 0.0;
    let exp = metaballs.color_blend_exponent;
    for (var k: u32 = 0u; k < used; k = k + 1u) {
        let w = pow(k_field[k], exp);
        if (w > 0.0) {
            blend_w_sum = blend_w_sum + w;
            blend_col = blend_col + w * metaballs.cluster_colors[k_indices[k]].rgb;
        }
    }
    if (blend_w_sum > 0.0) { blend_col = blend_col / blend_w_sum; }
    let color_info_col = metaballs.cluster_colors[k_indices[best_i]].rgb;
    let base_col = select(blend_col, color_info_col, metaballs.debug_view == 2u);

    // Edge AA mask from total field & total gradient (analytic derivative of summed field).
    let grad_len = max(length(total_grad), 1e-5);
    let s = (total_field - metaballs.iso) / grad_len;
    let px = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
    let aa = 1.5 * px;
    let mask = clamp(0.5 + 0.5 * s / aa, 0.0, 1.0);
    if (mask <= 0.0) { return vec4<f32>(0.0); }
    return vec4<f32>(base_col, mask);
}
