// ============================================================================
// Metaballs Unified Hybrid Shader
// Date: 2025-09-07
// ---------------------------------------------------------------------------
// This version merges the earlier feature‑rich (main branch) foreground/background
// shading, per-cluster accumulation, adaptive edge masking, bevel / outline glow
// lighting, and metadata diagnostics WITH the new SDF glyph masking pipeline added
// on the `sdf` branch. Uniform & binding layout are PRESERVED EXACTLY.
//
// Key Features Restored / Integrated:
//  * Per-cluster (color group) accumulation with dominant cluster selection.
//  * Optional gradient calculation for bevel & metadata paths.
//  * Adaptive AA edge mask (gradient aware) toggle via metaballs.v4.w (>0.5).
//  * Foreground modes: ClassicBlend(0), Bevel(1), OutlineGlow(2), Metadata(3).
//  * Background modes: SolidGray(0), ProceduralNoise(1), VerticalGradient(2).
//  * SDF glyph masking per ball (shape_idx high 16 bits of packed_gid) with
//    normalized feather half‑width metaballs.v5.y (0..0.5). Sample > 0.5 interior.
//  * Metadata mode hybrid: legacy cluster diagnostics + SDF debug variant when
//    debug_view==3. Metadata v2 (hi/lo cluster bytes) flag via metaballs.v4.z.
//  * Early exit (optional) when field >= iso and gradient not required.
//
// Non‑Goals / Explicit Omissions:
//  * No heightfield / depth / displacement logic (deprecated).
//  * No new bindings or uniform field reordering (layout stability maintained).
//  * Surface noise iso-shift pathway deferred (only additive amplitude hook kept minimal).
//  * SDF-aware gradient (sampling atlas multiple times) deferred; current gradient
//    ignores glyph mask which can slightly over-expand adaptive AA width for
//    thin glyph details (documented limitation).
//
// Limitations / TODO:
//  * TODO: Consider SDF normal sampling using metaballs.v5.w (max_gradient_samples)
//    & metaballs.v6.w (step scale) in future for bevel accuracy on glyph edges.
//  * TODO: Optional palette texture LUT for cluster colors.
//  * TODO: Iso-shift surface noise reintegration with early-exit awareness.
//
// Alpha Channel Policy:
//  * Non-metadata modes: alpha = final mask (0..1) after adaptive/legacy edge ramp.
//  * Metadata mode: alpha conveys cluster low 8 bits (or mask in SDF debug view).
//
// Safety Notes:
//  * All loops bounded by provided counts; no unbounded dynamic allocation.
//  * Gradient math guarded by epsilon; division by zero avoided.
//  * Out-of-range cluster indices clamped to last available palette entry.
// ============================================================================

// ----------------------------------------------------------------------------
// UNIFORMS (match Rust struct MetaballsUniform packing order)
// v0: (ball_count_exposed, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, fg_mode, bg_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)
// v4: (enable_early_exit, needs_gradient, metadata_v2_flag, adaptive_mask_enable)
// v5: (legacy_sdf_enabled_unused, distance_range, channel_mode, max_gradient_samples)
//      NOTE: The first component historically toggled SDF; SDF masking is now always active.
//      distance_range currently reinterpreted as a normalized SDF feather HALF-WIDTH (0 => hard edge, typical <= 0.25).
//      channel_mode/max_gradient_samples reserved.
// v6: (atlas_width, atlas_height, atlas_tile_size, gradient_step_scale)
struct MetaballsData {
    v0: vec4<f32>,
    v1: vec4<f32>,
    v2: vec4<f32>,
    v3: vec4<f32>,
    v4: vec4<f32>,
    v5: vec4<f32>,
    v6: vec4<f32>,
};
@group(2) @binding(0) var<uniform> metaballs: MetaballsData;

// Noise (packed to keep binding indices stable – unused for now)
struct NoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<u32> };
@group(2) @binding(1) var<uniform> noise_params: NoiseParamsStd140;

// Surface noise (unused placeholder)
struct SurfaceNoiseParamsStd140 { v0: vec4<f32>, v1: vec4<f32>, v2: vec4<f32>, v3: vec4<u32> };
@group(2) @binding(2) var<uniform> surface_noise: SurfaceNoiseParamsStd140;

// STORAGE BUFFERS (mirrors Rust material bindings)
struct GpuBall { data0: vec4<f32>, data1: vec4<f32> }; // data0:(x,y,radius,packed_gid) data1:(cos,sin,_,_)
struct TileHeader { offset: u32, count: u32, _pad0: u32, _pad1: u32 };
@group(2) @binding(3) var<storage, read> balls: array<GpuBall>;
@group(2) @binding(4) var<storage, read> tile_headers: array<TileHeader>;
@group(2) @binding(5) var<storage, read> tile_ball_indices: array<u32>;
struct ClusterColor { value: vec4<f32> };
@group(2) @binding(6) var<storage, read> cluster_palette: array<ClusterColor>;

// SDF Atlas bindings (texture + shape metadata) – optional in material but declared here.
@group(2) @binding(7) var sdf_atlas_tex: texture_2d<f32>;
// Matches Rust SdfShapeGpuMeta layout: uv0.xy, uv1.xy, pivot.xy, params (tile_size_px, distance_range_px, 0, 0)
struct SdfShapeMeta { uv0: vec2<f32>, uv1: vec2<f32>, pivot: vec2<f32>, params: vec4<f32> };
@group(2) @binding(8) var<storage, read> sdf_shape_meta: array<SdfShapeMeta>;
// Sampler for SDF atlas (linear filtering for smooth edges). Bound only when SDF enabled.
@group(2) @binding(9) var sdf_sampler: sampler;

// ----------------------------------------------------------------------------
// SDF Helpers
// Polarity: sample > 0.5 is INSIDE (white interior). 0.5 is the surface.
// distance_range (v5.y) is treated as a normalized feather HALF-WIDTH in 0..0.5 domain.
// If distance_range == 0 => hard edge. We clamp to a tiny epsilon to avoid div by zero in smoothstep ordering.
fn sdf_mask(sample_value: f32, feather_norm: f32) -> f32 {
    // sample_value in [0,1]; inside when > 0.5
    // Map to signed distance in normalized units around surface: d = sample - 0.5
    let d = sample_value - 0.5;
    // Feather half-width clamped; interpret feather_norm in (0..0.5].
    let f = clamp(feather_norm, 0.00001, 0.5);
    // Inside (positive d) should go toward 1; outside toward 0 with smooth transition across [-f, +f]
    // We want 0 at d <= -f, 1 at d >= +f
    return smoothstep(-f, f, d);
}

// Consolidated SDF evaluation helper. Returns (mask, sample_val). mask=0 when outside glyph or rejected.
fn sdf_evaluate(p: vec2<f32>, ctr: vec2<f32>, r: f32, cos_t: f32, sin_t: f32, shape_idx: u32, feather_norm: f32) -> vec2<f32> {
    if (shape_idx == 0u) { return vec2<f32>(1.0, 1.0); }
    let relp = p - ctr;
    let rel_rot = vec2<f32>( relp.x * cos_t + relp.y * sin_t, -relp.x * sin_t + relp.y * cos_t );
    let uv_local = (rel_rot / (2.0 * r)) + vec2<f32>(0.5, 0.5);
    if (any(uv_local < vec2<f32>(0.0)) || any(uv_local > vec2<f32>(1.0))) { return vec2<f32>(0.0, 1.0); }
    let shape_meta = sdf_shape_meta[shape_idx];
    let atlas_uv = shape_meta.uv0 + (shape_meta.uv1 - shape_meta.uv0) * uv_local;
    let sample_val = textureSample(sdf_atlas_tex, sdf_sampler, atlas_uv).r;
    let d = sample_val - 0.5;
    if (d <= -feather_norm) { return vec2<f32>(0.0, sample_val); }
    if (d >=  feather_norm) { return vec2<f32>(1.0, sample_val); }
    let mask_val = smoothstep(-feather_norm, feather_norm, d);
    return vec2<f32>(mask_val, sample_val);
}

// ----------------------------------------------------------------------------
// Vertex I/O
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

// ----------------------------------------------------------------------------
// Constants & Helpers
const CLUSTER_TRACK_MAX: u32 = 12u; // Max clusters tracked per pixel.
const EPS: f32 = 1e-5;

fn field_contrib(p: vec2<f32>, center: vec2<f32>, r: f32) -> f32 {
    if (r <= 0.0) { return 0.0; }
    let d = p - center;
    let d2 = dot(d, d);
    let r2 = r * r;
    if (d2 >= r2) { return 0.0; }
    let x = 1.0 - d2 / r2;
    return x * x * x;
}

// Parallel arrays for cluster accumulation (WGSL friendly, avoids pointer indirection).
fn cluster_find(ids: ptr<function, array<u32, 12>>, count: u32, id: u32) -> i32 {
    if (count == 0u) { return -1; }
    let last_i = count - 1u; // temporal locality: recently inserted likely reused
    if ((*ids)[last_i] == id) { return i32(last_i); }
    for (var i: u32 = 0u; i < last_i; i = i + 1u) { if ((*ids)[i] == id) { return i32(i); } }
    return -1;
}
fn cluster_insert(ids: ptr<function, array<u32, 12>>, count: ptr<function, u32>, id: u32) -> u32 {
    if (*count < CLUSTER_TRACK_MAX) { let idx = *count; (*ids)[idx] = id; *count = *count + 1u; return idx; }
    // Overwrite slot 0 (least sophisticated heuristic; bounded set small).
    (*ids)[0] = id; return 0u;
}

// Background Noise (value noise with simple domain warp)
fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}
fn interp(a: f32, b: f32, t: f32) -> f32 { return a + (b - a) * (t * t * (3.0 - 2.0 * t)); }
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return interp(interp(a, b, f.x), interp(c, d, f.x), f.y);
}

fn background_color(p: vec2<f32>, mode: u32) -> vec3<f32> {
    if (mode == 0u) { // SolidGray
        return vec3<f32>(0.08, 0.08, 0.085);
    }
    if (mode == 2u) { // VerticalGradient
        let vp = metaballs.v2.xy;
        let t = clamp((p.y / max(vp.y, 1.0) + 0.5), 0.0, 1.0);
        let c0 = vec3<f32>(0.07, 0.08, 0.12);
        let c1 = vec3<f32>(0.28, 0.30, 0.38);
        return mix(c0, c1, t);
    }
    // ProceduralNoise
    let np = noise_params.v0; // pack: (base_scale, warp_amp, warp_freq, speed_x)
    let np1 = noise_params.v1; // (speed_y, gain, lacunarity, contrast_pow)
    let time = metaballs.v2.z;
    var uv = p * np.x + vec2<f32>(time * np.zw.x, time * np.zw.y);
    // domain warp
    let warp = value_noise(uv * np.z) * np.y;
    uv += vec2<f32>(warp, warp);
    var amp: f32 = 0.5;
    var freq: f32 = 1.0;
    var sum: f32 = 0.0;
    let octs = f32(noise_params.v2.x); // octaves
    for (var o: u32 = 0u; o < u32(octs); o = o + 1u) {
        sum += value_noise(uv * freq) * amp;
        freq *= np1.y; // lacunarity
        amp *= np1.x; // gain
    }
    sum = pow(clamp(sum, 0.0, 1.0), np1.z); // contrast
    return vec3<f32>(sum);
}

fn bevel_shade(cluster_col: vec3<f32>, grad: vec2<f32>, normal_z_scale: f32) -> vec3<f32> {
    let n = normalize(vec3<f32>(grad.x, grad.y, normal_z_scale));
    let L = normalize(vec3<f32>(0.5, 0.5, 1.0));
    let diff = clamp(dot(n, L), 0.0, 1.0);
    let rim = pow(1.0 - clamp(n.z, 0.0, 1.0), 2.0);
    return cluster_col * (0.35 + 0.65 * diff) + rim * 0.15;
}

fn outline_glow_shade(cluster_col: vec3<f32>, field: f32, iso: f32) -> vec3<f32> {
    let t = clamp(field / max(iso, EPS), 0.0, 2.0);
    let glow = smoothstep(0.0, 1.0, t) * (1.0 - smoothstep(1.0, 1.6, t));
    return cluster_col * glow;
}

fn classic_blend_shade(cluster_col: vec3<f32>, field: f32, iso: f32) -> vec3<f32> {
    let g = clamp(field / max(iso, EPS), 0.0, 1.0);
    return cluster_col * g;
}

fn compute_adaptive_mask(field: f32, iso: f32, grad_len: f32) -> f32 {
    let grad_l = max(grad_len, 1e-5);
    // Heuristic width similar to legacy main branch: proportional to iso/|grad|
    let aa = clamp(iso / grad_l * 0.5, 0.75, 4.0);
    return clamp(smoothstep(iso - aa, iso + aa, field), 0.0, 1.0);
}

fn compute_legacy_mask(field: f32, iso: f32) -> f32 {
    return smoothstep(iso * 0.6, iso, field);
}

fn map_signed_distance(signed_d: f32, d_scale: f32) -> f32 {
    // Normalize signed distance to [0,1] with surface at 0.5 (negative inside)
    return clamp(0.5 - 0.5 * signed_d / d_scale, 0.0, 1.0);
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let p = v.world_pos;
    let ball_count = u32(metaballs.v0.x + 0.5);
    let iso = max(metaballs.v0.w, 1e-5);
    let radius_scale = metaballs.v0.z;
    let radius_mult = metaballs.v2.w;
    let radius_coeff = radius_scale * radius_mult;
    let fg_mode = u32(metaballs.v1.y + 0.5);
    let bg_mode = u32(metaballs.v1.z + 0.5);
    let debug_view = u32(metaballs.v1.w + 0.5);
    let enable_early_exit = metaballs.v4.x > 0.5;
    let needs_gradient = metaballs.v4.y > 0.5;
    let metadata_v2 = metaballs.v4.z > 0.5;
    let adaptive_mask_enable = metaballs.v4.w > 0.5;
    let feather_norm = metaballs.v5.y; // normalized half width (0..0.5)
    let cluster_color_count = u32(metaballs.v0.y + 0.5);

    // Flags for conditional work to reduce ALU where not needed
    let want_gradient = needs_gradient || fg_mode == 1u; // bevel needs gradient lighting
    let want_cluster_r = fg_mode == 3u; // metadata mode only uses cluster_r (approx distance)

    // ------------------------ Accumulation (tile-aware) ------------------------
    var cluster_ids: array<u32, 12>;
    var cluster_field: array<f32, 12>;
    var cluster_gx: array<f32, 12>;
    var cluster_gy: array<f32, 12>;
    var cluster_r: array<f32, 12>;
    for (var ci: u32 = 0u; ci < CLUSTER_TRACK_MAX; ci = ci + 1u) { cluster_field[ci] = 0.0; cluster_gx[ci] = 0.0; cluster_gy[ci] = 0.0; cluster_r[ci] = 0.0; }
    var cluster_used: u32 = 0u;
    var last_sdf_sample: f32 = 1.0;
    var last_sdf_mask: f32 = 1.0;
    var last_shape_idx: u32 = 0u;

    // Gradient accumulation for dominant cluster (computed lazily after dominance known)
    // We approximate gradient by re-summing contributions with partial derivatives; for simplicity
    // we accumulate weighted directional components during first pass (gx, gy) using (center - p) * contrib.

    // Tile path: derive tile index
    let tiles_x = u32(metaballs.v3.x + 0.5);
    let tiles_y = u32(metaballs.v3.y + 0.5);
    let tile_size = metaballs.v3.z; // pixels
    var used_tile_path = false;
    if (tiles_x > 0u && tiles_y > 0u && tile_size > 0.0) {
        let vp = metaballs.v2.xy;
        let origin = -vp * 0.5;
        let rel = p - origin;
        let tx = clamp(u32(rel.x / tile_size), 0u, tiles_x - 1u);
        let ty = clamp(u32(rel.y / tile_size), 0u, tiles_y - 1u);
        let tile_index = ty * tiles_x + tx;
        let header = tile_headers[tile_index];
        if (header.count > 0u) {
            used_tile_path = true;
            let end = header.offset + header.count;
            for (var t: u32 = header.offset; t < end; t = t + 1u) {
                let bi = tile_ball_indices[t];
                if (bi >= ball_count) { continue; }
                let b0 = balls[bi].data0;
                let b1 = balls[bi].data1;
                let ctr = b0.xy;
                let r = b0.z * radius_coeff;
                if (r <= 0.0) { continue; }
                var contrib = field_contrib(p, ctr, r);
                if (contrib <= 0.0) { continue; }
                // Decode packed gid
                let packed = u32(b0.w + 0.5);
                let shape_idx = (packed >> 16) & 0xFFFFu;
                let cluster_id = packed & 0xFFFFu;
                if (shape_idx > 0u) {
                    let eval = sdf_evaluate(p, ctr, r, b1.x, b1.y, shape_idx, feather_norm);
                    if (eval.x <= 0.0) { contrib = 0.0; }
                    else { contrib *= eval.x; last_sdf_sample = eval.y; last_sdf_mask = eval.x; last_shape_idx = shape_idx; }
                }
                if (contrib <= 0.0) { continue; }
                var idx_i = cluster_find(&cluster_ids, cluster_used, cluster_id);
                if (idx_i < 0) { let inserted = cluster_insert(&cluster_ids, &cluster_used, cluster_id); idx_i = i32(inserted); }
                let idx = u32(idx_i);
                cluster_field[idx] = cluster_field[idx] + contrib;
                if (want_gradient || want_cluster_r) {
                    let delta = ctr - p;
                    if (want_gradient) {
                        cluster_gx[idx] = cluster_gx[idx] + delta.x * contrib;
                        cluster_gy[idx] = cluster_gy[idx] + delta.y * contrib;
                    }
                    if (want_cluster_r) { cluster_r[idx] = max(cluster_r[idx], r); }
                }
                if (enable_early_exit && !needs_gradient && cluster_field[idx] >= iso) {
                    // Early exit if dominant cluster likely reached iso; dominance may shift but acceptable heuristic.
                    break;
                }
            }
        }
    }
    if (!used_tile_path) {
        // Fallback full scan
        for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
            let b0 = balls[i].data0; let b1 = balls[i].data1;
            let ctr = b0.xy; let r = b0.z * radius_coeff; if (r <= 0.0) { continue; }
            var contrib = field_contrib(p, ctr, r); if (contrib <= 0.0) { continue; }
            let packed = u32(b0.w + 0.5);
            let shape_idx = (packed >> 16) & 0xFFFFu; let cluster_id = packed & 0xFFFFu;
            if (shape_idx > 0u) {
                let eval = sdf_evaluate(p, ctr, r, b1.x, b1.y, shape_idx, feather_norm);
                if (eval.x <= 0.0) { contrib = 0.0; }
                else { contrib *= eval.x; last_sdf_sample = eval.y; last_sdf_mask = eval.x; last_shape_idx = shape_idx; }
            }
            if (contrib <= 0.0) { continue; }
            var idx_i = cluster_find(&cluster_ids, cluster_used, cluster_id);
            if (idx_i < 0) { let inserted = cluster_insert(&cluster_ids, &cluster_used, cluster_id); idx_i = i32(inserted); }
            let idx = u32(idx_i);
            cluster_field[idx] = cluster_field[idx] + contrib;
            if (want_gradient || want_cluster_r) {
                let delta = ctr - p;
                if (want_gradient) {
                    cluster_gx[idx] = cluster_gx[idx] + delta.x * contrib;
                    cluster_gy[idx] = cluster_gy[idx] + delta.y * contrib;
                }
                if (want_cluster_r) { cluster_r[idx] = max(cluster_r[idx], r); }
            }
            if (enable_early_exit && !needs_gradient && cluster_field[idx] >= iso) { break; }
        }
    }

    if (cluster_used == 0u) {
        // Nothing contributed: background only or metadata sentinel
        if (fg_mode == 3u) {
            return vec4<f32>(1.0, 0.0, 0.0, 0.0); // metadata sentinel
        } else {
            let bg = background_color(p, bg_mode);
            return vec4<f32>(bg, 1.0);
        }
    }

    var dominant_i: u32 = 0u;
    var best_f: f32 = -1.0;
    for (var di: u32 = 0u; di < cluster_used; di = di + 1u) { if (cluster_field[di] > best_f) { best_f = cluster_field[di]; dominant_i = di; } }
    let dominant_field = cluster_field[dominant_i];
    var grad_vec = vec2<f32>(0.0, 0.0);
    if (want_gradient) { grad_vec = -vec2<f32>(cluster_gx[dominant_i], cluster_gy[dominant_i]); }

    // Compute edge mask with fast path
    var mask: f32;
    if (!adaptive_mask_enable && dominant_field >= iso) {
        mask = 1.0; // fully inside, legacy path would smoothstep to 1 anyway
    } else {
        if (adaptive_mask_enable && want_gradient) {
            let g_len = length(grad_vec);
            mask = compute_adaptive_mask(dominant_field, iso, g_len);
        } else {
            mask = compute_legacy_mask(dominant_field, iso);
        }
        mask = clamp(mask, 0.0, 1.0);
    }

    // Clamp cluster id to palette
    var cluster_id = cluster_ids[dominant_i];
    if (cluster_color_count == 0u) { cluster_id = 0u; }
    if (cluster_color_count > 0u && cluster_id >= cluster_color_count) {
        cluster_id = cluster_color_count - 1u; // clamp
    }
    let cluster_col = cluster_palette[cluster_id].value.rgb;

    // Foreground shading selection
    var fg_rgb = vec3<f32>(0.0);
    if (fg_mode == 0u) { fg_rgb = classic_blend_shade(cluster_col, dominant_field, iso); }
    else if (fg_mode == 1u) { fg_rgb = bevel_shade(cluster_col, grad_vec, metaballs.v1.x); }
    else if (fg_mode == 2u) { fg_rgb = outline_glow_shade(cluster_col, dominant_field, iso); }
    else { // Metadata mode will branch later
        fg_rgb = vec3<f32>(0.0);
    }

    // Debug view overrides (except metadata specific debug 3 applied later)
    if (debug_view == 1u && fg_mode != 3u) {
    let raw = clamp(dominant_field / iso, 0.0, 1.0);
        fg_rgb = vec3<f32>(raw);
    }

    // Background
    let bg_col = background_color(p, bg_mode);

    // Metadata handling
    if (fg_mode == 3u) {
        if (debug_view == 3u && last_shape_idx > 0u) {
            let dist_vis = clamp((last_sdf_sample - 0.5) * 8.0 + 0.5, 0.0, 1.0);
            return vec4<f32>(last_sdf_sample, last_sdf_mask, dist_vis, mask);
        }
        // Distance proxy: approximate signed distance using gradient magnitude heuristics if available
    let approx_r = max(cluster_r[dominant_i], 1.0);
    let signed_d = (iso - dominant_field) * approx_r;
        let dist_norm = map_signed_distance(signed_d, approx_r);
        let cid_lo = f32(cluster_id & 0xFFu) / 255.0;
        if (metadata_v2) {
            let cid_hi = f32((cluster_id >> 8) & 0xFFu) / 255.0;
            return vec4<f32>(dist_norm, mask, cid_hi, cid_lo);
        } else {
            return vec4<f32>(dist_norm, mask, 0.0, cid_lo);
        }
    }

    // Composite over background (simple alpha blend)
    let out_rgb = mix(bg_col, fg_rgb, mask);
    return vec4<f32>(out_rgb, mask);
}
