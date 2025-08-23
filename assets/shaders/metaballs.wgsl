// Phase 7 (initial scaffold) metaballs shader copied from legacy (minor comment tweaks).
// Per-ball metaballs using a Wyvill-style bounded kernel f = (1 - (d/R)^2)^3 for d<R>.
// Accumulates field & gradient for analytic normal, applies smooth AA around iso threshold.

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

// Mirrors packed Rust uniform layout:
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

// Material2d bind group index 2 (view=0, mesh=1, material=2) in Bevy 0.16.
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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count = u32(metaballs.v0.x + 0.5);
    let cluster_color_count = u32(metaballs.v0.y + 0.5);
    let radius_scale = metaballs.v0.z;
    let iso = metaballs.v0.w;
    let radius_multiplier = metaballs.v1.z;
    let debug_view = u32(metaballs.v1.w + 0.5);
    if (ball_count == 0u) { discard; }
    let p = in.world_pos;

    const K_MAX : u32 = 12u;
    var k_indices: array<u32, 12>;
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
            let x = 1.0 - d2 / r2;
            let x2 = x * x;
            let fi = x2 * x;
            let g = (-6.0 / r2) * d * x2;
            let cluster = u32(b.w + 0.5);
            if (cluster >= cluster_color_count) { continue; }
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
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var kk: u32 = 0u; kk < K_MAX; kk = kk + 1u) {
                    if (k_field[kk] < smallest) { smallest = k_field[kk]; smallest_i = kk; }
                }
                if (fi > smallest) {
                    k_indices[smallest_i] = cluster;
                    k_field[smallest_i] = fi;
                    k_grad[smallest_i] = g;
                }
            }
        }
    }

    if (used == 0u) { discard; }

    var best_i: u32 = 0u;
    var best_field: f32 = k_field[0u];
    for (var k: u32 = 1u; k < used; k = k + 1u) {
        if (k_field[k] > best_field) { best_field = k_field[k]; best_i = k; }
    }

    if (debug_view == 1u) {
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    let base_col = metaballs.cluster_colors[k_indices[best_i]].rgb;
    let grad = k_grad[best_i];
    let grad_len = max(length(grad), 1e-5);
    let field_delta = best_field - iso;
    let px_world = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
    let smooth_width = grad_len * px_world * 1.0;
    if (field_delta <= -smooth_width * 0.5) { discard; }
    let mask = clamp(0.5 + field_delta / smooth_width, 0.0, 1.0);
    if (mask <= 0.0) { discard; }
    return vec4<f32>(base_col, mask);
}
