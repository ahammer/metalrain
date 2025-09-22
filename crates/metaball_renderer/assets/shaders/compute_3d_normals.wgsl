struct Params {
  screen_size: vec2<f32>,
  num_balls: u32,
  clustering_enabled: u32,
};

@group(0) @binding(0) var field_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var normals_tex: texture_storage_2d<rgba16float, write>;

// Values above it are "inside" the surface.
const ISO: f32 = 1.0;
const FIELD_MAX: f32 = 1.2;

@compute @workgroup_size(8, 8, 1)
fn compute_normals(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) {
    return;
  }

  let coord = vec2<i32>(i32(gid.x), i32(gid.y));
  let packed = textureLoad(field_tex, coord);

  let field = packed.r;
  let grad_dir = vec2<f32>(packed.g, packed.b);
  let inv_grad_len = packed.a;

  if (field <= ISO || inv_grad_len <= 0.0) {
    textureStore(normals_tex, coord, vec4<f32>(0.0, 0.0, 1.0, 0.0));
    return;
  }

  let range = max(FIELD_MAX - ISO, 1e-6);
  let height = 1.0 - clamp((field - ISO) / range, 0.0, 1.0);

  let h_smooth = smoothstep(0.0, 1.0, height);
  let slope_magnitude = sqrt(h_smooth * (2.0 - h_smooth)); // Maps [0,1] to a circular profile
  let slope = -grad_dir * slope_magnitude;

  let normal_z = sqrt(max(1.0 - dot(slope, slope), 1e-6));

  let n = vec3<f32>(slope.x, slope.y, normal_z);

  textureStore(normals_tex, coord, vec4<f32>(n, height));
}
