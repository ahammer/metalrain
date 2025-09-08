// Metaballs Unified (Simplified Functional Variant)
// Restores full binding layout & entry point names expected by Rust material while
// keeping logic trimmed for initial bring‑up. Once validated, richer shading (noise,
// bevel, SDF, tiling early‑exit) can be layered back in incrementally.

// ----------------------------------------------------------------------------
// UNIFORMS (match Rust struct MetaballsUniform packing order)
// v0: (ball_count_exposed, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, fg_mode, bg_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// v3: (tiles_x, tiles_y, tile_size_px, balls_len_actual)
// v4: (enable_early_exit, needs_gradient, metadata_v2_flag, _reserved)
// v5: (sdf_enabled, distance_range, channel_mode, max_gradient_samples)
//      distance_range currently reinterpreted in this simplified variant as a normalized
//      SDF feather HALF-WIDTH (0 => hard edge, typical <= 0.25). channel_mode/max_gradient_samples reserved.
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
fn world_to_uv(p: vec2<f32>) -> vec2<f32> {
    // world_pos spans roughly [-viewport/2, +viewport/2]
    let vp = metaballs.v2.xy; // (w,h)
    return (p / vp) + vec2<f32>(0.5, 0.5);
}

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
// Field kernel (classic polynomial falloff)
fn field_contrib(p: vec2<f32>, center: vec2<f32>, r: f32) -> f32 {
    if (r <= 0.0) { return 0.0; }
    let d = p - center;
    let d2 = dot(d, d);
    let r2 = r * r;
    if (d2 >= r2) { return 0.0; }
    let x = 1.0 - d2 / r2;
    return x * x * x; // (1 - d^2/r^2)^3
}

// ----------------------------------------------------------------------------
// Simple shading modes (subset) controlled by fg_mode & debug_view
fn shade_classic(field: f32, iso: f32) -> vec3<f32> {
    let g = clamp(field / iso, 0.0, 1.0);
    return vec3<f32>(g, g, g);
}

fn shade_metadata(field: f32, iso: f32) -> vec3<f32> {
    // Encode normalized field + iso into RGB for quick inspection
    let n = clamp(field / max(iso, 1e-5), 0.0, 1.0);
    return vec3<f32>(n, iso, 1.0 - n);
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let p = v.world_pos;
    let ball_count = u32(metaballs.v0.x + 0.5);
    let iso = max(metaballs.v0.w, 1e-5);
    let radius_scale = metaballs.v0.z;      // 1/k normalization
    let radius_mult = metaballs.v2.w;       // runtime multiplier
    let fg_mode = u32(metaballs.v1.y + 0.5); // (ClassicBlend=0, Bevel=1, OutlineGlow=2, Metadata=3)
    var sum_field: f32 = 0.0;
    let sdf_enabled = metaballs.v5.x > 0.5;
    let feather_norm = metaballs.v5.y;
    // Per-ball loop with optional per-glyph SDF masking. Only samples atlas for balls with shape_index > 0.
    // NOTE: This is more expensive (textureSample per influencing ball). Optimize later with tiling / early-exit.
    var last_sdf_sample: f32 = 1.0;    // for debug visualization
    var last_sdf_mask: f32 = 1.0;
    var last_shape_idx: u32 = 0u;
    for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
    let b0 = balls[i].data0;
    let b1 = balls[i].data1;
    let ctr = b0.xy;
    let r = b0.z * radius_scale * radius_mult;
        if (r <= 0.0) { continue; }
        // Compute analytic contribution first (we will scale by glyph mask if applicable)
        var contrib = field_contrib(p, ctr, r);
        if (sdf_enabled && contrib > 0.0) {
            // Decode packed gid: high 16 bits shape index, low 16 bits color group
            let packed = u32(b0.w + 0.5);
            let shape_idx = (packed >> 16) & 0xFFFFu;
            if (shape_idx > 0u) {
                last_shape_idx = shape_idx;
                // Guard against OOB: clamp to available metadata length-1 (assumes contiguous indexing with sentinel 0)
                let shape_meta = sdf_shape_meta[shape_idx];
                // Map fragment point into the ball's AABB square in [0,1]^2
                // Apply inverse rotation so glyph texture space follows physics rotation.
                // data1.xy packed as (cos_theta, sin_theta) from CPU (GpuBall.data1).
                let cos_t = b1.x;
                let sin_t = b1.y;
                let rel = p - ctr;
                // Inverse rotation matrix (cos sin; -sin cos) applied to rel
                let rel_rot = vec2<f32>( rel.x * cos_t + rel.y * sin_t, -rel.x * sin_t + rel.y * cos_t );
                // Map rotated vector into square AABB spanning [-r, +r] -> [0,1]
                let uv_local = (rel_rot / (2.0 * r)) + vec2<f32>(0.5, 0.5);
                // If outside the square, skip (keeps circle silhouette outside glyph bounds)
                if (all(uv_local >= vec2<f32>(0.0, 0.0)) && all(uv_local <= vec2<f32>(1.0, 1.0))) {
                    // Interpolate within atlas sub-rect
                    let atlas_uv = shape_meta.uv0 + (shape_meta.uv1 - shape_meta.uv0) * uv_local;
                    let sample_val = textureSample(sdf_atlas_tex, sdf_sampler, atlas_uv).r;
                    let mask_val = sdf_mask(sample_val, feather_norm);
                    contrib *= mask_val;
                    last_sdf_sample = sample_val;
                    last_sdf_mask = mask_val;
                }
            }
        }
        sum_field += contrib;
    }

    var rgb: vec3<f32>;
    if (fg_mode == 3u) { // Metadata mode (extended when SDF enabled)
        if (sdf_enabled && last_shape_idx > 0u) {
            // Visualize last sampled glyph: R = raw SDF sample, G = mask, B = amplified signed distance proxy
            let d_vis = clamp((last_sdf_sample - 0.5) * 8.0 + 0.5, 0.0, 1.0);
            rgb = vec3<f32>(last_sdf_sample, last_sdf_mask, d_vis);
        } else {
            rgb = shade_metadata(sum_field, iso);
        }
    } else {
        rgb = shade_classic(sum_field, iso);
    }
    return vec4<f32>(rgb, 1.0);
}
