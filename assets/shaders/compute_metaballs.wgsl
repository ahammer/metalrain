// Spatially-accelerated metaball rendering
// Uses a uniform grid to reduce per-pixel ball checks from O(N) to O(k)
// where k is the local ball density.
//
// Packed output layout (rgba16float):
//   R: field value (Σ r_i^2 / d_i^2)
//   G: normalized gradient x (unit, 0 if length ~ 0)
//   B: normalized gradient y
//   A: inverse gradient length (1/|∇|), clamped; 0 if |∇| tiny.

struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,
  clustering_enabled: u32,
  grid_dimensions: vec2<u32>,
  active_ball_count: u32,
  _pad: u32,
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

struct GridCell {
  offset: u32,
  count: u32,
}

@group(0) @binding(0) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<uniform> time_u: TimeU;
@group(0) @binding(3) var<storage, read> balls: array<Ball>;
@group(0) @binding(4) var out_albedo: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var<storage, read> grid_cells: array<GridCell>;
@group(0) @binding(6) var<storage, read> ball_indices: array<u32>;

const EPS: f32 = 1e-4;
const GRID_CELL_SIZE: f32 = 64.0;

const WORLD_MIN: vec2<f32> = vec2<f32>(-256.0, -256.0);
const WORLD_MAX: vec2<f32> = vec2<f32>( 256.0,  256.0);
const WORLD_SIZE: vec2<f32> = WORLD_MAX - WORLD_MIN;

fn to_world(pixel: vec2<f32>) -> vec2<f32> {
  return pixel;
}

fn ball_center(i: u32) -> vec2<f32> {
  let b = balls[i];
  return to_world(b.center);
}

/// Calculate which grid cell a pixel belongs to
fn get_grid_cell(coord: vec2<f32>) -> vec2<u32> {
  let cell_x = u32(coord.x / GRID_CELL_SIZE);
  let cell_y = u32(coord.y / GRID_CELL_SIZE);
  return vec2<u32>(
    min(cell_x, params.grid_dimensions.x - 1u),
    min(cell_y, params.grid_dimensions.y - 1u)
  );
}

/// Get the flattened cell index
fn get_cell_id(cell: vec2<u32>) -> u32 {
  return cell.y * params.grid_dimensions.x + cell.x;
}

@compute @workgroup_size(8, 8, 1)
fn metaballs(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) { return; }

  // Sample point in world space
  let coord = to_world(vec2<f32>(f32(gid.x), f32(gid.y)));

  // Determine which grid cell this pixel belongs to
  let cell = get_grid_cell(coord);
  let cell_id = get_cell_id(cell);

  // Bounds check
  let total_cells = params.grid_dimensions.x * params.grid_dimensions.y;
  if (cell_id >= total_cells || cell_id >= arrayLength(&grid_cells)) {
    // Out of bounds, clear output
    textureStore(output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(0.0, 0.0, 0.0, 0.0));
    textureStore(out_albedo, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(0.0, 0.0, 0.0, 0.0));
    return;
  }

  // Get the balls in this cell
  let cell_data = grid_cells[cell_id];
  let ball_offset = cell_data.offset;
  let ball_count = cell_data.count;

  var field: f32 = 0.0;
  var grad: vec2<f32> = vec2<f32>(0.0, 0.0);
  var blended_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

  // First pass: compute contribs and track dominant cluster
  var max_contrib: f32 = 0.0;
  var dominant_cluster: i32 = 0;

  // Iterate only over balls in this cell (spatial acceleration!)
  for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
    let ball_idx_pos = ball_offset + i;
    if (ball_idx_pos >= arrayLength(&ball_indices)) {
      break;
    }

    let ball_idx = ball_indices[ball_idx_pos];
    if (ball_idx >= arrayLength(&balls)) {
      continue;
    }

    let c = ball_center(ball_idx);
    let d = coord - c;
    let dist2 = max(dot(d, d), EPS);

    let r = balls[ball_idx].radius;
    let r2 = r * r;

    let inv_dist2 = 1.0 / dist2;
    let contrib = r2 * inv_dist2;
    field = field + contrib;

    let inv_dist4 = inv_dist2 * inv_dist2;
    let scale = -2.0 * r2 * inv_dist4;
    grad = grad + scale * d;

    if (contrib > max_contrib) {
      max_contrib = contrib;
      dominant_cluster = balls[ball_idx].cluster_id;
    }
  }

  // If clustering is enabled, recompute field & gradient for dominant cluster only
  if (params.clustering_enabled > 0u) {
    field = 0.0;
    grad = vec2<f32>(0.0, 0.0);
    var cluster_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
      let ball_idx_pos = ball_offset + i;
      if (ball_idx_pos >= arrayLength(&ball_indices)) {
        break;
      }

      let ball_idx = ball_indices[ball_idx_pos];
      if (ball_idx >= arrayLength(&balls)) {
        continue;
      }

      if (balls[ball_idx].cluster_id == dominant_cluster) {
        let c = ball_center(ball_idx);
        let d = (coord - c);
        let dist2 = max(dot(d, d), EPS);

        let r = balls[ball_idx].radius * 1.3;
        let r2 = r * r;

        let inv_dist2 = 1.0 / dist2;
        let contrib = r2 * inv_dist2;
        field = field + contrib;

        let inv_dist4 = inv_dist2 * inv_dist2;
        let scale = -2.0 * r2 * inv_dist4;
        grad = grad + scale * d;

        cluster_color = balls[ball_idx].color;
      }
    }

    blended_color = cluster_color;
  } else {
    // clustering disabled: blend colors by influence
    if (field > 0.0) {
      var color_acc: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
      for (var i: u32 = 0u; i < ball_count; i = i + 1u) {
        let ball_idx_pos = ball_offset + i;
        if (ball_idx_pos >= arrayLength(&ball_indices)) {
          break;
        }

        let ball_idx = ball_indices[ball_idx_pos];
        if (ball_idx >= arrayLength(&balls)) {
          continue;
        }

        let c = ball_center(ball_idx);
        let d = (coord - c);
        let dist2 = max(dot(d, d), EPS);

        let r = balls[ball_idx].radius;
        let r2 = r * r;

        let inv_dist2 = 1.0 / dist2;
        let contrib = r2 * inv_dist2;
        let bc = balls[ball_idx].color.rgb;
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
    vec4<f32>(field * 0.5, norm_grad.x, norm_grad.y, inv_grad_len)
  );

  // Write albedo as premultiplied by a simple field-derived coverage so the present shader
  // can recover the base color. Use a clamp on field to produce a stable alpha across
  // typical field magnitudes.
  let coverage = clamp(field * 0.5, 0.0, 1.0);
  let out_albedo_color = vec4<f32>(blended_color.rgb * coverage, coverage);
  textureStore(out_albedo, vec2<i32>(i32(gid.x), i32(gid.y)), out_albedo_color);
}
