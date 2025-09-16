// Present fragment shader – improved for a soft silicone look
//
// Packed texture (rgba16f):
//   R: field
//   G,B: normalized gradient (x, y)
//   A: inverse gradient length (1/|∇|) or 0 if tiny
//
// Improvements for visual appeal:
// - Cooler base color and unified edge tint for a modern, soft hue:contentReference[oaicite:12]{index=12}.
// - Broader, slightly stronger specular highlights (silicone-like sheen):contentReference[oaicite:13]{index=13}.
// - Slightly reduced Fresnel rim intensity to avoid harsh edges:contentReference[oaicite:14]{index=14}.
// - Wider bevel highlight band for a smooth inner glow at edges.
// - Deeper, softer drop shadow (more blur and darkness) for depth:contentReference[oaicite:15]{index=15}.
//
// (No new uniforms or bindings added; all tweaks use existing parameters.)
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

// Core iso-surface & edge AA
const ISO: f32             = 0.50;
const EDGE_BAND: f32       = 1.50;
const USE_DERIV_EDGE: bool = true;

// Bevel (surface shading) controls
const BEVEL_PX: f32        = 18.0;
const BEVEL_CURVE_EXP: f32 = 1.4;
const BEVEL_SECOND_EXP: f32= 1.0;    // (disable second shaping by =1.0)

// Interior flattening (keeps top relatively flat after bevel region)
const FLAT_PX: f32         = 42.0;
const EDGE_FADE_EXP: f32   = 1.3;    // how quickly edge effects fade toward center

// Lighting constants for soft plastic/silicone look
const NORMAL_Z: f32        = 0.65;
const AMBIENT: f32         = 0.18;
const DIFFUSE_INT: f32     = 0.90;

const BASE_COLOR: vec3<f32> = vec3<f32>(0.55, 0.58, 0.66); // base hue (soft blue-gray):contentReference[oaicite:16]{index=16}
const EDGE_COLOR: vec3<f32> = vec3<f32>(0.45, 0.60, 0.75); // edge tint (cooler blue) for subtle hue shift
const EDGE_MIX: f32        = 0.35;   // mix strength of edge tint at surface

// Specular highlight (slightly stronger and moderately sharp)
const SPEC_POW: f32        = 48.0;   // was 34; higher yields a bit sharper highlight
const SPEC_INT: f32        = 0.60;   // was 0.55; a touch brighter highlight
const SPEC_GRAD_SCALE: f32 = 0.10;   // keep gradient scaling to avoid center hotspot

// Fresnel rim light (slightly dialed back for softness)
const FRES_INT: f32        = 0.50;   // was 0.55; lower intensity so rim is subtle:contentReference[oaicite:17]{index=17}
const FRES_POW: f32        = 3.0;    // keep power; moderate falloff for rim

// Wetness/edge tint (cool hue already set above)
const EDGE_MIX_CURVE: f32  = EDGE_MIX;  // (same as EDGE_MIX, just for clarity)

// Bevel inner highlight (soft bright band just inside the edge)
const BEVEL_HIGHLIGHT_COLOR: vec3<f32> = vec3<f32>(0.95, 0.96, 0.98);
const BEVEL_HIGHLIGHT_INT: f32   = 0.40;  // was 0.35; slightly higher for a bit more glow
const BEVEL_HIGHLIGHT_WIDTH: f32 = 0.80;  // was 0.55; wider band for smoother transition
const BEVEL_HIGHLIGHT_EXP: f32   = 1.2;   // shaping exponent for highlight falloff (kept same)

// Shadow sampling (drop shadow)
const SHADOW_OFF: vec2<f32> = vec2<f32>(0.006, -0.009); // base offset (in UV units) a bit larger for deeper shadow
const SHADOW_SOFT: f32      = 1.0;   // was 0.75; increased for more blur (softer shadow edges)
const SHADOW_INT: f32       = 0.90;  // shadow intensity factor (mix amount, retained)
const BG_SHADOW_FACTOR: f32 = 0.30;  // was 0.40; darker shadow (background brightness multiplier under blob)

// Background gradient colors (kept subtle color so shadow is visible:contentReference[oaicite:18]{index=18})
const BG_TOP: vec3<f32> = vec3<f32>(0.08, 0.09, 0.12);  // slightly lighter top color
const BG_BOT: vec3<f32> = vec3<f32>(0.03, 0.035, 0.06); // slightly lighter bottom color (dark blue-gray)

 // Light direction (unnormalized) – keep top-left lighting for consistency:contentReference[oaicite:19]{index=19}
const LIGHT_DIR_UNNORM: vec3<f32> = vec3<f32>(-0.6, 0.5, 1.0);

// Utility functions
fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a * (1.0 - t) + b * t;
}
fn sample_packed(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(present_tex, present_sampler, uv, 0.0);
}
fn approx_sd(field: f32, iso: f32, inv_grad_len: f32) -> f32 {
    if (inv_grad_len <= 0.0) { return 0.0; }
    return (field - iso) * inv_grad_len;
}

// Calculate bevel interpolation parameter: 0 at surface, 1 deep into interior (up to BEVEL_PX)
fn bevel_t(sd_px: f32) -> f32 {
    var t = clamp(sd_px / BEVEL_PX, 0.0, 1.0);
    if (BEVEL_SECOND_EXP != 1.0) {
        t = pow(t, BEVEL_SECOND_EXP);
    }
    return pow(t, BEVEL_CURVE_EXP);
}

// Fresnel term for rim lighting
fn fresnel(dot_nv: f32) -> f32 {
    return pow(1.0 - max(dot_nv, 0.0), FRES_POW) * FRES_INT;
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let uv = v.uv;

    // Determine sampling UV for the square field (maintain aspect ratio cover)
    let dims = vec2<f32>(f32(textureDimensions(present_tex, 0).x),
                         f32(textureDimensions(present_tex, 0).y));
    var sample_uv = uv;
    let aspect = dims.x / dims.y;
    if (aspect > 1.0) {
        // Wide texture: scale Y
        let scale = aspect;
        sample_uv.y = (uv.y - 0.5) * scale + 0.5;
    } else if (aspect < 1.0) {
        // Tall texture: scale X
        let scale = 1.0 / aspect;
        sample_uv.x = (uv.x - 0.5) * scale + 0.5;
    }

    // Background color (vertical gradient)
    let bg = lerp(BG_BOT, BG_TOP, clamp(uv.y, 0.0, 1.0));

    // Sample the packed field/gradient texture
    let packed       = sample_packed(sample_uv);
    let field        = packed.r;
    let ngrad        = vec2<f32>(packed.g, packed.b);
    let inv_grad_len = packed.a;

    // Edge anti-aliasing width
    var w = 1e-4;
    if (USE_DERIV_EDGE) {
        w = max(fwidth(field) * EDGE_BAND, 1e-4);
    } else {
        let est = inv_grad_len * 0.5;
        w = clamp(est, 0.001, 0.05);
    }

    // Smooth coverage for the metaball shape (anti-aliased edge)
    let inside_mask = smoothstep(ISO - w, ISO + w, field);

    // Approximate signed distance in field (positive inside shape)
    let sd = approx_sd(field, ISO, inv_grad_len);
    // Convert to pixel distance (assuming the field texture covers the blob fully)
    let sd_px = sd * dims.x;

    // Bevel interpolation (0 at surface, 1 at BEVEL_PX into interior)
    let t_bevel = bevel_t(min(sd_px, BEVEL_PX));

    // Edge factor for shading effects: 1 at surface, 0 deep inside (after FLAT_PX)
    var edge_factor = 0.0;
    if (FLAT_PX > BEVEL_PX) {
        let x = clamp(1.0 - (sd_px - BEVEL_PX) / max(FLAT_PX - BEVEL_PX, 1e-4), 0.0, 1.0);
        edge_factor = pow(x, EDGE_FADE_EXP);
    } else {
        // If no flat region, taper by bevel amount
        edge_factor = pow(1.0 - t_bevel, EDGE_FADE_EXP);
    }

    // Compute normals: raw curved normal vs flat top normal
    let rawN = normalize(vec3<f32>(-ngrad.x, -ngrad.y, NORMAL_Z));
    let flatN = vec3<f32>(0.0, 0.0, 1.0);
    // Interior flattening factor: 0 in bevel region, 1 when fully flat (past FLAT_PX)
    var interior_flat_factor = 0.0;
    if (FLAT_PX > BEVEL_PX) {
        interior_flat_factor = clamp((sd_px - BEVEL_PX) / max(FLAT_PX - BEVEL_PX, 1e-4), 0.0, 1.0);
    }
    // Final normal: mix curved normal and flat normal based on interior_flat_factor
    let N = normalize(mix(rawN, flatN, interior_flat_factor));
    let L = normalize(LIGHT_DIR_UNNORM);
    let V = vec3<f32>(0.0, 0.0, 1.0);
    let H = normalize(L + V);

    // Lighting computations
    let ndl = max(dot(N, L), 0.0);
    let diffuse = ndl * DIFFUSE_INT;

    // Specular: scaled by gradient magnitude to avoid strong highlight on flat areas
    var grad_len = 0.0;
    if (inv_grad_len > 0.0) {
        grad_len = 1.0 / inv_grad_len;
    }
    let spec_scale = clamp(grad_len * SPEC_GRAD_SCALE, 0.0, 1.0) * edge_factor;
    let spec = pow(max(dot(N, H), 0.0), SPEC_POW) * SPEC_INT * spec_scale;

    // Fresnel rim (view-dependent rim light)
    let fr = fresnel(dot(N, V)) * edge_factor;

    // Edge tint: mix base color toward cooler EDGE_COLOR at the surface
    let edge_tint_mix = (1.0 - t_bevel) * edge_factor;
    var base_col = lerp(BASE_COLOR, EDGE_COLOR, edge_tint_mix * EDGE_MIX_CURVE);

    // Bevel inner highlight: a slight brightening inside the edge for a soft glow
    var highlight_w = 0.0;
    if (sd_px <= BEVEL_PX * BEVEL_HIGHLIGHT_WIDTH) {
        let h = 1.0 - clamp(sd_px / max(BEVEL_PX * BEVEL_HIGHLIGHT_WIDTH, 1e-4), 0.0, 1.0);
        highlight_w = pow(h, BEVEL_HIGHLIGHT_EXP) * edge_factor;
    }
    // Mix in the bevel highlight color
    base_col = mix(base_col, BEVEL_HIGHLIGHT_COLOR, highlight_w * BEVEL_HIGHLIGHT_INT);

    // Combine lighting: ambient + diffuse shaded base, plus specular and Fresnel highlights
    var blob_rgb = base_col * (AMBIENT + diffuse) + spec + fr;

    // Soft drop shadow via multi-tap sampling
    let sh_uv0 = sample_uv + SHADOW_OFF * 0.5;
    let sh_uv1 = sample_uv + SHADOW_OFF * (1.0 + SHADOW_SOFT);
    let sh_uv2 = sample_uv + SHADOW_OFF * (1.5 + 2.0 * SHADOW_SOFT);
    let sh0 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv0).r);
    let sh1 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv1).r);
    let sh2 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv2).r);
    let shadow = (sh0 + sh1 + sh2) / 3.0;
    // Darken background under the blob (soft shadow)
    let bg_shadowed = lerp(bg, bg * BG_SHADOW_FACTOR, clamp(shadow * SHADOW_INT, 0.0, 1.0));

    // Composite the blob over the background
    var out_rgb = bg_shadowed;
    out_rgb = lerp(out_rgb, blob_rgb, inside_mask);
    return vec4<f32>(out_rgb, 1.0);
}
