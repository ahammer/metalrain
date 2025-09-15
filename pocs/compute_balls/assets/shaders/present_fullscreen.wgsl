// Present fragment shader utilizing packed metadata from compute pass.
// Incoming texture channels (rgba16float):
//   R: field
//   G: normalized gradient x
//   B: normalized gradient y
//   A: inverse gradient length (1/|∇field|), 0 if |∇| tiny
//
// Removes need for derivative or 4-tap gradient reconstruction, enabling
// cleaner normals and a stable signed distance for shading, glow & outlines.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

// Tunable constants (unchanged semantics)
const ISO: f32                 = 0.50;
const EDGE_BAND: f32           = 1.50;
const NORMAL_Z: f32            = 0.75;
const AMBIENT: f32             = 0.12;

const BASE_COLOR: vec3<f32>    = vec3<f32>(0.55, 0.48, 0.50);
const LIGHT_DIR_UNNORM: vec3<f32> = vec3<f32>(-0.6, 0.5, 1.0);
const SPEC_POW: f32            = 64.0;
const SPEC_INT: f32            = 0.45;
const RIM_POW: f32             = 2.0;
const RIM_INT: f32             = 0.25;

const OUTLINE_W: f32           = 1.5;
const OUTLINE_INT: f32         = 0.60;
const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.05, 0.10, 0.18);

const GLOW_W: f32              = 2.5;
const GLOW_INT: f32            = 0.35;
const GLOW_COLOR: vec3<f32>    = vec3<f32>(0.35, 0.55, 1.0);

const SHADOW_OFF: vec2<f32>    = vec2<f32>(0.004, -0.006);
const SHADOW_SOFT: f32         = 0.75;
const SHADOW_INT: f32          = 0.90;

// Derivative-based AA still used for edge width (more stable under zoom)
const USE_DERIV_EDGE: bool     = true;

fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return a * (1.0 - t) + b * t;
}

fn sample_packed(uv: vec2<f32>) -> vec4<f32> {
  return textureSampleLevel(present_tex, present_sampler, uv, 0.0);
}

// Approx signed distance using pre-packed inverse gradient length
fn approx_sd(field: f32, iso: f32, inv_grad_len: f32) -> f32 {
  if (inv_grad_len <= 0.0) {
    return 0.0;
  }
  return (field - iso) * inv_grad_len;
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
  let uv = v.uv;

  // Texture metrics
  let dims_u = textureDimensions(present_tex, 0);
  let dims   = vec2<f32>(f32(dims_u.x), f32(dims_u.y));

  // --- ASPECT RATIO ADJUST (COVER) ------------------------------------------
  // We have a square baked field texture. We want a CSS 'cover' style fit:
  // keep the field square aspect, fill the entire viewport, cropping excess.
  // That means we scale the shorter axis up so there is no letter/pillar box.
  // sample_uv in 0..1 maps to the square field; values outside are cropped.
  var sample_uv = uv;
  let aspect = dims.x / dims.y; // >1 => wide, <1 => tall
  if (aspect > 1.0) {
    // Wider surface: y is the shorter axis. Expand y range so square covers.
    // Inverse of previous contain logic: stretch sample coordinates in y.
    let scale = aspect; // >1
    sample_uv.y = (uv.y - 0.5) * scale + 0.5;
  } else if (aspect < 1.0) {
    // Taller surface: x is the shorter axis. Expand x range.
    let scale = 1.0 / aspect; // >1
    sample_uv.x = (uv.x - 0.5) * scale + 0.5;
  }

  // Background gradient spans full viewport (use original uv) and will be
  // fully covered by blob shading since we no longer early-return outside.
  let bg = lerp(
    vec3<f32>(0.02, 0.03, 0.05),
    vec3<f32>(0.06, 0.07, 0.10),
    clamp(uv.y, 0.0, 1.0)
  );
  // NOTE: No early return. Regions where sample_uv is outside 0..1 simply
  // sample the texture outside its baked domain (assumed clamped by sampler)
  // or produce border; gradients there will fade due to field values.
  // -------------------------------------------------------------------------

  // Fetch packed data with cover-adjusted UV
  let packed = sample_packed(sample_uv);
  let field = packed.r;
  let ngrad = vec2<f32>(packed.g, packed.b); // normalized gradient
  let inv_grad_len = packed.a;

  // Edge anti-alias width
  var w = 1e-4;
  if (USE_DERIV_EDGE) {
    w = max(fwidth(field) * EDGE_BAND, 1e-4);
  } else {
    // Fallback heuristic: scale with inverse gradient length
    // (Smaller gradients -> larger transition)
    let est = inv_grad_len * 0.5;
    w = clamp(est, 0.001, 0.05);
  }

  // Coverage via smoothstep around iso
  let inside_mask = smoothstep(ISO - w, ISO + w, field);

  // Signed distance approximation (positive inside)
  let sd = approx_sd(field, ISO, inv_grad_len);

  // Normal from packed normalized gradient; z biases volume feel
  let N = normalize(vec3<f32>(-ngrad.x, -ngrad.y, NORMAL_Z));
  let L = normalize(LIGHT_DIR_UNNORM);
  let V = vec3<f32>(0.0, 0.0, 1.0);
  let H = normalize(L + V);

  let ndl = max(dot(N, L), 0.0);
  let diffuse = ndl;
  let spec = pow(max(dot(N, H), 0.0), SPEC_POW) * SPEC_INT;
  let rim  = pow(1.0 - max(dot(N, V), 0.0), RIM_POW) * RIM_INT;

  // Thickness heuristic (inverse of inv_grad_len) for subtle color modulation
  var thickness = 0.0;
  if (inv_grad_len > 0.0) {
    // thickness ~ |∇| scaled into 0..1 range
    let grad_len = 1.0 / inv_grad_len;
    thickness = clamp(grad_len * 0.02, 0.0, 1.0);
  }

  // Base shading with slight hue shift by thickness
  let tint = lerp(vec3<f32>(0.90, 0.85, 0.80), vec3<f32>(0.40, 0.35, 0.50), thickness);
  var blob_rgb = (BASE_COLOR * tint) * (AMBIENT + diffuse) + spec + rim;

  // Outline band (same logic, reuse w)
  let ow = OUTLINE_W * w;
  let edge_lo = smoothstep(ISO - ow, ISO + ow, field);
  let edge_hi = smoothstep(ISO - ow * 0.5, ISO + ow * 0.5, field);
  let outline = clamp(edge_hi - edge_lo, 0.0, 1.0);

  // Shadow taps (still rely only on field channel) within square UV space
  let sh_uv0 = sample_uv + SHADOW_OFF * 0.5;
  let sh_uv1 = sample_uv + SHADOW_OFF * (1.0 + SHADOW_SOFT);
  let sh_uv2 = sample_uv + SHADOW_OFF * (1.5 + 2.0 * SHADOW_SOFT);

  let sh0 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv0).r);
  let sh1 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv1).r);
  let sh2 = smoothstep(ISO - w, ISO + w, sample_packed(sh_uv2).r);
  let shadow = (sh0 + sh1 + sh2) / 3.0;

  let bg_shadowed = lerp(bg, bg * 0.4, clamp(shadow * SHADOW_INT, 0.0, 1.0));

  // Glow primarily outside
  let outside_sd = max(-sd, 0.0);
  let glow_profile = exp(- (outside_sd / max(GLOW_W, 1e-4)) * (outside_sd / max(GLOW_W, 1e-4)));
  let glow_rgb = GLOW_COLOR * (glow_profile * GLOW_INT);

  // Composite
  var out_rgb = bg_shadowed + glow_rgb * (1.0 - inside_mask);
  out_rgb = lerp(out_rgb, blob_rgb, inside_mask);
  out_rgb = lerp(out_rgb, OUTLINE_COLOR, outline * OUTLINE_INT);

  return vec4<f32>(out_rgb, 1.0);
}