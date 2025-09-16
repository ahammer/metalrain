// Packed output layout (rgba16float):
//   R: field value (Σ r_i^2 / d_i^2)
//   G: normalized gradient x (unit, 0 if length ~ 0)
//   B: normalized gradient y
//   A: inverse gradient length (1/|∇|), clamped; 0 if |∇| tiny.
//
// This allows the present shader to:
//   * Reconstruct a normal from (G,B) without extra texture taps
//   * Compute signed distance ≈ (field - ISO) * inv_grad_len
//   * Potentially vary shading by gradient magnitude (thickness)
//
// Contract preserved: same bindings, same texture format, no new uniforms.

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,
  _unused0: u32,
  iso: f32,
  _unused2: f32,
  _unused3: f32,
  _unused4: u32,
  clustering_enabled: u32,
}

struct TimeU {
  time: f32,
}

struct Ball {
  center: vec2<f32>,
  radius: f32,
  cluster_id: i32,
  color: vec4<f32>,
}

@group(0) @binding(0) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<uniform> time_u: TimeU;
@group(0) @binding(3) var<storage, read> balls: array<Ball>;
@group(0) @binding(4) var out_albedo: texture_storage_2d<rgba8unorm, write>;

const EPS: f32 = 1e-4;

// Fixed world-space bounds baked into the shader. The logical "central screen"
// region (entire render target) is mapped to [-200,-200] -> [200,200]. This
// provides a stable, resolution‑independent coordinate space for field
// evaluation and future effects (iso adjustments, SDF usage, etc.).
const WORLD_MIN: vec2<f32> = vec2<f32>(-1000.0, -1000.0);
const WORLD_MAX: vec2<f32> = vec2<f32>( 1000.0,  1000.0);
const WORLD_SIZE: vec2<f32> = WORLD_MAX - WORLD_MIN; 

// Convert pixel-space (0..screen_size) into world space using an affine map.
// Note: This applies per-axis scaling; if the render target is non-square the
// world aspect will stretch accordingly. If uniform scaling becomes desirable
// later, switch to a scalar based on min(screen_size.x, screen_size.y).
fn to_world(pixel: vec2<f32>) -> vec2<f32> {
  // Center of pixel: add 0.5 to reduce aliasing bias.
  let uv = (pixel + vec2<f32>(0.5, 0.5)) / params.screen_size;
  return WORLD_MIN + uv * WORLD_SIZE;
}

fn ball_center(i: u32) -> vec2<f32> {
  // Procedural wobble (unchanged) in pixel space, then map to world space.
  let b = balls[i];
  let phase = f32(i) * 0.37;
  let wobble = vec2<f32>(
    sin(time_u.time * 0.6 + phase) * 12.0,
    cos(time_u.time * 0.8 + phase * 1.7) * 9.0
  );
  let pixel_pos = b.center + wobble;
  return to_world(pixel_pos);
}

@compute @workgroup_size(8, 8, 1)
fn metaballs(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) { return; }

  // Sample point in world space (was pixel space previously).
  let coord = to_world(vec2<f32>(f32(gid.x), f32(gid.y)));

  var field: f32 = 0.0;
  var grad: vec2<f32> = vec2<f32>(0.0, 0.0);
  var blended_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
  let count = min(params.num_balls, arrayLength(&balls));

  // First pass: compute contribs and track dominant cluster by max contribution
  var max_contrib: f32 = 0.0;
  var dominant_cluster: i32 = 0;

  for (var i: u32 = 0u; i < count; i = i + 1u) {
    let c = ball_center(i);
    let d = coord - c;
    let dist2 = max(dot(d, d), EPS);

    let r = balls[i].radius;
    let r2 = r * r;

    let inv_dist2 = 1.0 / dist2;
    let contrib = r2 * inv_dist2; // r^2 / d^2
    field = field + contrib;

    let inv_dist4 = inv_dist2 * inv_dist2;
    let scale = -2.0 * r2 * inv_dist4; // shared factor
    grad = grad + scale * d;

    if (contrib > max_contrib) {
      max_contrib = contrib;
      dominant_cluster = balls[i].cluster_id;
    }
  }

  // If clustering is enabled, recompute field & gradient considering only dominant cluster
  if (params.clustering_enabled > 0u) {
    field = 0.0;
    grad = vec2<f32>(0.0, 0.0);
    var cluster_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < count; i = i + 1u) {
      if (balls[i].cluster_id == dominant_cluster) {
        let c = ball_center(i);
        let d = coord - c;
        let dist2 = max(dot(d, d), EPS);

        let r = balls[i].radius;
        let r2 = r * r;

        let inv_dist2 = 1.0 / dist2;
        let contrib = r2 * inv_dist2;
        field = field + contrib;

        let inv_dist4 = inv_dist2 * inv_dist2;
        let scale = -2.0 * r2 * inv_dist4;
        grad = grad + scale * d;

        cluster_color = balls[i].color;
      }
    }

    blended_color = cluster_color;
  } else {
    // clustering disabled: blend colors by influence
    if (field > 0.0) {
      var color_acc: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
      for (var i: u32 = 0u; i < count; i = i + 1u) {
        let c = ball_center(i);
        let d = coord - c;
        let dist2 = max(dot(d, d), EPS);

        let r = balls[i].radius;
        let r2 = r * r;

        let inv_dist2 = 1.0 / dist2;
        let contrib = r2 * inv_dist2;
        let bc = balls[i].color.rgb;
        color_acc = color_acc + bc * contrib;
      }
      blended_color = vec4<f32>(color_acc / field, 1.0);
    }
  }

  // Prepare packed gradient
  let grad_len = length(grad);

  // Reciprocal gradient length for signed distance; clamp to avoid huge values in flat zones.
  var inv_grad_len = 0.0;
  if (grad_len > 1e-6) {
    inv_grad_len = min(1.0 / grad_len, 2048.0); // clamp for fp16 safety
  }

  // Normalized gradient (WGSL: use select instead of ternary)
  let norm_grad = select(
    vec2<f32>(0.0, 0.0),
    grad * (1.0 / grad_len),
    grad_len > 1e-6
  );

  textureStore(
    output_tex,
    vec2<i32>(i32(gid.x), i32(gid.y)),
    vec4<f32>(field, norm_grad.x, norm_grad.y, inv_grad_len)
  );

  // Write albedo as pre-multiplied by coverage (we'll use smoothstep in present)
  // Compute simple coverage based on iso and a small smoothstep width
  let w = 0.5; // small smoothing factor to avoid hard cutoff in albedo
  let coverage = smoothstep(params.iso - w, params.iso + w, field);
  let out_albedo_color = vec4<f32>(blended_color.rgb * coverage, coverage);
  textureStore(out_albedo, vec2<i32>(i32(gid.x), i32(gid.y)), out_albedo_color);
}