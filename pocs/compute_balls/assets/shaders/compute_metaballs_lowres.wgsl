struct Params {
  iso: f32,
  edge_band: f32,
  normal_z: f32,
  height_scale: f32,
  height_sharp: f32,
  ambient: f32,
  spec_pow: f32,
  spec_int: f32,
  rim_pow: f32,
  rim_int: f32,
  fresnel_int: f32,
  outline_w: f32,
  outline_int: f32,
  glow_w: f32,
  glow_int: f32,
  light_dir: vec3<f32>, _pad0: f32,
  low_res_inv: vec2<f32>,
  full_res_inv: vec2<f32>,
  scale_factor: f32, shadow_steps: u32, shadow_step_scale: f32, shadow_int: f32,
  refine_threshold: f32, enable_refine: u32, mode: u32, cluster_count: u32,
}

struct TimeU { time: f32, }

struct Ball { center: vec2<f32>, radius: f32, cluster_id: u32, color_index: u32, _pad: u32 }

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var index_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<uniform> params: Params;
@group(0) @binding(3) var<uniform> time_u: TimeU;
@group(0) @binding(4) var<storage, read> balls: array<Ball>;

const EPS: f32 = 1e-6;

fn kernel(r: f32, d2: f32) -> f32 { let rr = r * r; return rr / (d2 + 1.0); }
fn kernel_grad(r: f32, dp: vec2<f32>, d2: f32) -> vec2<f32> {
  let rr = r * r;
  let denom = (d2 + 1.0) * (d2 + 1.0);
  return -2.0 * rr * dp / denom;
}

@compute @workgroup_size(8,8,1)
fn metaballs_lowres(@builtin(global_invocation_id) gid: vec3<u32>) {
  let dims = vec2<u32>(u32(1.0/params.low_res_inv.x), u32(1.0/params.low_res_inv.y));
  if (gid.x >= dims.x || gid.y >= dims.y) { return; }
  let uv = (vec2<f32>(f32(gid.x), f32(gid.y)) + 0.5) * params.low_res_inv;

  var F: f32 = 0.0;
  var G: vec2<f32> = vec2<f32>(0.0, 0.0);
  var best_val: f32 = 0.0;
  var second_val: f32 = 0.0;
  var best_ball: u32 = 0xFFFFFFFFu;
  var best_cluster: u32 = 0xFFFFFFFFu;
  var second_cluster: u32 = 0xFFFFFFFFu;

  let count = arrayLength(&balls);
  for (var i: u32 = 0u; i < count; i = i + 1u) {
    let b = balls[i];
    let dp = uv - b.center;
    let d2 = dot(dp, dp) + EPS;
    let val = kernel(b.radius, d2);
    F = F + val;
    G = G + kernel_grad(b.radius, dp, d2);
    if (val > best_val) {
      second_val = best_val;
      second_cluster = best_cluster;
      best_val = val;
      best_ball = i;
      best_cluster = b.cluster_id;
    } else if (val > second_val) {
      second_val = val;
      second_cluster = b.cluster_id;
    }
  }

  // Shadow (simple multi-step using current gradient) if enabled
  var shadow_fac: f32 = 1.0;
  if (params.mode >= 1u) {
    let g_len = max(length(G), EPS);
    var occ: f32 = 0.0;
    var p = uv;
    let dir2d = normalize(params.light_dir.xy);
    for (var s: u32 = 0u; s < params.shadow_steps; s = s + 1u) {
      let sd = (F - params.iso) / g_len;
      let step_len = clamp(sd * params.shadow_step_scale, 0.002, 0.05);
      p = p + dir2d * step_len;
      if (p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0) { break; }
      // Re-evaluate field cheaply (naive re-loop) -- optimize later
      var Fp: f32 = 0.0;
      for (var j: u32 = 0u; j < count; j = j + 1u) {
        let bj = balls[j];
        let dpp = p - bj.center;
        let d2p = dot(dpp, dpp) + EPS;
        Fp = Fp + kernel(bj.radius, d2p);
      }
      let sdp = (Fp - params.iso) / g_len;
      if (sdp > 0.0) { occ = occ + 0.2; }
      if (occ >= 0.95) { break; }
    }
    shadow_fac = 1.0 - clamp(occ, 0.0, 1.0);
  }

  let dominance = best_val / max(best_val + second_val, EPS);
  textureStore(field_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(F, G.x, G.y, shadow_fac));
  textureStore(index_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(f32(best_ball), f32(best_cluster), f32(second_cluster), dominance));
}
