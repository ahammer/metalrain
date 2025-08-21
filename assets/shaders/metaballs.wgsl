// Per-ball metaballs using a Wyvill-style bounded kernel f = (1 - (d/R)^2)^3 for d<R.
// Accumulates field & gradient for analytic normal, applies smooth AA around iso threshold.
// Single full-screen pass (vertex passthrough) using Material2d bind group (index 2 in Bevy 0.16).

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

// Mirrors the packed Rust `MetaballsUniform` layout:
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, color_blend_exponent, radius_multiplier, debug_view)
// v2: (window_size.x, window_size.y, reserved2, reserved3)
struct MetaballsData {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
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
    let half_size = metaballs.v2.xy * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

// Simplified single-path fragment: per-cluster field accumulation (sparse) and flat color output.
// We aggregate contributions for up to K_MAX clusters influencing this pixel, pick the cluster with
// the largest field value, then use that cluster's color and analytic gradient (for AA mask only).
// All previous lighting / blending modes removed for clarity & performance.
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // If there are no balls at all, we don't want this material to draw anything.
    let ball_count = u32(metaballs.v0.x + 0.5);
    let cluster_color_count = u32(metaballs.v0.y + 0.5);
    let radius_scale = metaballs.v0.z;
    let iso = metaballs.v0.w;
    let radius_multiplier = metaballs.v1.z;
    let debug_view = u32(metaballs.v1.w + 0.5);
    if (ball_count == 0u) { discard; }
    let p = in.world_pos;

    // Sparse per-pixel cluster accumulation (top-K style). K kept small for ALU efficiency.
    const K_MAX : u32 = 12u;
    var k_indices: array<u32, 12>; // uninitialized entries only valid up to used count
    var k_field: array<f32, 12>;
    var k_grad: array<vec2<f32>, 12>;
    var used: u32 = 0u;

    for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
    let radius = b.z * radius_multiplier;
        if (radius <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d, d);
    let scaled_r = radius * radius_scale;
        let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2; // [0,1]
            let x2 = x * x;
            let fi = x2 * x; // field contribution
            let g = (-6.0 / r2) * d * x2; // gradient contribution (2D)
            let cluster = u32(b.w + 0.5);
            if (cluster >= cluster_color_count) { continue; }
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

    // No clusters contributed any field at this pixel -> do not draw (avoid writing zero alpha).
    if (used == 0u) { discard; }
    // Determine dominant cluster (max field at this pixel). We only blend balls WITHIN that cluster.
    var best_i: u32 = 0u;
    var best_field: f32 = k_field[0u];
    for (var k: u32 = 1u; k < used; k = k + 1u) {
        if (k_field[k] > best_field) { best_field = k_field[k]; best_i = k; }
    }
    // Heightfield (debug_view==1): show only the dominant cluster's scalar field normalized.
    if (debug_view == 1u) {
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }
    // Base color comes from dominant cluster (ColorInfo identical in this mode now).
    let base_col = metaballs.cluster_colors[k_indices[best_i]].rgb;
    // Hard alpha variant: remove fog / semi-transparent halo.
    // We still keep a micro AA band (one pixel) to avoid harsh stair-steps at the iso contour.
    let grad = k_grad[best_i];
    let grad_len = max(length(grad), 1e-5);
    let field_delta = best_field - iso;
    // Pixel size in world units (approx) for adaptive AA width.
    let px_world = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
    // Width of smoothing band in field units -> tune multiplier for softer/harder edge.
    let smooth_width = grad_len * px_world * 1.0; // 1.0 ~ 1 pixel band
    if (field_delta <= -smooth_width * 0.5) { discard; }
    // Map field_delta in [-smooth_width/2, smooth_width/2] to [0,1] for transitional alpha.
    let mask = clamp(0.5 + field_delta / smooth_width, 0.0, 1.0);
    if (mask <= 0.0) { discard; }
    return vec4<f32>(base_col, mask);
}
