// Pseudo‑3D normals: stable screen‑space thickness band around ISO to avoid sub‑pixel flicker.
// Inputs (field pass):
//  R: field value
//  G: normalized gradient x
//  B: normalized gradient y
//  A: inverse gradient length (inv_grad_len)
// Output (rgba16f):
//  RGB: pseudo 3D normal
//  A:   height (smooth bump) inside band, 0 outside

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,          // unused
  clustering_enabled: u32, // unused
};

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var normals_tex: texture_storage_2d<rgba16float, write>;

const ISO: f32 = 1.0;

// Desired half thickness in SCREEN PIXELS (so visible width = ~2 * HALF_THICKNESS_PX).
// Increase if still flickering (e.g. 3.0 - 4.0), decrease for crisper rim.
const HALF_THICKNESS_PX: f32 = 2.5;

// Vertical exaggeration for surface curvature.
const NORMAL_Z_SCALE: f32 = 8.0;

@compute @workgroup_size(8, 8, 1)
fn compute_normals(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) {
    return;
  }

  let coord = vec2<i32>(i32(gid.x), i32(gid.y));
  let packed = textureLoad(field_tex, coord);

  let field = packed.r;
  let grad_dir = vec2<f32>(packed.g, packed.b); // unit (or near-unit) direction
  let inv_grad_len = packed.a;

  // Reconstruct gradient length; guard zero.
  let grad_len = select(0.0, 1.0 / inv_grad_len, inv_grad_len > 0.0);

  // Signed offset from iso in field units.
  let s = field - ISO;

  // Approximate pixel distance from iso surface.
  // dist_px ≈ |s| * |∇field| (since field changes ~ grad_len per pixel screen-space).
  let dist_px = abs(s) * grad_len;

  // Inside band if within desired screen-space half thickness.
  let inside = dist_px < HALF_THICKNESS_PX;

  // k scales field offset into normalized band space: when |s| = HALF_THICKNESS_PX / grad_len => k^2 * s^2 = 1.
  // Avoid division by zero (if HALF_THICKNESS_PX very small or grad_len=0).
  let safe_half = max(HALF_THICKNESS_PX, 1e-6);
  let k = grad_len / safe_half;

  // base = 1 - (k^2 * s^2). We square 'base' for smoother derivative (zero at edge) reducing flicker.
  let ks = k * s;
  let ks2 = ks * ks;
  let base = 1.0 - ks2;
  let height = select(0.0, base * base, inside);

  // Derivative dh/ds inside band:
  // h = (1 - k^2 s^2)^2
  // dh/ds = -4 k^2 s (1 - k^2 s^2)
  // (Treat k constant per pixel; variation of k not differentiated for stability.)
  let dh_ds = select(0.0, -4.0 * (k * k) * s * base, inside);

  // Slope factor multiplies gradient vector (|∇field| already separated).
  let slope_factor = dh_ds * grad_len;

  // Horizontal normal components (negated gradient scaled by slope, masked).
  let horiz = -grad_dir * slope_factor * f32(inside);

  // Strengthen curvature visually.
  let horiz_scaled = horiz * NORMAL_Z_SCALE;

  var n = vec3<f32>(horiz_scaled.x, horiz_scaled.y, 1.0);
  n = normalize(n);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
