// Pseudo‑3D normals: stable screen‑space thickness band around ISO to avoid sub‑pixel flicker.
// Inputs (field pass):
//  R: field value
//  G: normalized gradient x
//  B: normalized gradient y
//  A: inverse gradient length (inv_grad_len)
// Output (rgba16f):
//  RGB: pseudo 3D normal
//  A:   height (profile-dependent) (0 outside lighting region)
//
// PROFILE MODES:
// 0 = Legacy symmetric band (old behavior)
// 1 = Monotonic interior-only (peak slope at iso, height grows inward)  [DEFAULT]
// 2 = Gaussian interior-only (softer falloff; parameterized by GAUSS_WIDTH_PX)

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,          // unused
  clustering_enabled: u32, // unused
};

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var normals_tex: texture_storage_2d<rgba16float, write>;

const ISO: f32 = 0.5;

// Screen‑space target half thickness (pixels) for legacy & interior profiles.
const HALF_THICKNESS_PX: f32 = 5.5;

// Vertical exaggeration for surface curvature (affects perceived “bulge”).
const NORMAL_Z_SCALE: f32 = 10.0;

// Profile selection.
const PROFILE_MODE: u32 = 1u;          // 0 legacy symmetric, 1 interior monotonic, 2 gaussian
const GAUSS_WIDTH_PX: f32 = 10.0;       // Used only in mode 2

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

  // Signed offset from iso in field units (positive inside, field grows toward centers).
  let s = field - ISO;

  // Approximate pixel distance from iso surface (always positive).
  let dist_px = abs(s) * grad_len;

  // Outputs we will determine per profile:
  var height: f32 = 0.0;
  var slope_factor: f32 = 0.0;

  if (PROFILE_MODE == 0u) {
    // --- Legacy symmetric band (original) ---
    let inside = dist_px < HALF_THICKNESS_PX;
    let safe_half = max(HALF_THICKNESS_PX, 1e-6);
    let k = grad_len / safe_half;
    let ks = k * s;
    let ks2 = ks * ks;
    let base = 1.0 - ks2;
    height = select(0.0, base * base, inside);

    // dh/ds = -4 k^2 s (1 - k^2 s^2)
    let dh_ds = select(0.0, -4.0 * (k * k) * s * base, inside);
    slope_factor = dh_ds * grad_len;
  } else if (PROFILE_MODE == 1u) {
    // --- Monotonic interior-only profile ---
    // We only light interior (field > ISO). No exterior band -> highlight hugs silhouette.
    let inside = field > ISO;
    if (inside) {
      // Interior pixel distance from iso:
      let d = (field - ISO) * grad_len;
      let half = max(HALF_THICKNESS_PX, 1e-6);
      let t = clamp(d / half, 0.0, 1.0);
      // Height starts 0 at iso (t=0) and shrinks toward interior (t→1):
      // Choose h = (1 - t)^2 (smooth, with zero slope at deep interior).
      height = (1.0 - t) * (1.0 - t);

      // Derivative wrt t: dh/dt = -2 (1 - t)
      // dt/ds = grad_len / half   (since d = s * grad_len, s>0 inside)
      // dh/ds = -2 (1 - t) * grad_len / half
      let dh_ds = -2.0 * (1.0 - t) * (grad_len / half);

      slope_factor = dh_ds * grad_len;
    }
  } else {
    // --- Gaussian interior-only profile (PROFILE_MODE == 2) ---
    // h = exp(-a * d^2), a chosen from width so that d = GAUSS_WIDTH_PX -> small value.
    let inside = field > ISO;
    if (inside) {
      let d = (field - ISO) * grad_len;
      let width = max(GAUSS_WIDTH_PX, 1e-6);
      let a = 1.0 / (width * width);          // controlling falloff
      height = exp(-a * d * d);
      // dh/dd = -2 a d * exp(-a d^2) = -2 a d * h
      // dd/ds = grad_len
      // dh/ds = (-2 a d) * h * grad_len
      let dh_ds = (-2.0 * a * d) * height * grad_len;
      slope_factor = dh_ds * grad_len;
    }
  }

  // Horizontal normal components (negative gradient gives outward surface direction).
  let horiz = -grad_dir * slope_factor;

  // Apply curvature exaggeration then compose with Z.
  var n = vec3<f32>(horiz.x * NORMAL_Z_SCALE, horiz.y * NORMAL_Z_SCALE, 1.0);
  n = normalize(n);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
