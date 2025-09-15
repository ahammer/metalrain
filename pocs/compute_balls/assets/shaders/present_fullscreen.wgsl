// Present fragment shader for Material2d (Bevy 0.16).
// Bindings: group(2) -> 0: texture_2d<f32>, 1: sampler
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

// -----------------------------------
// Hardwired look (tweak & recompile)
// -----------------------------------
const ISO: f32                 = 0.50;   // isosurface threshold
const EDGE_BAND: f32           = 1.50;   // AA expansion over fwidth(field)
const NORMAL_Z: f32            = 0.75;   // pseudo-3D roundness
const AMBIENT: f32             = 0.12;

const BASE_COLOR: vec3<f32>    = vec3<f32>(0.55, 0.48, 0.50);
const LIGHT_DIR_UNNORM: vec3<f32> = vec3<f32>(-0.6, 0.5, 1.0);
const SPEC_POW: f32            = 64.0;
const SPEC_INT: f32            = 0.45;
const RIM_POW: f32             = 2.0;
const RIM_INT: f32             = 0.25;

const OUTLINE_W: f32           = 1.5;    // measured in AA-band units
const OUTLINE_INT: f32         = 0.60;
const OUTLINE_COLOR: vec3<f32> = vec3<f32>(0.05, 0.10, 0.18);

const GLOW_W: f32              = 2.5;    // in approx SDF units
const GLOW_INT: f32            = 0.35;
const GLOW_COLOR: vec3<f32>    = vec3<f32>(0.35, 0.55, 1.0);

const SHADOW_OFF: vec2<f32>    = vec2<f32>(0.004, -0.006); // UV space
const SHADOW_SOFT: f32         = 0.75;   // 0..1
const SHADOW_INT: f32          = 0.90;

// If true, use hardware derivatives for normals (0 extra texture taps).
// If false, use 4-tap central differences (smoother in some content).
const USE_DERIV_NORMALS: bool  = true;

// -----------------------------------
// Helpers
// -----------------------------------
fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
  return a * (1.0 - t) + b * t;
}

fn sample_field(uv: vec2<f32>) -> f32 {
  return textureSampleLevel(present_tex, present_sampler, uv, 0.0).r;
}

// Central-difference gradient (texture-space)
fn central_grad(uv: vec2<f32>, texel: vec2<f32>) -> vec2<f32> {
  let fx1 = sample_field(uv + vec2<f32>(texel.x, 0.0));
  let fx0 = sample_field(uv - vec2<f32>(texel.x, 0.0));
  let fy1 = sample_field(uv + vec2<f32>(0.0, texel.y));
  let fy0 = sample_field(uv - vec2<f32>(0.0, texel.y));
  return 0.5 * vec2<f32>(fx1 - fx0, fy1 - fy0);
}

// Local signed-distance approximation near the iso
fn approx_signed_dist(field: f32, iso: f32, grad: vec2<f32>) -> f32 {
  let gl = max(length(grad), 1e-6);
  return (field - iso) / gl;
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
  let uv = v.uv;

  // Texture metrics
  let dims_u = textureDimensions(present_tex, 0);
  let dims   = vec2<f32>(f32(dims_u.x), f32(dims_u.y));
  let texel  = 1.0 / max(dims, vec2<f32>(1.0));

  // Sample field
  let field = sample_field(uv);

  // AA width (stable edge)
  let w = max(fwidth(field) * EDGE_BAND, 1e-4);
  let inside_mask = smoothstep(ISO - w, ISO + w, field);

  // Gradient for normals
  var grad = vec2<f32>(0.0, 0.0);
  if (USE_DERIV_NORMALS) {
    grad = vec2<f32>(dpdx(field), dpdy(field));
  } else {
    grad = central_grad(uv, texel);
  }

  // Signed-distance approx (positive inside if field>ISO)
  let sd = approx_signed_dist(field, ISO, grad);

  // Pseudo 2.5D normal & lighting
  let N = normalize(vec3<f32>(-grad.x, -grad.y, NORMAL_Z));
  let L = normalize(LIGHT_DIR_UNNORM);
  let V = vec3<f32>(0.0, 0.0, 1.0);
  let H = normalize(L + V);

  let ndl = max(dot(N, L), 0.0);
  let diffuse = ndl;
  let spec = pow(max(dot(N, H), 0.0), SPEC_POW) * SPEC_INT;
  let rim  = pow(1.0 - max(dot(N, V), 0.0), RIM_POW) * RIM_INT;

  var blob_rgb = BASE_COLOR * (AMBIENT + diffuse) + spec + rim;

  // Outline band (straddles both sides of the iso)
  let ow = OUTLINE_W * w;
  let edge_lo = smoothstep(ISO - ow, ISO + ow, field);
  let edge_hi = smoothstep(ISO - ow * 0.5, ISO + ow * 0.5, field);
  let outline = clamp(edge_hi - edge_lo, 0.0, 1.0);

  // Soft drop shadow: 3 taps along offset
  let sh_uv0 = uv + SHADOW_OFF * 0.5;
  let sh_uv1 = uv + SHADOW_OFF * (1.0 + SHADOW_SOFT);
  let sh_uv2 = uv + SHADOW_OFF * (1.5 + 2.0 * SHADOW_SOFT);

  let sh0 = smoothstep(ISO - w, ISO + w, sample_field(sh_uv0));
  let sh1 = smoothstep(ISO - w, ISO + w, sample_field(sh_uv1));
  let sh2 = smoothstep(ISO - w, ISO + w, sample_field(sh_uv2));
  let shadow = (sh0 + sh1 + sh2) / 3.0;

  // Background (simple vertical gradient) + shadow
  let bg = lerp(vec3<f32>(0.02, 0.03, 0.05), vec3<f32>(0.06, 0.07, 0.10), clamp(uv.y, 0.0, 1.0));
  let bg_shadowed = lerp(bg, bg * 0.4, clamp(shadow * SHADOW_INT, 0.0, 1.0));

  // Glow mainly outside the surface (Gaussian-ish on -sd)
  let outside_sd = max(-sd, 0.0);
  let glow_profile = exp(- (outside_sd / max(GLOW_W, 1e-4)) * (outside_sd / max(GLOW_W, 1e-4)));
  let glow_rgb = GLOW_COLOR * (glow_profile * GLOW_INT);

  // Composite:
  // 1) Background + outside glow
  // 2) Inside blob shading over that
  var out_rgb = bg_shadowed + glow_rgb * (1.0 - inside_mask);
  out_rgb = lerp(out_rgb, blob_rgb, inside_mask);

  // Apply outline on top so it affects both sides
  out_rgb = lerp(out_rgb, OUTLINE_COLOR, outline * OUTLINE_INT);

  return vec4<f32>(out_rgb, 1.0);
}
