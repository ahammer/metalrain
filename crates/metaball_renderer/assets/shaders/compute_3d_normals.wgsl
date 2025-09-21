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

const ISO: f32 = 0.80;               // Keep in sync with present shader constant for now
const HEIGHT_HALF_EXTENT: f32 = 4.0; // Tunable: widens plateau regions
const NORMAL_Z_SCALE: f32 = 40.0;    // Vertical exaggeration before normalization

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
  let packed = textureLoad(field_tex, coord, 0);
  let field = packed.r;
  let gx = packed.g;
  let gy = packed.b;
  let inv_grad_len = packed.a;

  // Signed distance approximation
  let sd = (field - ISO) * inv_grad_len;
  let h_extent = max(HEIGHT_HALF_EXTENT, 1e-4);
  let sd_clamped = clamp(sd, -h_extent, h_extent);
  let u = 0.5 + 0.5 * (sd_clamped / h_extent); // maps [-H,0,H] -> [0,0.5,1]

  let height = in_out_circ(u);
  let du_dsd = 0.5 / h_extent;
  let slope_factor = in_out_circ_derivative(u) * du_dsd;

  let grad_valid = inv_grad_len > 0.0;
  var horiz = vec2<f32>(0.0, 0.0);
  if (grad_valid) { horiz = -vec2<f32>(gx, gy) * slope_factor * NORMAL_Z_SCALE; }

  var n = vec3<f32>(horiz.x, horiz.y, 1.0);
  n = normalize(n);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
