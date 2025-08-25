// Metaballs Dual-Axis Shader (Foreground / Background)
// ------------------------------------------------------------------------------------
// BINARY LAYOUT UNCHANGED (only semantic remap):
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// Arrays: balls[MAX_BALLS] (x,y,radius,cluster_index), cluster_colors[MAX_CLUSTERS] (rgb,_)
//
 // Foreground modes (v1.y):
 // 0 = ClassicBlend     (alpha = mask; transparent only within metaball interior over opaque bg)
 // 1 = Bevel            (bevel lighting; opaque blend inside mask)
 // 2 = OutlineGlow      (currently simple glow / classic hybrid)
//
 // Background modes (v1.z) (reindexed after removal of legacy external background quad):
 // 0 = SolidGray        (neutral 0.42)
 // 1 = ProceduralNoise  (animated 2-octave value noise)
 // 2 = VerticalGradient (y-based gradient)
 // NOTE: Legacy external background mode removed; all backgrounds now internally shader-driven & opaque.
//
// Heavy accumulation path left intact; new compositing only happens after field evaluation.
// Pointer-based helper functions avoided to preserve prior Naga/SPIR-V stability.
//
// Future extensibility:
// - ForegroundContext struct allows reactive backgrounds w/o recomputation.
// - Could pack modes into bitfield if uniform pressure increases.
// - Potential future second pass for complex backgrounds to keep shader lean.
//
// ------------------------------------------------------------------------------------

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;
const K_MAX : u32 = 12u;

struct MetaballsData {
    v0: vec4<f32>, // (ball_count, cluster_color_count, radius_scale, iso)
    v1: vec4<f32>, // (normal_z_scale, foreground_mode, background_mode, debug_view)
    v2: vec4<f32>, // (viewport_w, viewport_h, time_seconds, radius_multiplier)
    balls: array<vec4<f32>, MAX_BALLS>,
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>,
};

@group(2) @binding(0)
var<uniform> metaballs: MetaballsData;

// Procedural background noise parameters (NEW; separate UBO to avoid touching MetaballsData)
// Layout: 3 * vec4 slots (48 bytes). Octaves==0 => legacy 2-octave fallback.
struct NoiseParams {
    base_scale: f32,    // domain scale (inverse size)
    warp_amp: f32,      // 0 disables warp
    warp_freq: f32,     // warp noise frequency
    speed_x: f32,       // animation velocity x
    speed_y: f32,       // animation velocity y
    gain: f32,          // fBm gain
    lacunarity: f32,    // frequency multiplier
    contrast_pow: f32,  // post curve exponent
    octaves: u32,       // [0 => fallback, 1..6] default 5
    ridged: u32,        // 0|1
    _pad0: u32,
    _pad1: u32,
};

@group(2) @binding(1)
var<uniform> noise_params: NoiseParams;

// Surface (edge) noise params (independent high-frequency modulation)
// Matches Rust SurfaceNoiseParamsUniform layout
struct SurfaceNoiseParams {
    amp: f32,
    base_scale: f32,
    speed_x: f32,
    speed_y: f32,
    warp_amp: f32,
    warp_freq: f32,
    gain: f32,
    lacunarity: f32,
    contrast_pow: f32,
    octaves: u32,
    ridged: u32,
    mode: u32,     // 0=field add,1=iso shift
    enabled: u32,
    _pad0: u32,
    _pad1: u32,
};

@group(2) @binding(2)
var<uniform> surface_noise: SurfaceNoiseParams;

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

// ---------------------------------------------
// Value Noise (unchanged core cost)
// ---------------------------------------------
fn hash2(p: vec2<i32>) -> f32 {
    // Integer hash -> [0,1)
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
    return mix(mix(a,b,w.x), mix(c,d,w.x), w.y);
}
fn noise_color(p: vec2<f32>, time: f32) -> vec3<f32> {
    // Fallback path for safety (octaves==0 => legacy 2-oct blend; keeps binary compat if UBO misconfigured)
    if (noise_params.octaves == 0u) {
        let q = p * 0.004 + vec2<f32>(time * 0.03, time * 0.02);
        let n1 = value_noise(q);
        let n2 = value_noise(q * 2.15 + 7.3);
        let n_legacy = clamp(0.65 * n1 + 0.35 * n2, 0.0, 1.0);
        let c1L = vec3<f32>(0.05, 0.08, 0.15);
        let c2L = vec3<f32>(0.04, 0.35, 0.45);
        let c3L = vec3<f32>(0.85, 0.65, 0.30);
        let midL = smoothstep(0.0, 0.6, n_legacy);
        let hiL = smoothstep(0.55, 1.0, n_legacy);
        let baseL = mix(c1L, c2L, midL);
        return mix(baseL, c3L, hiL * 0.35) * 0.9;
    }

    // Base coordinate & animation
    var pw = p * noise_params.base_scale +
             vec2<f32>(time * noise_params.speed_x, time * noise_params.speed_y);

    // Single-pass domain warp (adds structure, cheap: 2 noise evals)
    if (noise_params.warp_amp > 0.0) {
        let wp = pw * noise_params.warp_freq;
        let w1 = value_noise(wp + vec2<f32>(37.2, 17.9));
        let w2 = value_noise(wp * 1.7 + vec2<f32>(11.7, 93.1));
        let warp = vec2<f32>(w1, w2) - 0.5;
        pw += warp * noise_params.warp_amp; // early skip if amp==0
    }

    // fBm (max 6 fixed loop with break => predictable)
    let is_ridged = (noise_params.ridged != 0u);
    var n: f32 = 0.0;
    var amp: f32 = 1.0;
    var freq: f32 = 1.0;
    var weight_sum: f32 = 0.0;

    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (i >= noise_params.octaves) { break; }
        var s = value_noise(pw * freq);
        if (is_ridged) {
            // Ridged shaping: invert & sharpen; emphasize first octave
            s = 1.0 - abs(s * 2.0 - 1.0);
            let ridge_boost = select(1.0, 1.25, i == 0u);
            n += s * amp * ridge_boost;
            weight_sum += amp * ridge_boost;
        } else {
            n += s * amp;
            weight_sum += amp;
        }
        freq *= noise_params.lacunarity;
        amp *= noise_params.gain;
    }

    n = n / max(weight_sum, 1e-5);

    // Contrast shaping: gamma-like + mid sharpening
    n = clamp(n, 0.0, 1.0);
    n = pow(n, noise_params.contrast_pow);
    let mid_sharp = smoothstep(0.30, 0.70, n);      // tuned window
    let hi = smoothstep(0.55, 0.95, n);             // highlights sooner for crisp sparks
    // Slight remap blend to enhance microcontrast while preserving range
    n = mix(n, mid_sharp, 0.15);

    // Palette (cool -> teal -> warm accent)
    let cA = vec3<f32>(0.05, 0.08, 0.15);
    let cB = vec3<f32>(0.04, 0.35, 0.45);
    let cHi = vec3<f32>(0.85, 0.65, 0.30);
    let base = mix(cA, cB, mid_sharp);
    let out_col = mix(base, cHi, hi * 0.35);

    return out_col * 0.95;
}

// ---------------------------------------------
// Surface Noise Scalar (edge perturbation)
// ---------------------------------------------
fn surface_noise_scalar(p: vec2<f32>, time: f32) -> f32 {
    // Base coordinate & animation
    var pw = p * surface_noise.base_scale +
             vec2<f32>(time * surface_noise.speed_x, time * surface_noise.speed_y);

    // Optional warp
    if (surface_noise.warp_amp > 0.0) {
        let wp = pw * surface_noise.warp_freq;
        let w1 = value_noise(wp + vec2<f32>(13.17, 91.3));
        let w2 = value_noise(wp * 1.73 + vec2<f32>(47.9, 5.1));
        let warp = vec2<f32>(w1, w2) - 0.5;
        pw += warp * surface_noise.warp_amp;
    }

    // fBm up to 6 octaves, early break
    let is_ridged = (surface_noise.ridged != 0u);
    var n: f32 = 0.0;
    var amp: f32 = 1.0;
    var freq: f32 = 1.0;
    var weight_sum: f32 = 0.0;
    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (i >= surface_noise.octaves) { break; }
        var s = value_noise(pw * freq);
        if (is_ridged) {
            s = 1.0 - abs(s * 2.0 - 1.0);
        }
        n += s * amp;
        weight_sum += amp;
        freq *= surface_noise.lacunarity;
        amp *= surface_noise.gain;
    }
    n = n / max(weight_sum, 1e-5);
    n = clamp(n, 0.0, 1.0);
    n = pow(n, surface_noise.contrast_pow);
    return n;
}

// ---------------------------------------------
// Accumulation
// ---------------------------------------------
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
            for (var k: u32 = 0u; k < res.used; k = k + 1u) {
                if (res.indices[k] == cluster) { found = i32(k); break; }
            }
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
                // Replace smallest contribution if new one is larger (keeps strongest clusters)
                var smallest: f32 = 1e30;
                var smallest_i: u32 = 0u;
                for (var kk: u32 = 0u; kk < K_MAX; kk = kk + 1u) {
                    if (res.field[kk] < smallest) { smallest = res.field[kk]; smallest_i = kk; }
                }
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

fn dominant(acc: AccumResult) -> u32 {
    var best_i: u32 = 0u;
    var best_field: f32 = acc.field[0u];
    for (var k: u32 = 1u; k < acc.used; k = k + 1u) {
        if (acc.field[k] > best_field) { best_field = acc.field[k]; best_i = k; }
    }
    return best_i;
}

fn compute_mask(best_field: f32, iso: f32, grad: vec2<f32>, p: vec2<f32>) -> f32 {
    let grad_len = max(length(grad), 1e-5);
    let field_delta = best_field - iso;
    let px_world = length(vec2<f32>(dpdx(p.x), dpdy(p.y)));
    let smooth_width = grad_len * px_world * 1.0;
    return clamp(0.5 + field_delta / smooth_width, 0.0, 1.0);
}

// ---------------------------------------------
// Lighting & Foreground Helpers
// ---------------------------------------------
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
    // Straight alpha (WHY: matches legacy classic transparency)
    return vec4<f32>(base_col, mask);
}

fn fg_bevel(base_col: vec3<f32>, grad: vec2<f32>, mask: f32, normal_z_scale: f32, bg_col: vec3<f32>) -> vec4<f32> {
    let lit = bevel_lighting(base_col, grad, normal_z_scale);
    let out_col = mix(bg_col, lit, mask);
    return vec4<f32>(out_col, 1.0); // Opaque foreground (alpha 1 inside; final comp handles outside)
}

fn fg_outline_glow(base_col: vec3<f32>, best_field: f32, iso: f32, mask: f32, grad: vec2<f32>) -> vec4<f32> {
    // Edge emphasis: two smoothsteps produce band near iso
    let aa = 0.01; // heuristic thickness (WHY: small to keep thin rim)
    let edge_factor = smoothstep(iso - aa, iso, best_field) * (1.0 - smoothstep(iso, iso + aa, best_field));
    let glow = pow(edge_factor, 0.5);
    let color = base_col * 0.25 + vec3<f32>(0.9,0.9,1.2) * glow;
    // Use max(mask, edge_factor) to show faint interior fill while preserving rim
    let a = max(mask, edge_factor);
    return vec4<f32>(color, a);
}

// ---------------------------------------------
// Background Helpers
// ---------------------------------------------
fn bg_solid_gray() -> vec4<f32> { let g = 0.42; return vec4<f32>(vec3<f32>(g,g,g), 1.0); }
fn bg_noise(p: vec2<f32>, time: f32) -> vec4<f32> {
    // Wrapper to keep background mode call-site unchanged
    return vec4<f32>(noise_color(p, time), 1.0);
}
fn bg_vertical(p: vec2<f32>, viewport_h: f32) -> vec4<f32> {
    // Normalize y into [-1,1] then map to [0,1]
    let t_raw = (p.y / max(viewport_h, 1.0)) * 2.0;
    let t = clamp(t_raw * 0.5 + 0.5, 0.0, 1.0);
    // Color stops chosen for subtle atmospheric gradient (WHY: low saturation base -> teal)
    let c_bottom = vec3<f32>(0.05, 0.08, 0.15);
    let c_top    = vec3<f32>(0.04, 0.35, 0.45);
    let tt = smoothstep(0.0, 1.0, t);
    return vec4<f32>(mix(c_bottom, c_top, tt), 1.0);
}

// ---------------------------------------------
// Foreground / Background Context (for future reactive features)
// ---------------------------------------------
struct ForegroundContext {
    best_field: f32,
    mask: f32,
    grad: vec2<f32>,
    iso: f32,
    cluster_index: u32,
    cluster_color: vec3<f32>,
}

// ---------------------------------------------
// Fragment
// ---------------------------------------------
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let ball_count = u32(metaballs.v0.x + 0.5);
    let cluster_color_count = u32(metaballs.v0.y + 0.5);
    let radius_scale = metaballs.v0.z;
    let iso = metaballs.v0.w;
    let normal_z_scale = metaballs.v1.x;
    let fg_mode = u32(metaballs.v1.y + 0.5);
    let bg_mode = u32(metaballs.v1.z + 0.5); // already reindexed (0..=2)
    let debug_view = u32(metaballs.v1.w + 0.5);
    let time_seconds = metaballs.v2.z;
    let radius_multiplier = metaballs.v2.w;

    let p = in.world_pos;

    // Early out when no balls: just draw background (always opaque now)
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
        // Scalar field grayscale independent of modes
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    // Surface noise modulation (edge only) - single eval guarded
    var effective_iso = iso;
    if (surface_noise.enabled == 1u && surface_noise.amp > 0.00001 && surface_noise.octaves > 0u) {
        let n = surface_noise_scalar(p, time_seconds);
        let delta = surface_noise.amp * (n - 0.5);
        if (surface_noise.mode == 0u) {
            best_field = best_field + delta;
        } else {
            effective_iso = iso + delta;
        }
    }

    let cluster_idx = acc.indices[dom];
    let base_col = metaballs.cluster_colors[cluster_idx].rgb;
    let mask = compute_mask(best_field, effective_iso, grad, p);

    let fg_ctx = ForegroundContext(
        best_field,
        mask,
        grad,
        iso,
        cluster_idx,
        base_col
    );

    // Background first
    var bg_col: vec4<f32>;
    switch (bg_mode) {
        case 0u: { bg_col = bg_solid_gray(); }
        case 1u: { bg_col = bg_noise(p, time_seconds); }
        default: { bg_col = bg_vertical(p, metaballs.v2.y); }
    }

    // Foreground
    var fg_col: vec4<f32>;
    switch (fg_mode) {
        case 0u: { // Classic
            if (fg_ctx.mask <= 0.0) {
                // Outside iso: no foreground contribution (alpha 0); background already opaque
                fg_col = vec4<f32>(fg_ctx.cluster_color, 0.0);
            } else {
                fg_col = fg_classic(fg_ctx.cluster_color, fg_ctx.mask);
            }
        }
        case 1u: { // Bevel
            fg_col = fg_bevel(fg_ctx.cluster_color, fg_ctx.grad, fg_ctx.mask, normal_z_scale, bg_col.rgb);
        }
        default: { // OutlineGlow (currently simple implementation)
            let glow = fg_outline_glow(fg_ctx.cluster_color, fg_ctx.best_field, fg_ctx.iso, fg_ctx.mask, fg_ctx.grad);
            fg_col = glow;
        }
    }

    // Composition
    // If bg transparent (External) and we are NOT ClassicBlend (already remapped earlier), bg alpha is 0 but we disallowed that case via bg_mode remap.
    // For Classic+External we used discard outside iso, so composition here handles inside region.
    var out_col: vec3<f32>;
    var out_a: f32;
    out_col = mix(bg_col.rgb, fg_col.rgb, fg_col.a);
    out_a = max(bg_col.a, fg_col.a);

    return vec4<f32>(out_col, out_a);
}
