// Metaballs Dual-Axis Shader (Foreground / Background) — 16B-aligned UBOs
// ------------------------------------------------------------------------------------
// Changes vs. your version:
// - All uniform structs are packed into vec4 slots (std140-like) for strict 16-byte alignment.
// - No scalar members in UBOs; flags are u32 lanes in vec4<u32>.
// - Accumulator arrays are initialized; "replace-smallest" only scans used range.
// - Bindings unchanged: group(2)/binding(0..2).
// ------------------------------------------------------------------------------------

const MAX_BALLS    : u32 = 1024u;
const MAX_CLUSTERS : u32 =  256u;
const K_MAX        : u32 =   12u;

// =============================
// Uniform Buffers (16B aligned)
// =============================

// Metaballs params + big arrays remain vec4-packed already.
struct MetaballsData {
    // v0: (ball_count, cluster_color_count, radius_scale, iso)
    v0: vec4<f32>,
    // v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
    v1: vec4<f32>,
    // v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
    v2: vec4<f32>,
    // Arrays are vec4 aligned; fixed sizes keep UBO valid for WebGPU.
    balls:          array<vec4<f32>, MAX_BALLS>,     // (x, y, radius, cluster_index)
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>,  // (r, g, b, _)
};

@group(2) @binding(0)
var<uniform> metaballs: MetaballsData;

// Noise params, packed into three vec4 "slots":
// v0 = (base_scale, warp_amp, warp_freq, speed_x)
// v1 = (speed_y, gain, lacunarity, contrast_pow)
// v2 = (octaves, ridged, _pad0, _pad1)   // u32 lanes for flags
struct NoiseParamsStd140 {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<u32>,
};

@group(2) @binding(1)
var<uniform> noise_params: NoiseParamsStd140;

// Surface noise params, 4 slots total (64B):
// v0 = (amp, base_scale, speed_x, speed_y)
// v1 = (warp_amp, warp_freq, gain, lacunarity)
// v2 = (contrast_pow, _fpad0, _fpad1, _fpad2)     // keep as f32s for alignment
// v3 = (octaves, ridged, mode, enabled)           // u32 flags
struct SurfaceNoiseParamsStd140 {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<u32>,
};

@group(2) @binding(2)
var<uniform> surface_noise: SurfaceNoiseParamsStd140;

// =============================
// Vertex I/O
// =============================
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0)       world_pos: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    let half_size = metaballs.v2.xy * 0.5; // viewport (w,h) * 0.5
    out.world_pos = position.xy * half_size;
    return out;
}

// =============================
// Value Noise (cheap, stable)
// =============================
fn hash2(p: vec2<i32>) -> f32 {
    var h: i32 = p.x * 374761393 + p.y * 668265263;
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
    return mix(mix(a, b, w.x), mix(c, d, w.x), w.y);
}

// Procedural color field for background
fn noise_color(p: vec2<f32>, time: f32) -> vec3<f32> {
    let octaves : u32 = noise_params.v2.x;
    if (octaves == 0u) {
        // Legacy fallback (2-octave)
        let q = p * 0.004 + vec2<f32>(time * 0.03, time * 0.02);
        let n1 = value_noise(q);
        let n2 = value_noise(q * 2.15 + 7.3);
        let n_legacy = clamp(0.65 * n1 + 0.35 * n2, 0.0, 1.0);
        let c1L = vec3<f32>(0.05, 0.08, 0.15);
        let c2L = vec3<f32>(0.04, 0.35, 0.45);
        let c3L = vec3<f32>(0.85, 0.65, 0.30);
        let midL = smoothstep(0.0, 0.6, n_legacy);
        let hiL  = smoothstep(0.55, 1.0, n_legacy);
        let baseL = mix(c1L, c2L, midL);
        return mix(baseL, c3L, hiL * 0.35) * 0.9;
    }

    // Unpack floats
    let base_scale = noise_params.v0.x;
    let warp_amp   = noise_params.v0.y;
    let warp_freq  = noise_params.v0.z;
    let speed_x    = noise_params.v0.w;

    let speed_y    = noise_params.v1.x;
    let gain       = noise_params.v1.y;
    let lacunarity = noise_params.v1.z;
    let contrast   = noise_params.v1.w;

    let ridged     = (noise_params.v2.y != 0u);

    var pw = p * base_scale + vec2<f32>(time * speed_x, time * speed_y);

    if (warp_amp > 0.0) {
        let wp = pw * warp_freq;
        let w1 = value_noise(wp + vec2<f32>(37.2, 17.9));
        let w2 = value_noise(wp * 1.7 + vec2<f32>(11.7, 93.1));
        let warp = vec2<f32>(w1, w2) - 0.5;
        pw += warp * warp_amp;
    }

    var n: f32 = 0.0;
    var amp: f32 = 1.0;
    var freq: f32 = 1.0;
    var weight_sum: f32 = 0.0;

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (i >= octaves) { break; }
        var s = value_noise(pw * freq);
        if (ridged) {
            s = 1.0 - abs(s * 2.0 - 1.0);
            let ridge_boost = select(1.0, 1.25, i == 0u);
            n += s * amp * ridge_boost;
            weight_sum += amp * ridge_boost;
        } else {
            n += s * amp;
            weight_sum += amp;
        }
        freq *= lacunarity;
        amp  *= gain;
    }

    n = n / max(weight_sum, 1e-5);
    n = clamp(n, 0.0, 1.0);
    n = pow(n, contrast);

    let mid_sharp = smoothstep(0.30, 0.70, n);
    let hi        = smoothstep(0.55, 0.95, n);
    n = mix(n, mid_sharp, 0.15);

    let cA = vec3<f32>(0.05, 0.08, 0.15);
    let cB = vec3<f32>(0.04, 0.35, 0.45);
    let cHi= vec3<f32>(0.85, 0.65, 0.30);
    let base = mix(cA, cB, mid_sharp);
    let out_col = mix(base, cHi, hi * 0.35);
    return out_col * 0.95;
}

// =============================
// Surface Noise (edge mod)
// =============================
fn surface_noise_scalar(p: vec2<f32>, time: f32) -> f32 {
    // Unpack floats
    let amp        = surface_noise.v0.x;
    let base_scale = surface_noise.v0.y;
    let speed_x    = surface_noise.v0.z;
    let speed_y    = surface_noise.v0.w;

    let warp_amp   = surface_noise.v1.x;
    let warp_freq  = surface_noise.v1.y;
    let gain       = surface_noise.v1.z;
    let lacunarity = surface_noise.v1.w;

    let contrast   = surface_noise.v2.x;

    let octaves    = surface_noise.v3.x;
    let ridged     = (surface_noise.v3.y != 0u);

    var pw = p * base_scale + vec2<f32>(time * speed_x, time * speed_y);

    if (warp_amp > 0.0) {
        let wp = pw * warp_freq;
        let w1 = value_noise(wp + vec2<f32>(13.17, 91.3));
        let w2 = value_noise(wp * 1.73 + vec2<f32>(47.9, 5.1));
        let warp = vec2<f32>(w1, w2) - 0.5;
        pw += warp * warp_amp;
    }

    var n: f32 = 0.0;
    var a: f32 = 1.0;
    var f: f32 = 1.0;
    var wsum: f32 = 0.0;

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (i >= octaves) { break; }
        var s = value_noise(pw * f);
        if (ridged) {
            s = 1.0 - abs(s * 2.0 - 1.0);
        }
        n += s * a;
        wsum += a;
        f *= lacunarity;
        a *= gain;
    }

    n = n / max(wsum, 1e-5);
    n = clamp(n, 0.0, 1.0);
    n = pow(n, contrast);
    return n * amp;
}

// =============================
// Accumulation
// =============================
struct AccumResult {
    used:   u32,
    indices: array<u32,  K_MAX>,
    field:   array<f32,  K_MAX>,
    grad:    array<vec2<f32>, K_MAX>,
};

fn accumulate_clusters(
    p: vec2<f32>,
    ball_count: u32,
    cluster_color_count: u32,
    radius_scale: f32,
    radius_multiplier: f32
) -> AccumResult {
    var res: AccumResult;

    // Initialize everything to avoid any undefined reads.
    res.used = 0u;
    for (var k: u32 = 0u; k < K_MAX; k = k + 1u) {
        res.indices[k] = 0u;
        res.field[k]   = 0.0;
        res.grad[k]    = vec2<f32>(0.0, 0.0);
    }

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
            let x  = 1.0 - d2 / r2;
            let x2 = x * x;
            let fi = x2 * x;
            let g  = (-6.0 / r2) * d * x2;

            let cluster = u32(b.w + 0.5);
            if (cluster >= cluster_color_count) { continue; }

            var found: i32 = -1;
            for (var k: u32 = 0u; k < res.used; k = k + 1u) {
                if (res.indices[k] == cluster) { found = i32(k); break; }
            }

            if (found >= 0) {
                let idx = u32(found);
                res.field[idx] = res.field[idx] + fi;
                res.grad[idx]  = res.grad[idx]  + g;
            } else if (res.used < K_MAX) {
                res.indices[res.used] = cluster;
                res.field[res.used]   = fi;
                res.grad[res.used]    = g;
                res.used = res.used + 1u;
            } else {
                // Replace the smallest among the *used* entries.
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var kk: u32 = 0u; kk < res.used; kk = kk + 1u) {
                    if (res.field[kk] < smallest) { smallest = res.field[kk]; smallest_i = kk; }
                }
                if (fi > smallest) {
                    res.indices[smallest_i] = cluster;
                    res.field[smallest_i]   = fi;
                    res.grad[smallest_i]    = g;
                }
            }
        }
    }
    return res;
}

fn dominant(acc: AccumResult) -> u32 {
    // precondition: acc.used > 0
    var best_i: u32 = 0u;
    var best_field: f32 = acc.field[0u];
    for (var k: u32 = 1u; k < acc.used; k = k + 1u) {
        if (acc.field[k] > best_field) { best_field = acc.field[k]; best_i = k; }
    }
    return best_i;
}

// Conservative AA around iso via gradient magnitude and pixel footprint
// Mask computation (AA): always use derivative-free ramp for cross-platform stability.
// We intentionally avoid dpdx/dpdy after observing adapter-specific failures (black output).
// The ramp start factor (0.6) empirically balances edge softness vs. thickness.
fn compute_mask(best_field: f32, iso: f32) -> f32 {
    let ramp_start = iso * 0.6;
    return smoothstep(ramp_start, iso, best_field);
}

// =============================
// Lighting & Foreground
// =============================
fn bevel_lighting(base_col: vec3<f32>, grad: vec2<f32>, normal_z_scale: f32) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(-0.707, 0.707, 0.5));
    let n = normalize(vec3<f32>(-grad.x, -grad.y, normal_z_scale));
    let diff = clamp(dot(n, light_dir), 0.0, 1.0);
    let ambient = 0.35;
    let base_lit = ambient + diff * 0.75;
    let spec = pow(max(dot(reflect(-light_dir, n), vec3<f32>(0.0,0.0,1.0)), 0.0), 24.0) * 0.35;
    return base_col * base_lit + spec;
}

fn fg_classic(base_col: vec3<f32>, mask: f32) -> vec4<f32> {
    return vec4<f32>(base_col, mask);
}

fn fg_bevel(base_col: vec3<f32>, grad: vec2<f32>, mask: f32, normal_z_scale: f32, bg_col: vec3<f32>) -> vec4<f32> {
    let lit = bevel_lighting(base_col, grad, normal_z_scale);
    let out_col = mix(bg_col, lit, mask);
    return vec4<f32>(out_col, 1.0);
}

fn fg_outline_glow(base_col: vec3<f32>, best_field: f32, iso: f32, mask: f32, _grad: vec2<f32>) -> vec4<f32> {
    let aa = 0.01;
    let edge_factor = smoothstep(iso - aa, iso, best_field) * (1.0 - smoothstep(iso, iso + aa, best_field));
    let glow = pow(edge_factor, 0.5);
    let color = base_col * 0.25 + vec3<f32>(0.9, 0.9, 1.2) * glow;
    let a = max(mask, edge_factor);
    return vec4<f32>(color, a);
}

// =============================
// Background helpers
// =============================
fn bg_solid_gray() -> vec4<f32> {
    let g = 0.42;
    return vec4<f32>(vec3<f32>(g, g, g), 1.0);
}
fn bg_noise(p: vec2<f32>, time: f32) -> vec4<f32> {
    return vec4<f32>(noise_color(p, time), 1.0);
}
fn bg_vertical(p: vec2<f32>, viewport_h: f32) -> vec4<f32> {
    let t_raw = (p.y / max(viewport_h, 1.0)) * 2.0;
    let t = clamp(t_raw * 0.5 + 0.5, 0.0, 1.0);
    let c_bottom = vec3<f32>(0.05, 0.08, 0.15);
    let c_top    = vec3<f32>(0.04, 0.35, 0.45);
    let tt = smoothstep(0.0, 1.0, t);
    return vec4<f32>(mix(c_bottom, c_top, tt), 1.0);
}

// Context if you later want reactive BG/FG logic without recomputing field
struct ForegroundContext {
    best_field:    f32,
    mask:          f32,
    grad:          vec2<f32>,
    iso:           f32,
    cluster_index: u32,
    cluster_color: vec3<f32>,
};

// =============================
// Fragment
// =============================
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count           = u32(metaballs.v0.x + 0.5);
    let cluster_color_count  = u32(metaballs.v0.y + 0.5);
    let radius_scale         = metaballs.v0.z;
    let iso                  = metaballs.v0.w;

    let normal_z_scale       = metaballs.v1.x;
    let fg_mode              = u32(metaballs.v1.y + 0.5);
    let bg_mode              = u32(metaballs.v1.z + 0.5);
    let debug_view           = u32(metaballs.v1.w + 0.5);

    let time_seconds         = metaballs.v2.z;
    let radius_multiplier    = metaballs.v2.w;

    let p = in.world_pos;

    // Background-only path
    if (ball_count == 0u) {
        var bg_col: vec4<f32>;
        switch (bg_mode) {
            case 0u: { bg_col = bg_solid_gray(); }
            case 1u: { bg_col = bg_noise(p, time_seconds); }
            default: { bg_col = bg_vertical(p, metaballs.v2.y); }
        }
        return bg_col;
    }

    var acc = accumulate_clusters(p, ball_count, cluster_color_count, radius_scale, radius_multiplier);
    if (acc.used == 0u) {
        var bg_col2: vec4<f32>;
        switch (bg_mode) {
            case 0u: { bg_col2 = bg_solid_gray(); }
            case 1u: { bg_col2 = bg_noise(p, time_seconds); }
            default: { bg_col2 = bg_vertical(p, metaballs.v2.y); }
        }
        return bg_col2;
    }

    let dom = dominant(acc);
    var best_field = acc.field[dom];
    let grad = acc.grad[dom];

    if (debug_view == 1u) {
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    // Surface edge modulation (optional)
    var effective_iso = iso;
    let sn_octaves  = surface_noise.v3.x;
    let sn_mode     = surface_noise.v3.z;
    let sn_enabled  = (surface_noise.v3.w != 0u);

    if (sn_enabled && sn_octaves > 0u && surface_noise.v0.x > 0.00001) {
        let delta = surface_noise_scalar(p, time_seconds); // already includes amplitude
        if (sn_mode == 0u) {
            best_field = best_field + (delta - surface_noise.v0.x * 0.5); // amp*(n-0.5)
        } else {
            effective_iso = iso + (delta - surface_noise.v0.x * 0.5);
        }
    }

    let cluster_idx = acc.indices[dom];
    let base_col = metaballs.cluster_colors[cluster_idx].rgb;
    let mask = compute_mask(best_field, effective_iso);

    let fg_ctx = ForegroundContext(
        best_field,
        mask,
        grad,
        iso,
        cluster_idx,
        base_col
    );

    // Background
    var bg_col: vec4<f32>;
    switch (bg_mode) {
        case 0u: { bg_col = bg_solid_gray(); }
        case 1u: { bg_col = bg_noise(p, time_seconds); }
        default: { bg_col = bg_vertical(p, metaballs.v2.y); }
    }

    // Foreground
    var fg_col: vec4<f32>;
    switch (fg_mode) {
        case 0u: { // ClassicBlend
            if (fg_ctx.mask <= 0.0) {
                fg_col = vec4<f32>(fg_ctx.cluster_color, 0.0);
            } else {
                fg_col = fg_classic(fg_ctx.cluster_color, fg_ctx.mask);
            }
        }
        case 1u: { // Bevel (opaque in-mask)
            fg_col = fg_bevel(fg_ctx.cluster_color, fg_ctx.grad, fg_ctx.mask, normal_z_scale, bg_col.rgb);
        }
        default: { // OutlineGlow
            fg_col = fg_outline_glow(fg_ctx.cluster_color, fg_ctx.best_field, fg_ctx.iso, fg_ctx.mask, fg_ctx.grad);
        }
    }

    // Opaque BG, alpha from FG determines mix factor
    let out_rgb = mix(bg_col.rgb, fg_col.rgb, fg_col.a);
    let out_a   = max(bg_col.a, fg_col.a);
    return vec4<f32>(out_rgb, out_a);
}
