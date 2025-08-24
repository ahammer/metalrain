// Bevel + Shadow variant of metaballs.
// Rationale: adds faux 3D bevel lighting (single directional light) and soft drop shadow
// while keeping a solid opaque background so compositing order is simplified.
// Shadow computed by sampling field at an offset; bevel uses analytic gradient
// of scalar field to construct a pseudo normal; AA band maintained.

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

struct MetaballsData {
    v0: vec4<f32>, // (ball_count, cluster_color_count, radius_scale, iso)
    v1: vec4<f32>, // (normal_z_scale, color_blend_exp, radius_multiplier, debug_view)
    v2: vec4<f32>, // (window_size.x, window_size.y, reserved2, reserved3)
    balls: array<vec4<f32>, MAX_BALLS>,
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>,
};

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
    let normal_z_scale = metaballs.v1.x; // reused param
    let radius_multiplier = metaballs.v1.z;
    if (ball_count == 0u) { return vec4<f32>(0.42,0.42,0.42,1.0); }
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
        let d2 = dot(d,d);
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
            for (var k: u32 = 0u; k < used; k = k + 1u) { if (k_indices[k] == cluster) { found = i32(k); break; } }
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

    // Base background (opaque)
    var out_col = vec3<f32>(0.42,0.42,0.42);
    if (used == 0u) { return vec4<f32>(out_col,1.0); }

    // Dominant cluster selection
    var best_i: u32 = 0u;
    var best_field: f32 = k_field[0u];
    for (var k: u32 = 1u; k < used; k = k + 1u) { if (k_field[k] > best_field) { best_field = k_field[k]; best_i = k; }}

    let base_col = metaballs.cluster_colors[k_indices[best_i]].rgb;
    let grad = k_grad[best_i];
    let grad_len = max(length(grad), 1e-5);
    let field_delta = best_field - iso;
    let px_world = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
    let smooth_width = grad_len * px_world * 1.0;
    // mask in [0,1]
    let mask = clamp(0.5 + field_delta / smooth_width, 0.0, 1.0);

    // Shadow pass: sample field at offset to decide shadow mask (cheap soft falloff via mask^2)
    let shadow_vec = vec2<f32>(6.0, -6.0); // screen-space dir (magic # ok w/comment)
    var shadow_field: f32 = 0.0;
    if (mask < 1.0) { // only need if not completely inside main shape
        // limited re-accumulation: approximate using original dominant gradient only (fast)
        // More accurate: re-run full accumulation at p - shadow_vec (TODO future improvement)
        // Approx heuristic: reuse best_field but attenuate based on distance along gradient
        let shift = p - shadow_vec;
        // Simple distance-based fade from main mask (not physically accurate but cheap)
        shadow_field = max(best_field - 0.001 * length(shadow_vec), 0.0);
    }
    let shadow_mask = clamp(shadow_field / iso, 0.0, 1.0) * 0.6; // reduce intensity
    out_col = mix(out_col, out_col * 0.35, shadow_mask*shadow_mask);

    // Bevel lighting
    let light_dir = normalize(vec3<f32>(-0.707, 0.707, 0.5));
    // normal from gradient (flip for inward gradient). z term from parameter for adjustable relief
    let n = normalize(vec3<f32>(-grad.x, -grad.y, normal_z_scale));
    let diff = clamp(dot(n, light_dir), 0.0, 1.0);
    let ambient = 0.35;
    let base_lit = ambient + diff * 0.75;
    let spec = pow(max(dot(reflect(-light_dir, n), vec3<f32>(0.0,0.0,1.0)), 0.0), 24.0) * 0.35; // spec hardness 24
    let bevel_col = base_col * base_lit + spec;

    out_col = mix(out_col, bevel_col, mask);

    return vec4<f32>(out_col, 1.0); // always opaque
}
