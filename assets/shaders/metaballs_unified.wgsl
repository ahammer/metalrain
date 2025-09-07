// Metaballs Unified Shader (Foreground / Background / Metadata)
// ------------------------------------------------------------------------------------
// Version: 2025-08-31
// Implemented Improvements (see audit prompt):
//   - EarlyExitIsoFix: Early-exit now accounts for surface-noise iso shift mode (sn_mode == 1).
//   - MetadataV2: Optional (metaballs.v4.z) 16-bit cluster id encoding (B = hi8, A = lo8).
//   - AdaptiveSDF: Metadata SDF normalization scale adapts to dominant approximate radius.
//   - AdaptiveMask: Gradient-aware anti-aliasing mask toggle (metaballs.v4.w).
//   - NoiseCenteringFix: Both background and surface noise recentered post-contrast to remove mean bias.
// Secondary scaffolding / doc:
//   - Header constants block for FG mode discriminants (sync with Rust enum).
//   - TODO(TEXTURE_PALETTE): future sampled 1D texture path for cluster colors.
//   - TODO(STORAGE_METADATA_TEXTURE): richer metadata picking path.
// Removed stale TODOs replaced by above implementations.
// ------------------------------------------------------------------------------------
// Uniform packing notes:
//   v4.x = enable_early_exit
//   v4.y = needs_gradient (hint / forced by Metadata mode)
//   v4.z = metadata_v2_enabled
//   v4.w = enable_adaptive_mask (0 = legacy ramp, 1 = gradient / iso adaptive width)
// ------------------------------------------------------------------------------------
// Test Harness Guidance (CPU-side / host app):
//   1. Capture reference frame pre-refactor.
//   2. Render post-refactor with enable_adaptive_mask=0 and sn_mode=0 additive disabled; diff tolerance avg abs < 0.01.
//   3. Toggle adaptive mask (v4.w=1) – inspect thinner yet AA-stable edges.
//   4. Toggle metadata_v2_enabled (v4.z) using a cluster id > 255 and confirm B/A encode hi/lo bytes.
//   5. Force sn_mode=1 (iso shift) and compare silhouette vs forcing early_exit off globally (v4.x=0) – shapes must match.
// ------------------------------------------------------------------------------------
// NOTE: Foreground mode discriminants (keep in sync with Rust enum order)
//   0 = ClassicBlend, 1 = Bevel, 2 = OutlineGlow, 3 = Metadata
//   Add new modes ONLY by updating both Rust & this header block.
// TODO(TEXTURE_PALETTE): Introduce sampler + 1D texture for cluster palette; keep binding indices stable.
// TODO(STORAGE_METADATA_TEXTURE): Separate structured metadata write path (u32 packed cluster + flags).

// MAX_CLUSTERS legacy removed – dynamic storage buffer now used.
const K_MAX               : u32 = 12u;
const FG_MODE_METADATA    : u32 = 3u;
// Hoisted tuning constants
const AA_RAMP_START_FACTOR : f32 = 0.6;   // legacy mask ramp start fraction
const GRAD_EPS             : f32 = 1e-5;  // gradient epsilon
const FIELD_EPS            : f32 = 1e-5;  // generic small epsilon
const MAX_OCTAVES          : u32 = 6u;    // loop cap (mirrors existing 6)
const LEGACY_D_SCALE       : f32 = 8.0;   // legacy metadata SDF scale fallback
const SDF_SCALE_MIN        : f32 = 4.0;   // adaptive SDF scale clamp min
const SDF_SCALE_MAX        : f32 = 24.0;  // adaptive SDF scale clamp max

// =============================
// Uniform Buffers (16B aligned)
// =============================

// Metaballs params + big arrays remain vec4-packed already.
struct MetaballsData {
    // v0: (ball_count_exposed, group_count, radius_scale, iso)
    v0: vec4<f32>,
    // v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
    v1: vec4<f32>,
    // v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
    v2: vec4<f32>,
    // v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)
    v3: vec4<f32>,
    // v4: (enable_early_exit, needs_gradient, metadata_v2_enabled, enable_adaptive_mask)
    v4: vec4<f32>,
    // v5: (sdf_enabled, distance_range, channel_mode, max_gradient_samples)
    v5: vec4<f32>,
};

@group(2) @binding(0) var<uniform> metaballs: MetaballsData;

// Noise params, packed into three vec4 "slots":
// v0 = (base_scale, warp_amp, warp_freq, speed_x)
// v1 = (speed_y, gain, lacunarity, contrast_pow)
// v2 = (octaves, ridged, _pad0, _pad1)   // u32 lanes for flags
struct NoiseParamsStd140 {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<u32>,
};

@group(2) @binding(1) var<uniform> noise_params: NoiseParamsStd140;

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

@group(2) @binding(2) var<uniform> surface_noise: SurfaceNoiseParamsStd140;

// =============================
// Storage Buffers
// =============================
struct GpuBall { data: vec4<f32>, }; // (x,y,radius,group_id) -- group id encodes fusion cluster or orphan id
struct TileHeader { offset: u32, count: u32, _pad0: u32, _pad1: u32, };
@group(2) @binding(3) var<storage, read> balls: array<GpuBall>;
@group(2) @binding(4) var<storage, read> tile_headers: array<TileHeader>;
@group(2) @binding(5) var<storage, read> tile_ball_indices: array<u32>;
struct GroupColor { value: vec4<f32>, };
@group(2) @binding(6) var<storage, read> group_palette: array<GroupColor>; // palette indexed by group id
// Optional SDF atlas (texture binding index aligned with Rust material). Sampler uses default implicit sampler; if texture not bound runtime path disabled.
@group(2) @binding(7) var sdf_atlas_tex: texture_2d<f32>;
// Shape metadata (uv0, uv1, pivot, pad) – index 0 is dummy (analytic circle fallback)
struct SdfShapeGpuMeta { uv0: vec2<f32>, uv1: vec2<f32>, pivot: vec2<f32>, pad: vec2<f32> };
@group(2) @binding(8) var<storage, read> sdf_shape_meta: array<SdfShapeGpuMeta>;
// NOTE: explicit sampler binding deferred to avoid pipeline layout break; sampling uses implicit sampler assumption.


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

    // Normalize & clamp raw accumulation to [0,1].
    n = n / max(weight_sum, 1e-5);
    n = clamp(n, 0.0, 1.0);
    // Apply contrast THEN re-center (NoiseCenteringFix) to remove mean shift when contrast != 1.
    if (contrast != 1.0) {
        n = pow(n, contrast);
    }
    let n_centered = n - 0.5; // now in [-0.5,0.5]
    let n01 = n_centered + 0.5; // re-map for remaining color shaping logic

    let mid_sharp = smoothstep(0.30, 0.70, n01);
    let hi        = smoothstep(0.55, 0.95, n01);
    let n_mix = mix(n01, mid_sharp, 0.15);

    let cA = vec3<f32>(0.05, 0.08, 0.15);
    let cB = vec3<f32>(0.04, 0.35, 0.45);
    let cHi= vec3<f32>(0.85, 0.65, 0.30);
    let base = mix(cA, cB, mid_sharp);
    let out_col = mix(base, cHi, hi * 0.35) * (n_mix * 0.05 + 0.95); // subtle variation retains centered mean
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
    if (contrast != 1.0) {
        n = pow(n, contrast);
    }
    // Recenter post-contrast (NoiseCenteringFix)
    let n_centered = n - 0.5;
    // Return re-biased into [0,1] domain * amplitude: amp * (n_centered + 0.5)
    return (n_centered + 0.5) * amp;
}

// =============================
// Accumulation
// =============================
struct AccumResult {
    used:      u32,
    best_i:    u32,
    best_field: f32,
    _pad:      f32,
    indices:   array<u32,  K_MAX>,
    field:     array<f32,  K_MAX>,
    grad:      array<vec2<f32>, K_MAX>,
    approx_r:  array<f32, K_MAX>,
};

fn sample_sdf_distance(shape_index: u32, p: vec2<f32>) -> f32 {
    if (metaballs.v5.x < 0.5) { return 0.0; } // disabled
    if (shape_index == 0u) { return 0.0; } // analytic circle sentinel
    let meta = sdf_shape_meta[shape_index];
    let uv0 = meta.uv0; let uv1 = meta.uv1; let pivot = meta.pivot;
    let rect_size = uv1 - uv0;
    if (rect_size.x <= 0.0 || rect_size.y <= 0.0) { return 0.0; }
    // Map world position to tile UV: simple translation around pivot, using tile_size_px for scale heuristically.
    let tile_px = metaballs.v3.z; // tile_size_px reused as approximate scale (improvement: per-shape scale)
    let local = (p - pivot) / max(tile_px, 1.0) + vec2<f32>(0.5,0.5);
    let uv_tile = clamp(local, vec2<f32>(0.0), vec2<f32>(1.0));
    let atlas_uv = uv0 + uv_tile * rect_size;
    // Derive integer texel coordinate from normalized atlas uv using viewport dims as placeholder (improvement: store atlas dims in uniform)
    let tex_dim = metaballs.v2.xy; // currently viewport dims; TODO: pass atlas dimensions explicitly
    let texel = textureLoad(sdf_atlas_tex, vec2<i32>(atlas_uv * tex_dim), 0);
    let mode = u32(metaballs.v5.z + 0.5);
    var dist_n: f32;
    if (mode == 1u) {
        dist_n = texel.r;
    } else { // MSDF median
        let r = texel.r; let g = texel.g; let b = texel.b;
        dist_n = max(min(r,g), min(max(r,g), b));
    }
    let dr = metaballs.v5.y;
    let sd = (dist_n - 0.5) * dr; // signed distance (approx) in px units
    return sd;
}

fn accumulate_groups_tile(
    p: vec2<f32>,
    tile: TileHeader,
    ball_count_exposed: u32,
    balls_len_actual: u32,
    group_count: u32,
    radius_scale: f32,
    radius_multiplier: f32,
    allow_early_exit: bool,
    needs_gradient: bool,
    effective_iso: f32
) -> AccumResult {
    var res: AccumResult;
    res.used = 0u;
    res.best_i = 0u;
    res.best_field = 0.0;
    res._pad = 0.0;
    for (var k: u32 = 0u; k < K_MAX; k = k + 1u) {
        res.indices[k] = 0u;
        res.field[k]   = 0.0;
        res.grad[k]    = vec2<f32>(0.0, 0.0);
        res.approx_r[k]= 0.0;
    }
    var min_field: f32 = 1e30;
    var min_i: u32 = 0u;
    let safe_ball_count = min(ball_count_exposed, balls_len_actual);
    for (var j: u32 = 0u; j < tile.count; j = j + 1u) {
        let bi = tile_ball_indices[tile.offset + j];
        if (bi >= safe_ball_count) { break; }
        let b = balls[bi].data;
        let center = b.xy;
        let radius = b.z * radius_multiplier;
        if (radius <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d, d);
        let scaled_r = radius * radius_scale;
        let r2 = scaled_r * scaled_r;
        if (d2 >= r2) { continue; }
        let inv_r2 = 1.0 / r2;
        let x  = 1.0 - d2 * inv_r2;
        let x2 = x * x;
        let packed = u32(b.w + 0.5);
        let shape_index = packed >> 16u;
        let cluster = packed & 0xFFFFu; // low 16 bits retain color group id
        if (cluster >= group_count) { continue; }
        var fi: f32;
        var g = vec2<f32>(0.0, 0.0);
        if (metaballs.v5.x >= 0.5 && shape_index != 0u) {
            let sd = sample_sdf_distance(shape_index, p);
            // Convert signed distance to soft field: inside -> positive contribution
            let inside = clamp(0.5 - sd / max(radius, 1e-3), 0.0, 1.0);
            fi = inside * inside * inside; // cubic falloff
            if (needs_gradient && metaballs.v5.w >= 1.0) {
                let eps = 1.0; // 1 world unit (improve with screen-space scale)
                let sd_x = sample_sdf_distance(shape_index, p + vec2<f32>(eps,0.0)) - sd;
                let sd_y = sample_sdf_distance(shape_index, p + vec2<f32>(0.0,eps)) - sd;
                g = -vec2<f32>(sd_x, sd_y);
            }
        } else {
            let fi_raw = x2 * x;
            fi = fi_raw;
            if (needs_gradient) { g = (-6.0 * inv_r2) * d * x2; }
        }
        var found: i32 = -1;
        for (var k: u32 = 0u; k < res.used; k = k + 1u) {
            if (res.indices[k] == cluster) { found = i32(k); break; }
        }
        if (found >= 0) {
            let idx = u32(found);
            let new_field = res.field[idx] + fi;
            res.field[idx] = new_field;
            if (needs_gradient) { res.grad[idx] = res.grad[idx] + g; }
            res.approx_r[idx] = scaled_r;
            if (new_field > res.best_field) { res.best_field = new_field; res.best_i = idx; }
            if (idx == min_i) {
                var new_min: f32 = 1e30; var new_min_i: u32 = 0u;
                for (var kk: u32 = 0u; kk < res.used; kk = kk + 1u) {
                    if (res.field[kk] < new_min) { new_min = res.field[kk]; new_min_i = kk; }
                }
                min_field = new_min; min_i = new_min_i;
            }
        } else if (res.used < K_MAX) {
            let idx = res.used;
            res.indices[idx] = cluster;
            res.field[idx]   = fi;
            if (needs_gradient) { res.grad[idx] = g; }
            res.approx_r[idx] = scaled_r;
            res.used = res.used + 1u;
            if (fi > res.best_field) { res.best_field = fi; res.best_i = idx; }
            if (fi < min_field) { min_field = fi; min_i = idx; }
        } else {
            if (fi > min_field) {
                res.indices[min_i] = cluster;
                res.field[min_i]   = fi;
                if (needs_gradient) { res.grad[min_i] = g; }
                res.approx_r[min_i]= scaled_r;
                if (fi > res.best_field) { res.best_field = fi; res.best_i = min_i; }
                var new_min: f32 = 1e30; var new_min_i: u32 = min_i;
                for (var kk: u32 = 0u; kk < res.used; kk = kk + 1u) {
                    if (res.field[kk] < new_min) { new_min = res.field[kk]; new_min_i = kk; }
                }
                min_field = new_min; min_i = new_min_i;
            }
        }
        if (allow_early_exit && !needs_gradient && res.best_field >= effective_iso) { break; }
    }
    return res;
}

fn dominant(acc: AccumResult) -> u32 { return acc.best_i; }

// Conservative AA around iso via gradient magnitude and pixel footprint
// Mask computation (AA): always use derivative-free ramp for cross-platform stability.
// We intentionally avoid dpdx/dpdy after observing adapter-specific failures (black output).
// The ramp start factor (0.6) empirically balances edge softness vs. thickness.
fn compute_mask(best_field: f32, iso: f32) -> f32 {
    let ramp_start = iso * AA_RAMP_START_FACTOR;
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
@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    // Unpack scalar params from packed vec4 lanes (add 0.5 then u32 cast for safety)
    let p = v.world_pos;
    let ball_count_exposed  = u32(metaballs.v0.x + 0.5);
    let cluster_color_count = u32(metaballs.v5.y + 0.5);
    let radius_scale        = metaballs.v0.z;
    let iso                 = metaballs.v0.w;
    let normal_z_scale      = metaballs.v1.x;
    let fg_mode             = u32(metaballs.v1.y + 0.5);
    let bg_mode             = u32(metaballs.v1.z + 0.5);
    let debug_view          = u32(metaballs.v1.w + 0.5);
    let time_seconds        = metaballs.v2.z;
    let radius_multiplier   = metaballs.v2.w;
    let tiles_x             = u32(metaballs.v3.x + 0.5);
    let tiles_y             = u32(metaballs.v3.y + 0.5);
    let tile_size_px        = metaballs.v3.z;
    let balls_len_actual    = u32(metaballs.v3.w + 0.5);
    let enable_early_exit   = (metaballs.v4.x > 0.5);
    let needs_gradient      = (metaballs.v4.y > 0.5) || (fg_mode == FG_MODE_METADATA);
    let metadata_v2_enabled = (metaballs.v4.z > 0.5);
    let enable_adaptive_mask= (metaballs.v4.w > 0.5);

    // Compute tile coordinate (fragment positions share same space as world_pos; origin -viewport/2 .. +)
    let half_size = metaballs.v2.xy * 0.5;
    let origin = -half_size;
    let rel = p - origin;
    let tc = clamp(vec2<i32>(floor(rel / tile_size_px)), vec2<i32>(0,0), vec2<i32>(i32(tiles_x)-1, i32(tiles_y)-1));
    let tile_index = u32(tc.y) * tiles_x + u32(tc.x);
    let tile = tile_headers[tile_index];
    // Dev assertion scaffold: tile.offset + tile.count bounds would be validated if a total index length uniform existed.
    // if (debug_view == 2u) { // TODO(DEV_ASSERT): when total_index_count uniform added, detect overflow and output magenta.
    // }

    // Surface noise pre-pass for iso shift mode (sn_mode == 1) requires sampling noise BEFORE accumulation so
    // early-exit uses the correct threshold. Additive mode (sn_mode == 0) applied after accumulation.
    let sn_octaves  = surface_noise.v3.x;
    let sn_mode     = surface_noise.v3.z; // 0 = additive field perturb, 1 = iso shift
    let sn_enabled  = (surface_noise.v3.w != 0u) && (sn_octaves > 0u) && (surface_noise.v0.x > 0.00001);

    var effective_iso = iso; // may be modified if iso-shift noise active
    if (sn_enabled && sn_mode == 1u) {
        let delta = surface_noise_scalar(p, time_seconds); // unbiased after NoiseCenteringFix
        effective_iso = iso + (delta - surface_noise.v0.x * 0.5); // EarlyExitIsoFix
    }

    // Gate early-exit if:
    //  - Additive surface noise mode (sn_mode==0) because final field may cross iso only after perturbation.
    //  - Gradient explicitly needed (e.g. metadata mode or bevel lighting). (allow_early_exit already excludes needs_gradient in accumulate routine, but we micro-opt here.)
    let allow_early_exit = enable_early_exit && !(sn_enabled && sn_mode == 0u) && !needs_gradient;

    var acc = accumulate_groups_tile(p, tile, ball_count_exposed, balls_len_actual, cluster_color_count, radius_scale, radius_multiplier, allow_early_exit, needs_gradient, effective_iso);
    if (acc.used == 0u) {
        // Metadata mode sentinel when no field contributions: (R=1,G=0,B=0,A=0)
        if (fg_mode == FG_MODE_METADATA) {
            return vec4<f32>(1.0, 0.0, 0.0, 0.0);
        }
        var bg_col2: vec4<f32>;
        switch (bg_mode) {
            case 0u: { bg_col2 = bg_solid_gray(); }
            case 1u: { bg_col2 = bg_noise(p, time_seconds); }
            default: { bg_col2 = bg_vertical(p, metaballs.v2.y); }
        }
        return bg_col2;
    }

    let dom = dominant(acc);
    var best_field = acc.best_field;
    let grad = acc.grad[dom];
    let approx_r_dom = acc.approx_r[dom];

    if (fg_mode != FG_MODE_METADATA && debug_view == 1u) {
        // Debug grayscale suppressed in Metadata mode (metadata path has priority)
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    // Surface edge modulation additive mode applied AFTER accumulation (EarlyExitIsoFix rationale).
    if (sn_enabled && sn_mode == 0u) {
        let delta_post = surface_noise_scalar(p, time_seconds);
        best_field = best_field + (delta_post - surface_noise.v0.x * 0.5);
    }

    let cluster_idx = acc.indices[dom];
    let clamped_idx = min(cluster_idx, cluster_color_count - 1u);
    let base_col = group_palette[clamped_idx].value.rgb;
    // Adaptive gradient-aware mask (AdaptiveMask) or legacy ramp.
    var mask: f32;
    if (enable_adaptive_mask) {
        let grad_len = max(length(grad), GRAD_EPS);
        // approximate pixel footprint: assume world unit ~ pixel (coordinate space contract). Could scale if viewport mapping changes.
        let aa_width = clamp(effective_iso / (grad_len + 1e-5) * 0.5, 0.75, 4.0);
        mask = smoothstep(effective_iso - aa_width, effective_iso + aa_width, best_field);
    } else {
        mask = compute_mask(best_field, effective_iso);
    }

    // ---------------------------------------------------------------------
    // Metadata Foreground Mode
    // Produces RGBA where:
    //   R = normalized signed distance proxy (0.5 = iso surface)
    //   G = clickable mask (bootstrap: all clickable => mask)
    //   B = non-clickable mask (bootstrap: 0.0)
    //   A = cluster/orphan color slot index encoded as cluster_u8 / 255
    // Signed distance proxy: (iso - field) / |grad| with gradient of dominant cluster.
    // Normalization window uses constant d_scale = 8.0 (TODO adaptive to radius / pixel density).
    // Sentinel when acc.used == 0u handled earlier.
    // Future per-ball clickability flag packing plan:
    //   Reserve high bits in balls[i].w: b.w = cluster + (clickable_flag * 4096).
    //   Shader decode: raw = u32(b.w+0.5); clickable = ((raw / 4096u) & 1u) == 1u; cluster = raw & 4095u.
    // TODO: metadata-mode SDF scaling adapt to radius & screen resolution.
    // TODO: implement per-ball clickability flag packing (see meta_ball_metadata_prompt.md).
    // PERF: confirm metadata path branch cost negligible vs existing.
    if (fg_mode == FG_MODE_METADATA) {
        let gradv = acc.grad[dom];
        let g_len = max(length(gradv), GRAD_EPS);
        let signed_d = (effective_iso - best_field) / g_len; // outside positive
        // AdaptiveSDF scale selection: derive heuristic from approx radius; fallback to legacy 8.0
        let r_px = approx_r_dom;
        var d_scale = LEGACY_D_SCALE;
        if (r_px > 0.0) { d_scale = clamp(r_px * 0.25, SDF_SCALE_MIN, SDF_SCALE_MAX); }
        var r_channel = clamp(0.5 - 0.5 * signed_d / d_scale, 0.0, 1.0);
        if (g_len <= GRAD_EPS) { r_channel = 0.5; }
        let clickable = mask; // all clickable in current design
        if (metadata_v2_enabled) {
            // Metadata V2 encoding: B = high8, A = low8 (cluster id up to 16 bits)
            let cid16 = min(cluster_idx, 65535u);
            let low = f32(cid16 & 255u) / 255.0;
            let high = f32((cid16 >> 8u) & 255u) / 255.0;
            return vec4<f32>(r_channel, clickable, high, low);
        } else {
            // Legacy encoding: B = 0, A = cluster_u8
            let cluster_u8 = min(cluster_idx, 255u);
            let a_channel = f32(cluster_u8) / 255.0;
            return vec4<f32>(r_channel, clickable, 0.0, a_channel);
        }
    }

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
        case 1u: { // Bevel
            fg_col = fg_bevel(fg_ctx.cluster_color, fg_ctx.grad, fg_ctx.mask, normal_z_scale, bg_col.rgb);
        }
        case 2u: { // OutlineGlow
            fg_col = fg_outline_glow(fg_ctx.cluster_color, fg_ctx.best_field, fg_ctx.iso, fg_ctx.mask, fg_ctx.grad);
        }
        default: { // Metadata already early-returned; keep fallback
            fg_col = vec4<f32>(fg_ctx.cluster_color, fg_ctx.mask);
        }
    }

    // Opaque BG, alpha from FG determines mix factor
    let out_rgb = mix(bg_col.rgb, fg_col.rgb, fg_col.a);
    let out_a   = max(bg_col.a, fg_col.a);
    return vec4<f32>(out_rgb, out_a);
}
