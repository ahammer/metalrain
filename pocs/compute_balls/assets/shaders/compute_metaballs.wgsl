struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,
  _unused0: u32,
  _unused1: f32,
  _unused2: f32,
  _unused3: f32,
  _unused4: u32,
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

  }
  // Optionally clamp field for display range (keep it monotonic).
  let f = field; // Leave unclamped; consumer can normalize.
  textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(f, 0.0, 0.0, 1.0));
}
