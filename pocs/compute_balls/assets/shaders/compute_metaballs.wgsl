// Compute metaballs distance field + simple surface shading.
// One dispatch per frame writes RGBA color into a storage texture.

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,
  debug_mode: u32, // 0 shaded,1 field,2 normals,3 iso bands,4 gradient dir
  iso: f32,
  ambient: f32,
  rim_power: f32,
  show_centers: u32,
}
struct TimeU {
  time: f32,
}

struct Ball {
  center: vec2<f32>,
  radius: f32,
  _pad: f32,
}

@group(0) @binding(0) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<uniform> time_u: TimeU;
@group(0) @binding(3) var<storage, read> balls: array<Ball>;

const EPS: f32 = 1e-4;

fn ball_center(i: u32) -> vec2<f32> {
  // Optional subtle orbit using global time (procedural motion)
  let b = balls[i];
  let phase = f32(i) * 0.37;
  let wobble = vec2<f32>(sin(time_u.time * 0.6 + phase) * 12.0, cos(time_u.time * 0.8 + phase * 1.7) * 9.0);
  return b.center + wobble;
}

@compute @workgroup_size(8, 8, 1)
fn metaballs(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) { return; }
  let coord = vec2<f32>(f32(gid.x), f32(gid.y));

  var field: f32 = 0.0;
  var grad: vec2<f32> = vec2<f32>(0.0, 0.0);

  let count = min(params.num_balls, arrayLength(&balls));

  for (var i: u32 = 0u; i < count; i = i + 1u) {
    let c = ball_center(i);
    let d = coord - c;
    let dist2 = max(dot(d, d), EPS);
    // Classic metaball contribution: r^2 / dist^2
    let r = balls[i].radius;
    let r2 = r * r;
    let inv = 1.0 / dist2;
    let contrib = r2 * inv;
    field = field + contrib;

    // Gradient of contrib wrt position: -2 * r^2 * (coord - c) / dist^4
    let inv2 = inv * inv;
    grad = grad - 2.0 * r2 * inv2 * d;
  }

  let inside = field >= params.iso;

  // Normal / gradient derived data
  let n_len = max(length(grad), EPS);
  let n2d = grad / n_len;
  let n = vec3<f32>(n2d, 0.0);

  var color: vec3<f32> = vec3<f32>(0.0);

  // Base background: dark vertical gradient with faint grid lines
  let uv = coord / params.screen_size;
  let base_bg = mix(vec3<f32>(0.07,0.08,0.10), vec3<f32>(0.10,0.11,0.13), uv.y);
  // Grid (every 32 px) using select (WGSL has no ternary ? : )
  let gx = fract(coord.x / 32.0) < 0.02;
  let gy = fract(coord.y / 32.0) < 0.02;
  let g = select(0.0, 0.25, gx || gy);
  let background = base_bg + g;

  switch (params.debug_mode) {
    case 0u: { // Shaded metaballs
      if (inside) {
        let l = normalize(vec3<f32>(0.5, 0.8, 0.6));
        let diff = max(dot(n, l), 0.0);
        let rim = pow(1.0 - max(n.z, 0.0), params.rim_power);
        let h = fract(field * 0.13);
        let base = vec3<f32>(
          0.6 + 0.4 * sin(6.2831 * h),
          0.55 + 0.45 * sin(6.2831 * (h + 0.33)),
          0.5 + 0.5 * sin(6.2831 * (h + 0.66))
        );
        color = base * (params.ambient + diff * 0.9) + rim * 0.3;
      } else {
        color = background;
      }
    }
    case 1u: { // Field grayscale (normalized by iso)
      let f = clamp(field / (params.iso * 1.5), 0.0, 1.0);
      color = mix(background, vec3<f32>(f,f,f), 0.85);
    }
    case 2u: { // Normal visualization
      color = n * 0.5 + vec3<f32>(0.5,0.5,0.5);
    }
    case 3u: { // Iso bands
      let band = fract(field * 0.25);
      let mask = select(0.3, 1.0, inside);
      color = mix(background, vec3<f32>(band, 1.0 - band, 0.5 + 0.5*band) * mask, 0.9);
    }
    case 4u: { // Gradient direction (angle -> color)
      let ang = atan2(n2d.y, n2d.x);
      let u = (ang / 6.2831) + 0.5;
      color = vec3<f32>(u, abs(n2d.x), abs(n2d.y));
    }
    default: {
      color = background;
    }
  }

  // Overlay centers if enabled
  if (params.show_centers == 1u) {
    let count = min(params.num_balls, arrayLength(&balls));
    for (var i: u32 = 0u; i < count; i = i + 1u) {
      let c = ball_center(i);
      if (abs(coord.x - c.x) < 2.0 && abs(coord.y - c.y) < 2.0) {
        color = vec3<f32>(1.0,0.2,0.1);
      }
    }
  }

  color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(color, 1.0));
}
