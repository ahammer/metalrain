// Secondary compute pass: derive faux 3D normals + height from packed field texture.
// Inputs (from compute_metaballs.wgsl output):
//  R: field value
//  G: normalized gradient x
//  B: normalized gradient y
//  A: inverse gradient length (inv_grad_len)
// Output (normals texture RGBA16F):
//  RGB: normalized 3D normal (xy from gradient, z is up)
//  A:   eased height (0..1) using inOutCirc on signed-distance remap.
// NOTE: Present shader currently samples but does not use this texture yet.

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,             // unused here
  clustering_enabled: u32,    // unused here
}

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var<uniform> params: Params; // for screen_size
@group(0) @binding(2) var normals_tex: texture_storage_2d<rgba16float, write>;

// Banded normal shaping:
// Everything below BAND_MIN and above BAND_MAX becomes flat (normal = (0,0,1), height plateaus 0 / 1).
// Only the field interval [BAND_MIN, BAND_MAX] produces curvature & eased height.
const BAND_MIN: f32 = 0.0;
const BAND_MAX: f32 = 1.0;
const BAND_WIDTH: f32 = (BAND_MAX - BAND_MIN);
const NORMAL_Z_SCALE: f32 = 10.0;    // Vertical exaggeration inside the active band

fn in_out_circ(u: f32) -> f32 {
  if (u < 0.5) {
    let a = 2.0 * u;
    return 0.5 * (1.0 - sqrt(1.0 - a * a));
  }
  let b = -2.0 * u + 2.0;
  return 0.5 * (sqrt(1.0 - b * b) + 1.0);
}
fn in_out_circ_derivative(u: f32) -> f32 {
  if (u < 0.5) {
    let a = 2.0 * u;
    return a / sqrt(max(1.0 - a * a, 1e-6));
  }
  let b = -2.0 * u + 2.0;
  return b / sqrt(max(1.0 - b * b, 1e-6));
}

@compute @workgroup_size(8,8,1)
fn compute_normals(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) { return; }
  let coord = vec2<i32>(i32(gid.x), i32(gid.y));
  // storage textures use textureLoad(texture, coords) (no mip level param)
  let packed = textureLoad(field_tex, coord);
  let field = packed.r;
  let gx = packed.g;
  let gy = packed.b;
  let inv_grad_len = packed.a;

  // Clamp field into band then remap to [0,1]. Outside band this produces plateaus (u=0 or 1).
  let field_clamped = clamp(field, BAND_MIN, BAND_MAX);
  let u = (field_clamped - BAND_MIN) / max(BAND_WIDTH, 1e-6);

  // Height easing (inOutCirc) gives smooth curvature only inside band.
  let height = in_out_circ(u);

  // Determine if fragment actually lies inside band (not just clamped to an edge).
  let in_band = field >= BAND_MIN && field <= BAND_MAX;
  let grad_valid = inv_grad_len > 0.0;

  // d(height)/d(field) only meaningful inside band.
  var slope_factor = 0.0;
  if (in_band && grad_valid) {
    let dh_df = in_out_circ_derivative(u) * (1.0 / max(BAND_WIDTH, 1e-6));
    let grad_len = 1.0 / max(inv_grad_len, 1e-6); // |grad(field)|
    slope_factor = dh_df * grad_len;
  }

  var horiz = vec2<f32>(0.0, 0.0);
  if (slope_factor != 0.0) { horiz = -vec2<f32>(gx, gy) * slope_factor * NORMAL_Z_SCALE; }

  var n = vec3<f32>(horiz.x, horiz.y, 1.0);
  n = normalize(n);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
