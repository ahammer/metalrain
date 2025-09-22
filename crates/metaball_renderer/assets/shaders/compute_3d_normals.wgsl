// Single-profile interior bevel normals.
// Field assumptions:
//   field == ISO                => exact silhouette edge
//   field in (ISO, INTERIOR_FIELD) maps to interior bevel ramp (height 0 -> 1)
//   field >= INTERIOR_FIELD     => flat interior plateau (height 1)
// Output (rgba16f):
//   RGB: pseudo 3D normal
//   A:   height (0..1)
//
// Tune INTERIOR_FIELD to control the inset thickness in *field units* (not pixels).
// If you prefer pixel-accurate sizing, you'd keep a thickness in pixels and use grad_len
// to convert; here we adopt a simpler "field range" specification for clarity.

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,          // (unused, kept for layout stability)
  clustering_enabled: u32, // (unused)
};

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var normals_tex: texture_storage_2d<rgba16float, write>;

const ISO: f32 = 1.0;
// Field value inside at which height should reach 1.0 (configurable "inset depth").
const INTERIOR_FIELD: f32 = 2.00;   // 1.0 -> 1.15 range becomes the bevel. Adjust as desired.

// Vertical exaggeration for perceived bulge (scales XY slope before normalization).
const NORMAL_Z_SCALE: f32 = 10.0;

// Set this to 0 for linear, 1 for eased (quadratic ease-out).
const USE_EASED_PROFILE: u32 = 1u;

@compute @workgroup_size(8, 8, 1)
fn compute_normals(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) {
    return;
  }

  let coord = vec2<i32>(i32(gid.x), i32(gid.y));
  let packed = textureLoad(field_tex, coord);

  let field = packed.r;
  let grad_dir = vec2<f32>(packed.g, packed.b); // normalized (or near-normalized) field gradient direction
  let inv_grad_len = packed.a;

  // Guard degenerate cases.
  let grad_len = select(0.0, 1.0 / inv_grad_len, inv_grad_len > 0.0);

  // Quick outs:
  if (field <= ISO || grad_len == 0.0) {
    // Outside (or exactly at) iso, or flat region with no gradient: zero height and flat normal.
    textureStore(normals_tex, coord, vec4<f32>(0.0, 0.0, 1.0, 0.0));
    return;
  }

  // Compute normalized interior progress u in [0,1].
  let range = max(INTERIOR_FIELD - ISO, 1e-6);
  let u = clamp((field - ISO) / range, 0.0, 1.0);

  // Height + derivative wrt field (dh/df).
  var height: f32;
  var dh_df: f32;

  if (USE_EASED_PROFILE == 1u) {
    // Ease-out profile: h = 1 - (1 - u)^2 = 2u - u^2
    // dh/du = 2 - 2u
    height = 1.0 - (1.0 - u) * (1.0 - u);
    dh_df = (2.0 - 2.0 * u) / range;
  } else {
    // Linear: h = u
    height = u;
    dh_df = 1.0 / range;
  }

  // slope_factor = (dh/df) * |∇field|
  let slope_factor = dh_df * grad_len;

  // Horizontal components of -∇h (outward) since grad_dir points inward.
  let horiz = -grad_dir * slope_factor;

  // Build & normalize pseudo 3D normal.
  var n = vec3<f32>(horiz.x * NORMAL_Z_SCALE, horiz.y * NORMAL_Z_SCALE, 1.0);
  n = normalize(n);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
