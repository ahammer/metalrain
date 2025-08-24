// Metaballs Dual-Axis Shader (Foreground / Background)
// ------------------------------------------------------------------------------------
// BINARY LAYOUT UNCHANGED (only semantic remap):
// v0: (ball_count, cluster_color_count, radius_scale, iso)
// v1: (normal_z_scale, foreground_mode, background_mode, debug_view)
// v2: (viewport_w, viewport_h, time_seconds, radius_multiplier)
// Arrays: balls[MAX_BALLS] (x,y,radius,cluster_index), cluster_colors[MAX_CLUSTERS] (rgb,_)
//
// Foreground modes (v1.y):
// 0 = ClassicBlend     (alpha = mask; only transparent outside when bg = External)
// 1 = Bevel            (bevel lighting; opaque over any opaque bg)
// 2 = OutlineGlow      (currently aliased to Classic logic or simple glow)
//
// Background modes (v1.z):
// 0 = ExternalBackground (transparent; shown only when paired with ClassicBlend for outside transparency)
// 1 = SolidGray          (neutral 0.42)
// 2 = ProceduralNoise    (animated 2-octave value noise)
// 3 = VerticalGradient   (y-based gradient)
//
// REQUIREMENT ENFORCEMENT (spec success criterion #6):
// Transparent output outside blobs ONLY when (fg=ClassicBlend AND bg=ExternalBackground).
// For any other foreground with ExternalBackground selected, we promote background
// to an implicit opaque neutral gray (visual fallback) to avoid unintended full-scene
// transparency. (Future: could allow Bevel over external with alpha outside by relaxing rule.)
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
    // Domain scale chosen for subtle broad features (WHY: 0.004 keeps pattern large-scale)
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
    return mix(base, c3, hi * 0.35) * 0.9;
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
fn bg_external() -> vec4<f32> { return vec4<f32>(0.0,0.0,0.0, 0.0); }
fn bg_solid_gray() -> vec4<f32> { let g = 0.42; return vec4<f32>(vec3<f32>(g,g,g), 1.0); }
fn bg_noise(p: vec2<f32>, time: f32) -> vec4<f32> { return vec4<f32>(noise_color(p, time), 1.0); }
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
    let bg_mode_raw = u32(metaballs.v1.z + 0.5);
    let debug_view = u32(metaballs.v1.w + 0.5);
    let time_seconds = metaballs.v2.z;
    let radius_multiplier = metaballs.v2.w;

    // Enforce success criterion #6:
    // If background selected is External (0) BUT foreground not Classic (0), treat as SolidGray (1).
    // WGSL does not support inline if-expression like Rust; expand to statement form.
    var bg_mode: u32 = bg_mode_raw;
    if (bg_mode_raw == 0u && fg_mode != 0u) {
        bg_mode = 1u;
    }

    let p = in.world_pos;

    // Early out when no balls: show background (or discard for the one allowed transparent combo)
    if (ball_count == 0u) {
        if (fg_mode == 0u && bg_mode == 0u) { discard; }
        var bg_col: vec4<f32>;
        switch (bg_mode) {
            case 0u: { bg_col = bg_external(); }
            case 1u: { bg_col = bg_solid_gray(); }
            case 2u: { bg_col = bg_noise(p, time_seconds); }
            default: { bg_col = bg_vertical(p, metaballs.v2.y); }
        }
        if (bg_col.a == 0.0) { discard; }
        return bg_col;
    }

    var acc = accumulate_clusters(p, ball_count, cluster_color_count, radius_scale, radius_multiplier);
    if (acc.used == 0u) {
        if (fg_mode == 0u && bg_mode == 0u) { discard; }
        var bg_col2: vec4<f32>;
        switch (bg_mode) {
            case 0u: { bg_col2 = bg_external(); }
            case 1u: { bg_col2 = bg_solid_gray(); }
            case 2u: { bg_col2 = bg_noise(p, time_seconds); }
            default: { bg_col2 = bg_vertical(p, metaballs.v2.y); }
        }
        if (bg_col2.a == 0.0) { discard; }
        return bg_col2;
    }

    let dom = dominant(acc);
    let best_field = acc.field[dom];
    let grad = acc.grad[dom];

    if (debug_view == 1u) {
        // Scalar field grayscale independent of modes
        let gray = clamp(best_field / iso, 0.0, 1.0);
        return vec4<f32>(vec3<f32>(gray, gray, gray), 1.0);
    }

    let cluster_idx = acc.indices[dom];
    let base_col = metaballs.cluster_colors[cluster_idx].rgb;
    let mask = compute_mask(best_field, iso, grad, p);

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
        case 0u: { bg_col = bg_external(); }
        case 1u: { bg_col = bg_solid_gray(); }
        case 2u: { bg_col = bg_noise(p, time_seconds); }
        default: { bg_col = bg_vertical(p, metaballs.v2.y); }
    }

    // Foreground
    var fg_col: vec4<f32>;
    switch (fg_mode) {
        case 0u: { // Classic
            if (fg_ctx.mask <= 0.0) {
                // Only allow outside transparency if bg is External (spec) else keep alpha 0 -> composite yields bg
                if (bg_mode == 0u) { discard; }
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
    if (bg_col.a == 0.0) {
        // External background path (only allowed with Classic)
        out_col = fg_col.rgb;
        out_a = fg_col.a;
    } else {
        out_col = mix(bg_col.rgb, fg_col.rgb, fg_col.a);
        out_a = max(bg_col.a, fg_col.a);
    }

    return vec4<f32>(out_col, out_a);
}
