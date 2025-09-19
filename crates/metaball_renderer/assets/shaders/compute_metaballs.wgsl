// Copied from POC (compute_balls) – kept identical for Phase 2 extraction.
struct Params {
	screen_size: vec2<f32>,
	num_balls: u32,
	_unused0: u32,
	iso: f32,
	_unused2: f32,
	_unused3: f32,
	_unused4: u32,
	clustering_enabled: u32,
	_pad: f32,
}
struct TimeU { time: f32, }
struct Ball { center: vec2<f32>, radius: f32, cluster_id: i32, color: vec4<f32>, }
@group(0) @binding(0) var output_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var<uniform> time_u: TimeU;
@group(0) @binding(3) var<storage, read> balls: array<Ball>;
@group(0) @binding(4) var out_albedo: texture_storage_2d<rgba8unorm, write>;
const EPS: f32 = 1e-4;
const WORLD_MIN: vec2<f32> = vec2<f32>(-1000.0, -1000.0);
const WORLD_MAX: vec2<f32> = vec2<f32>( 1000.0,  1000.0);
const WORLD_SIZE: vec2<f32> = WORLD_MAX - WORLD_MIN; 
fn to_world(pixel: vec2<f32>) -> vec2<f32> { let uv = (pixel + vec2<f32>(0.5, 0.5)) / params.screen_size; return WORLD_MIN + uv * WORLD_SIZE; }
fn ball_center(i: u32) -> vec2<f32> { let b = balls[i]; let phase = f32(i) * 0.37; let wobble = vec2<f32>( sin(time_u.time * 0.6 + phase) * 12.0, cos(time_u.time * 0.8 + phase * 1.7) * 9.0 ); let pixel_pos = b.center + wobble; return to_world(pixel_pos); }
@compute @workgroup_size(8, 8, 1)
fn metaballs(@builtin(global_invocation_id) gid: vec3<u32>) {
	if (gid.x >= u32(params.screen_size.x) || gid.y >= u32(params.screen_size.y)) { return; }
	let coord = to_world(vec2<f32>(f32(gid.x), f32(gid.y)));
	var field: f32 = 0.0; var grad: vec2<f32> = vec2<f32>(0.0, 0.0); var blended_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0); let count = min(params.num_balls, arrayLength(&balls));
	var max_contrib: f32 = 0.0; var dominant_cluster: i32 = 0;
	for (var i: u32 = 0u; i < count; i = i + 1u) { let c = ball_center(i); let d = coord - c; let dist2 = max(dot(d, d), EPS); let r = balls[i].radius; let r2 = r * r; let inv_dist2 = 1.0 / dist2; let contrib = r2 * inv_dist2; field = field + contrib; let inv_dist4 = inv_dist2 * inv_dist2; let scale = -2.0 * r2 * inv_dist4; grad = grad + scale * d; if (contrib > max_contrib) { max_contrib = contrib; dominant_cluster = balls[i].cluster_id; } }
	if (params.clustering_enabled > 0u) { field = 0.0; grad = vec2<f32>(0.0, 0.0); var cluster_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0); for (var i: u32 = 0u; i < count; i = i + 1u) { if (balls[i].cluster_id == dominant_cluster) { let c = ball_center(i); let d = coord - c; let dist2 = max(dot(d, d), EPS); let r = balls[i].radius; let r2 = r * r; let inv_dist2 = 1.0 / dist2; let contrib = r2 * inv_dist2; field = field + contrib; let inv_dist4 = inv_dist2 * inv_dist2; let scale = -2.0 * r2 * inv_dist4; grad = grad + scale * d; cluster_color = balls[i].color; } } blended_color = cluster_color; } else { if (field > 0.0) { var color_acc: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0); for (var i: u32 = 0u; i < count; i = i + 1u) { let c = ball_center(i); let d = coord - c; let dist2 = max(dot(d, d), EPS); let r = balls[i].radius; let r2 = r * r; let inv_dist2 = 1.0 / dist2; let contrib = r2 * inv_dist2; let bc = balls[i].color.rgb; color_acc = color_acc + bc * contrib; } blended_color = vec4<f32>(color_acc / field, 1.0); } }
	let grad_len = length(grad); var inv_grad_len = 0.0; if (grad_len > 1e-6) { inv_grad_len = min(1.0 / grad_len, 2048.0); } let norm_grad = select( vec2<f32>(0.0, 0.0), grad * (1.0 / grad_len), grad_len > 1e-6 ); textureStore( output_tex, vec2<i32>(i32(gid.x), i32(gid.y)), vec4<f32>(field, norm_grad.x, norm_grad.y, inv_grad_len) ); let coverage = clamp(field * 0.5, 0.0, 1.0); let out_albedo_color = vec4<f32>(blended_color.rgb * coverage, coverage); textureStore(out_albedo, vec2<i32>(i32(gid.x), i32(gid.y)), out_albedo_color); }
