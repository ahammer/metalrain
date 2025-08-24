// Unified Metaballs Shader (Classic, BevelGray, BevelNoise)
// Modes encoded in metaballs.v1.y (render_mode): 0=Classic transparent, 1=BevelGray opaque, 2=BevelNoise opaque animated background
// Time seconds encoded in metaballs.v2.z.
// Debug view flag remains metaballs.v1.w (1 => grayscale scalar field output).
// Existing uniform layout preserved EXACTLY (no size/alignment changes) to allow drop-in replacement.
// Shared field accumulation and selection logic consolidated; per-mode branching happens AFTER heavy accumulation.
// Bevel modes reuse identical AA mask and lighting; noise background adds lightweight two-octave value noise.
// Future TODO: make noise palette configurable via extended uniform (would require struct versioning).

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

struct MetaballsData {
    v0: vec4<f32>, // (ball_count, cluster_color_count, radius_scale, iso)
    v1: vec4<f32>, // (normal_z_scale, render_mode, radius_multiplier, debug_view)
    v2: vec4<f32>, // (window_size.x, window_size.y, time_seconds, reserved3)
    balls: array<vec4<f32>, MAX_BALLS>,             // (x, y, radius, cluster_index as float)
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>, // (r,g,b,_)
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

// --- Noise helpers (value noise: hash -> interpolate) ---
fn hash2(p: vec2<i32>) -> f32 {
    // Thomas Wang style integer hash adapted for 2D combining; outputs [0,1)
    var h: i32 = p.x * 374761393 + p.y * 668265263; // large primes
    h = (h ^ (h >> 13)) * 1274126177;
    h = h ^ (h >> 16);
    return f32(h & 0x7fffffff) / f32(0x7fffffff);
}
fn fade(t: vec2<f32>) -> vec2<f32> { return t * t * t * (t * (t * 6.0 - 15.0) + 10.0); }
fn value_noise(p: vec2<f32>) -> f32 {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let w = fade(f);
    let a = hash2(i);
    let b = hash2(i + vec2<i32>(1,0));
    let c = hash2(i + vec2<i32>(0,1));
    let d = hash2(i + vec2<i32>(1,1));
    let x1 = mix(a, b, w.x);
    let x2 = mix(c, d, w.x);
    return mix(x1, x2, w.y);
}
fn background_noise_color(p: vec2<f32>, time: f32) -> vec3<f32> {
    // Domain scale & animation
    let q = p * 0.004 + vec2<f32>(time * 0.03, time * 0.02);
    let n1 = value_noise(q);
    let n2 = value_noise(q * 2.15 + 7.3);
    let n = clamp(0.65 * n1 + 0.35 * n2, 0.0, 1.0);
    let c1 = vec3<f32>(0.05, 0.08, 0.15);
    let c2 = vec3<f32>(0.04, 0.35, 0.45);
    let c3 = vec3<f32>(0.85, 0.65, 0.30);
    let mid = smoothstep(0.0, 0.6, n);
    let hi = smoothstep(0.55, 1.0, n);
    let base = mix(c1, c2, mid);
    let final_col = mix(base, c3, hi * 0.35); // renamed from 'final' to avoid reserved keyword
    return final_col * 0.9; // slight darken for contrast
}

// --- Cluster field accumulation (sparse K) ---
const K_MAX : u32 = 12u;
struct AccumResult { used: u32, indices: array<u32, K_MAX>, field: array<f32, K_MAX>, grad: array<vec2<f32>, K_MAX> }

fn accumulate_clusters(p: vec2<f32>, ball_count: u32, cluster_color_count: u32, radius_scale: f32, radius_multiplier: f32) -> AccumResult {
    var res: AccumResult;
    res.used = 0u;
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
            for (var k: u32 = 0u; k < res.used; k = k + 1u) { if (res.indices[k] == cluster) { found = i32(k); break; } }
            if (found >= 0) {
                let idx = u32(found);
                res.field[idx] = res.field[idx] + fi;
                res.grad[idx] = res.grad[idx] + g;
            } else if (res.used < K_MAX) {
                res.indices[res.used] = cluster;
                res.field[res.used] = fi;
                res.grad[res.used] = g;
                res.used = res.used + 1u;
            } else {
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var k: u32 = 0u; k < K_MAX; k = k + 1u) { if (res.field[k] < smallest) { smallest = res.field[k]; smallest_i = k; } }
                if (fi > smallest) {
                    res.indices[smallest_i] = cluster;
                    res.field[smallest_i] = fi;
                    res.grad[smallest_i] = g;
                }
            }
        }
    }
    return res;
}

// NOTE: removed select_dominant with pointer parameter (suspected to trigger Naga SPIR-V backend bug).
// Dominant selection now inlined in fragment shader to avoid pointer-to-array passing.

fn compute_mask(best_field: f32, iso: f32, grad: vec2<f32>, p: vec2<f32>) -> f32 {
    let grad_len = max(length(grad), 1e-5);
    let field_delta = best_field - iso;
    let px_world = length(vec2<f32>(dpdx(p.x), dpdy(p.y)));
    let smooth_width = grad_len * px_world * 1.0;
    return clamp(0.5 + field_delta / smooth_width, 0.0, 1.0);
}

fn bevel_lighting(base_col: vec3<f32>, grad: vec2<f32>, normal_z_scale: f32) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(-0.707, 0.707, 0.5));
    let n = normalize(vec3<f32>(-grad.x, -grad.y, normal_z_scale));
    let diff = clamp(dot(n, light_dir), 0.0, 1.0);
    let ambient = 0.35;
    let base_lit = ambient + diff * 0.75;
    let spec = pow(max(dot(reflect(-light_dir, n), vec3<f32>(0.0,0.0,1.0)), 0.0), 24.0) * 0.35;
    return base_col * base_lit + spec;
}

fn apply_shadow(base_bg: vec3<f32>, best_field: f32, iso: f32) -> vec3<f32> {
    // Approximate soft shadow similar to bevel shader (heuristic only)
    let shadow_field = clamp(best_field - 0.006, 0.0, iso); // offset constant tuned
    let shadow_mask = clamp(shadow_field / iso, 0.0, 1.0) * 0.6;
    return mix(base_bg, base_bg * 0.35, shadow_mask * shadow_mask);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count = u32(metaballs.v0.x + 0.5);
    let cluster_color_count = u32(metaballs.v0.y + 0.5);
    let radius_scale = metaballs.v0.z;
    let iso = metaballs.v0.w;
    let normal_z_scale = metaballs.v1.x;
    let render_mode = u32(metaballs.v1.y + 0.5);
    let radius_multiplier = metaballs.v1.z;
    let debug_view = u32(metaballs.v1.w + 0.5);
    let time_seconds = metaballs.v2.z;

    let p = in.world_pos;
    if (ball_count == 0u) {
        if (render_mode == 0u) { discard; }
        // For bevel modes, show background directly
        if (render_mode == 1u) {
            let bg = vec3<f32>(0.42,0.42,0.42);
            return vec4<f32>(bg, 1.0);
        } else {
            let bg = background_noise_color(p, time_seconds);
            return vec4<f32>(bg, 1.0);
        }


    }

    var acc = accumulate_clusters(p, ball_count, cluster_color_count, radius_scale, radius_multiplier);
    if (acc.used == 0u) {
        if (render_mode == 0u) { discard; }
        if (render_mode == 1u) {
            let bg2 = vec3<f32>(0.42,0.42,0.42);
            return vec4<f32>(bg2, 1.0);
        } else {
            let bg2 = background_noise_color(p, time_seconds);
            return vec4<f32>(bg2, 1.0);
        }
    }
    // Inline dominant selection (was select_dominant) to avoid pointer passing bug
    var best_i: u32 = 0u;
    var best_field: f32 = acc.field[0u];
    for (var k: u32 = 1u; k < acc.used; k = k + 1u) { if (acc.field[k] > best_field) { best_field = acc.field[k]; best_i = k; } }
    let grad = acc.grad[best_i];
    if (debug_view == 1u) {
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }
    let base_col = metaballs.cluster_colors[acc.indices[best_i]].rgb;
    let mask = compute_mask(best_field, iso, grad, p);
    if (render_mode == 0u) { // Classic transparent
        if (mask <= 0.0) { discard; }
        return vec4<f32>(base_col, mask);
    }
    // Bevel modes: opaque background + bevel lighting blended by mask
    var bg: vec3<f32>;
    if (render_mode == 1u) {
        bg = vec3<f32>(0.42,0.42,0.42);
    } else {
        bg = background_noise_color(p, time_seconds);
    }
    bg = apply_shadow(bg, best_field, iso);
    let lit = bevel_lighting(base_col, grad, normal_z_scale);
    let out_col = mix(bg, lit, mask);
    return vec4<f32>(out_col, 1.0);
}
