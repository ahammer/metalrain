// Present fragment shader (simplified for fixed-width bevel metaballs)
// Focus: plastic / watery shading + soft shadow. Removed outline & glow.
//
// Packed texture (rgba16f):
//   R: field
//   G,B: normalized gradient xy
//   A: inverse gradient length (1/|∇|) or 0 if tiny
//
// Changes:
// - Fixed pixel-space bevel width via sd * resolution.
// - Configurable bevel curve (expo) -> smoother controllable shoulder.
// - Spec highlight scaled by gradient magnitude to kill central hotspot.
// - Fresnel rim for wet/plastic look; removed glow/outline/thickness tint.
// - Retained triple shadow taps; background subtly darkened by coverage.
// - Edge AA still derivative-based for stability.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

// Core iso & AA
const ISO: f32                  = 0.50;
const EDGE_BAND: f32            = 1.50;
const USE_DERIV_EDGE: bool      = true;

// Bevel controls
// BEVEL_PX: interior distance (in pixels) over which we transition from surface shading toward interior (t=0 at surface, t=1 deep).
const BEVEL_PX: f32             = 18.0;
const BEVEL_CURVE_EXP: f32      = 1.4;   // >1 = tighter highlight band near edge, <1 = broader
// Optional second shaping (set to 1.0 to disable)
const BEVEL_SECOND_EXP: f32     = 1.0;   // Use e.g. 0.8 then pow again with BEVEL_CURVE_EXP for S-shaped profile

// Interior flattening: distance beyond which interior is a flat plateau (no curvature shading or highlights)
// FLAT_PX should be >= BEVEL_PX so plateau starts after bevel ramp completes.
const FLAT_PX: f32              = 42.0;  // Increase for larger flat region (should scale with typical blob radius)
// Edge falloff shaping for shading effects (spec/fresnel/tint) separate from geometric bevel shape.
const EDGE_FADE_EXP: f32        = 1.3;   // Controls how quickly edge effects fade toward center (higher = narrower ring)

// Lighting style
const NORMAL_Z: f32             = 0.65;  // Lower than old 0.75 to flatten center
const AMBIENT: f32              = 0.18;
const DIFFUSE_INT: f32          = 0.90;

const BASE_COLOR: vec3<f32>     = vec3<f32>(0.58, 0.55, 0.56);

// Specular (scaled by gradient magnitude to avoid center pinpoint)
const SPEC_POW: f32             = 34.0;  // Lowered from 64
const SPEC_INT: f32             = 0.55;
const SPEC_GRAD_SCALE: f32      = 0.10;  // Multiplies |∇| to map into 0..1 region

// Fresnel / rim
const FRES_INT: f32             = 0.55;
const FRES_POW: f32             = 3.0;

// Edge wetness tint (toward cooler hue)
const EDGE_COLOR: vec3<f32>     = vec3<f32>(0.40, 0.52, 0.70);
const EDGE_MIX: f32             = 0.35;  // Mix strength at surface t=0

// Shadow taps
const SHADOW_OFF: vec2<f32>     = vec2<f32>(0.004, -0.006);
const SHADOW_SOFT: f32          = 0.75;
const SHADOW_INT: f32           = 0.90;

// Background gradient
const BG_TOP:  vec3<f32>        = vec3<f32>(0.06, 0.07, 0.10);
const BG_BOT:  vec3<f32>        = vec3<f32>(0.02, 0.03, 0.05);

// Light direction (unnormalized)
const LIGHT_DIR_UNNORM: vec3<f32> = vec3<f32>(-0.6, 0.5, 1.0);

// Utility
fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> { return a * (1.0 - t) + b * t; }
fn sample_packed(uv: vec2<f32>) -> vec4<f32> { return textureSampleLevel(present_tex, present_sampler, uv, 0.0); }
fn approx_sd(field: f32, iso: f32, inv_grad_len: f32) -> f32 { if (inv_grad_len <= 0.0) { return 0.0; } return (field - iso) * inv_grad_len; }

// Bevel parameter t: 0 at surface (iso), 1 at or past BEVEL_PX into interior.
fn bevel_t(sd_px: f32) -> f32 {
  var t = clamp(sd_px / BEVEL_PX, 0.0, 1.0);
  if (BEVEL_SECOND_EXP != 1.0) { t = pow(t, BEVEL_SECOND_EXP); }
  return pow(t, BEVEL_CURVE_EXP);
}

// Fresnel
fn fresnel(dot_nv: f32) -> f32 { return pow(1.0 - max(dot_nv, 0.0), FRES_POW) * FRES_INT; }

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
  let uv = v.uv;

  // Texture metrics
  let dims_u = textureDimensions(present_tex, 0);
  let dims   = vec2<f32>(f32(dims_u.x), f32(dims_u.y));

  // Cover fit for square baked field
  var sample_uv = uv;
  let aspect = dims.x / dims.y;
  if (aspect > 1.0) {
    let scale = aspect;
    sample_uv.y = (uv.y - 0.5) * scale + 0.5;
  } else if (aspect < 1.0) {
    let scale = 1.0 / aspect;
    sample_uv.x = (uv.x - 0.5) * scale + 0.5;
  }

  // Background gradient (original uv)
  let bg = lerp(BG_BOT, BG_TOP, clamp(uv.y, 0.0, 1.0));

  // Fetch packed
  let packed = sample_packed(sample_uv);
  let field = packed.r;
  let ngrad = vec2<f32>(packed.g, packed.b);
  let inv_grad_len = packed.a;

  // Edge AA width
  var w = 1e-4;
  if (USE_DERIV_EDGE) {
    w = max(fwidth(field) * EDGE_BAND, 1e-4);
  } else {
    let est = inv_grad_len * 0.5;
    w = clamp(est, 0.001, 0.05);
  }

  // Coverage (smooth transition strictly around iso)
  let inside_mask = smoothstep(ISO - w, ISO + w, field);

  // Signed distance (positive inside)
  let sd = approx_sd(field, ISO, inv_grad_len);
  // Convert to pixel distance (assumes square field domain)
  let sd_px = sd * dims.x;
  // Bevel parameter (geometric shading ramp) limited to FLAT_PX domain
  let t_bevel = bevel_t(min(sd_px, BEVEL_PX)); // 0 surface → 1 deep interior (capped at BEVEL_PX)

  // Edge factor: 1 at surface, 0 at/after FLAT_PX (used to extinguish spec/fresnel/edge tint inside)
  var edge_factor = 0.0;
  if (FLAT_PX > BEVEL_PX) {
    let x = clamp(1.0 - (sd_px - BEVEL_PX) / max(FLAT_PX - BEVEL_PX, 1e-4), 0.0, 1.0);
    edge_factor = pow(x, EDGE_FADE_EXP);
  } else {
    // Degenerate case: no flat region requested -> treat entire interior with taper by bevel t
    edge_factor = pow(1.0 - t_bevel, EDGE_FADE_EXP);
  }

  // Normal (flatten z to reduce central hotspot); further flatten interior by blending to view normal using edge_factor
  let rawN = normalize(vec3<f32>(-ngrad.x, -ngrad.y, NORMAL_Z));
  let flatN = vec3<f32>(0.0, 0.0, 1.0);
  let N = normalize(mix(flatN, rawN, edge_factor));
  let L = normalize(LIGHT_DIR_UNNORM);
  let V = vec3<f32>(0.0, 0.0, 1.0);
  let H = normalize(L + V);

  // Lighting factors
  let ndl = max(dot(N, L), 0.0);
  let diffuse = ndl * DIFFUSE_INT;

  // Gradient length
  var grad_len = 0.0;
  if (inv_grad_len > 0.0) { grad_len = 1.0 / inv_grad_len; }
  let spec_scale = clamp(grad_len * SPEC_GRAD_SCALE, 0.0, 1.0) * edge_factor;
  let spec = pow(max(dot(N, H), 0.0), SPEC_POW) * SPEC_INT * spec_scale;
  let fr = fresnel(dot(N, V)) * edge_factor;

  // Edge tint — strongest at surface (t_bevel ~0), fades interior
  // Edge tint only near edge; interior plateaus to BASE_COLOR
  let edge_tint_mix = (1.0 - t_bevel) * edge_factor; // stronger near surface before flat region
  let base_col = lerp(BASE_COLOR, EDGE_COLOR, edge_tint_mix * EDGE_MIX);

  var blob_rgb = base_col * (AMBIENT + diffuse) + spec + fr;

  // Soft shadow (triple sample)
  let sh_uv0 = sample_uv + SHADOW_OFF * 0.5;
  let sh_uv1 = sample_uv + SHADOW_OFF * (1.0 + SHADOW_SOFT);
  let sh_uv2 = sample_uv + SHADOW_OFF * (1.5 + 2.0 * SHADOW_SOFT);
  let sh0 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv0).r);
  let sh1 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv1).r);
  let sh2 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv2).r);
  let shadow = (sh0 + sh1 + sh2) / 3.0;
  let bg_shadowed = lerp(bg, bg * 0.40, clamp(shadow * SHADOW_INT, 0.0, 1.0));

  // Composite (no glow / outline)
  var out_rgb = bg_shadowed;
  out_rgb = lerp(out_rgb, blob_rgb, inside_mask);
  return vec4<f32>(out_rgb, 1.0);
}