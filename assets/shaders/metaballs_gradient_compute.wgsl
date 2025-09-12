// ============================================================================
// Metaballs Gradient + Field Half-Res Compute (Phase 1)
// Outputs RGBA16F: (field, dFdx, dFdy, cluster_or_zero)
// Cluster accumulation deferred -> A channel = 0.0
// ============================================================================

struct MetaballsData {
  v0: vec4<f32>,
  v1: vec4<f32>,
  v2: vec4<f32>,
  v3: vec4<f32>,
  v4: vec4<f32>,
  v5: vec4<f32>,
  v6: vec4<f32>,
  v7: vec4<f32>,
};

struct GpuBall { data0: vec4<f32>, data1: vec4<f32> }; // data0:(x,y,radius,packed_gid)
struct TileHeader { offset: u32, count: u32, _pad0: u32, _pad1: u32 };
struct ClusterColor { value: vec4<f32> }; // unused phase 1

@group(0) @binding(0) var<uniform> metaballs: MetaballsData;
@group(0) @binding(1) var<storage, read> balls: array<GpuBall>;
@group(0) @binding(2) var<storage, read> tile_headers: array<TileHeader>;
@group(0) @binding(3) var<storage, read> tile_ball_indices: array<u32>;
@group(0) @binding(4) var<storage, read> cluster_palette: array<ClusterColor>; // reserved
@group(0) @binding(5) var gradient_out: texture_storage_2d<rgba16float, write>;

const EPS: f32 = 1e-5;

fn field_contrib(p: vec2<f32>, center: vec2<f32>, r: f32) -> f32 {
  if (r <= 0.0) { return 0.0; }
  let d = p - center;
  let d2 = dot(d, d);
  let r2 = r * r;
  if (d2 >= r2) { return 0.0; }
  let x = 1.0 - d2 / r2;
  return x * x * x;
}

@compute @workgroup_size(8,8,1)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = textureDimensions(gradient_out);
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let vp = metaballs.v2.xy; // (full_w, full_h)
  let pixel = vec2<f32>(f32(gid.x), f32(gid.y));
  var full_center = pixel * 2.0 + vec2<f32>(1.0, 1.0);
  full_center.x = min(full_center.x, vp.x - 0.5);
  full_center.y = min(full_center.y, vp.y - 0.5);
  let world_pos = full_center - vp * 0.5;

  let tiles_x = u32(metaballs.v3.x + 0.5);
  let tiles_y = u32(metaballs.v3.y + 0.5);
  let tile_size = metaballs.v3.z;
  let ball_count_exposed = u32(metaballs.v0.x + 0.5);
  let balls_len_actual = u32(metaballs.v3.w + 0.5);
  let ball_count = min(ball_count_exposed, balls_len_actual);
  if (ball_count == 0u) {
    textureStore(gradient_out, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(0.0,0.0,0.0,0.0));
    return;
  }
  let iso = max(metaballs.v0.w, EPS);
  let radius_coeff = metaballs.v0.z * metaballs.v2.w; // radius_scale * radius_multiplier

  // Compute tile index
  let origin = -vp * 0.5;
  let local = world_pos - origin;
  let tx = clamp(u32(floor(local.x / tile_size)), 0u, tiles_x - 1u);
  let ty = clamp(u32(floor(local.y / tile_size)), 0u, tiles_y - 1u);
  let tile_index = ty * tiles_x + tx;
  let header = tile_headers[tile_index];
  if (header.count == 0u) {
    textureStore(gradient_out, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(0.0,0.0,0.0,0.0));
    return;
  }
  var field: f32 = 0.0;
  var grad: vec2<f32> = vec2<f32>(0.0, 0.0);
  // Iterate only balls in this tile
  for (var i: u32 = 0u; i < header.count; i = i + 1u) {
    let bi = tile_ball_indices[header.offset + i];
    if (bi >= ball_count) { continue; }
    let b0 = balls[bi].data0;
    let ctr = b0.xy;
    let r = b0.z * radius_coeff;
    if (r <= 0.0) { continue; }
    let d = world_pos - ctr;
    let d2 = dot(d,d);
    let r2 = r * r;
    if (d2 >= r2) { continue; }
    let x = 1.0 - d2 / r2;
    let contrib = x * x * x;
    if (contrib <= 0.0) { continue; }
    field = field + contrib;
    // Analytic gradient: dF/dp = -6 x^2 (p-c)/r^2
    let gscale = -6.0 * x * x / r2;
    grad = grad + gscale * d;
  }
  textureStore(gradient_out, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(field, grad.x, grad.y, 0.0));
}
