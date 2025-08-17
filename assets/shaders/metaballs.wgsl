// Per-ball metaballs using a Wyvill-style bounded kernel f = (1 - (d/R)^2)^3 for d<R.
// Accumulates field & gradient for analytic normal, applies smooth AA around iso threshold.
// Single full-screen pass (vertex passthrough) using Material2d bind group (index 2 in Bevy 0.14).

const MAX_BALLS : u32 = 1024u;
const MAX_CLUSTERS : u32 = 256u;

struct MetaballsData {
    ball_count: u32,
    cluster_color_count: u32,
    radius_scale: f32,
    _pad1: u32,
    window_size: vec2<f32>,
    iso: f32,
    normal_z_scale: f32,
    // New shading params (packed to 16B)
    metallic: f32,          // 0 = dielectric, 1 = full metal (affects F0 and specular coloration)
    roughness: f32,         // perceptual roughness in [0,1]
    env_intensity: f32,     // environment reflection intensity
    spec_intensity: f32,    // direct specular multiplier
    debug_view: u32,        // 0=Normal shaded,1=Heightfield,2=ColorInfo
    _pad_dbg: vec3<f32>,
    // (header now 64 bytes; arrays follow aligned to 16)
    balls: array<vec4<f32>, MAX_BALLS>,          // (x, y, radius, cluster_index as float)
    cluster_colors: array<vec4<f32>, MAX_CLUSTERS>, // (r,g,b,_)
};

// Material2d bind group index is 2 (0=view,1=mesh,2=material) in Bevy 0.14; use group(2).
@group(2) @binding(0)
var<uniform> metaballs: MetaballsData;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    let half_size = metaballs.window_size * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (metaballs.ball_count == 0u) { return vec4<f32>(0.0); }
    let p = in.world_pos;
    var field: f32 = 0.0;
    var grad: vec2<f32> = vec2<f32>(0.0);
    // For hard boundaries choose nearest contributing ball (distance based) -> material color.
    var nearest_d2: f32 = 1e30;
    var nearest_cluster: u32 = 0u;
    var nearest_local_r2: f32 = 1.0; // for spherical normal correction
    var nearest_center: vec2<f32> = vec2<f32>(0.0);
    // Accumulate field & gradient
    for (var i: u32 = 0u; i < metaballs.ball_count; i = i + 1u) {
        let b = metaballs.balls[i];
        let center = b.xy;
        let radius = b.z;
        if (radius <= 0.0) { continue; }
        let d = p - center;
        let d2 = dot(d, d);
        let scaled_r = radius * metaballs.radius_scale;
        let r2 = scaled_r * scaled_r;
        if (d2 < r2) {
            let x = 1.0 - d2 / r2; // in [0,1]
            let x2 = x * x;
            let fi = x2 * x; // contribution
            field = field + fi;
            grad = grad + (-6.0 / r2) * d * x2;
            if (d2 < nearest_d2) {
                nearest_d2 = d2;
                nearest_cluster = u32(b.w + 0.5);
                nearest_local_r2 = r2;
                nearest_center = center;
            }
        }
    }
    if (field <= 0.0001) { return vec4<f32>(0.0); }
    // Signed distance approximation (field - iso) / |grad|
    let grad_len = max(length(grad), 1e-5);
    let s = (field - metaballs.iso) / grad_len;
    // Smooth AA mask around iso (kept for edges) but interior gets hard color separation.
    let px = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
    let aa = 1.5 * px;
    let mask = clamp(0.5 + 0.5 * s / aa, 0.0, 1.0);

    // Material base color from nearest contributing cluster.
    var base_col: vec3<f32> = vec3<f32>(0.8,0.8,0.8);
    if (nearest_cluster < metaballs.cluster_color_count) {
        base_col = metaballs.cluster_colors[nearest_cluster].rgb;
    }

    // Reconstruct a more spherical-ish normal: combine field gradient with a sphere normal of nearest ball.
    let to_center = p - nearest_center;
    let radial = normalize(vec3<f32>(to_center, 0.0));
    // Sphere Z from radius^2 - d^2 (hemisphere) -> approximate depth and normal.
    let sphere_z = sqrt(max(nearest_local_r2 - nearest_d2, 0.0));
    let sphere_normal = normalize(vec3<f32>(to_center, sphere_z * metaballs.normal_z_scale));
    let field_normal = normalize(vec3<f32>(grad, metaballs.normal_z_scale));
    let n = normalize(mix(field_normal, sphere_normal, 0.6));

    // Branch early for debug view variants that bypass full shading.
    if (metaballs.debug_view == 1u) { // Heightfield: visualize raw field value pre-iso with edge mask
        let grad_len = max(length(grad), 1e-5);
        let s = (field - metaballs.iso) / grad_len;
        let px = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
        let aa = 1.5 * px;
        let mask = clamp(0.5 + 0.5 * s / aa, 0.0, 1.0);
        let gray = clamp(field, 0.0, 4.0) / 4.0; // normalized approx
        return vec4<f32>(vec3<f32>(gray), mask);
    }
    if (metaballs.debug_view == 2u) { // ColorInfo: show cluster color table directly, no lighting
        let grad_len = max(length(grad), 1e-5);
        let s = (field - metaballs.iso) / grad_len;
        let px = length(vec2<f32>(dpdx(in.world_pos.x), dpdy(in.world_pos.y)));
        let aa = 1.5 * px;
        let mask = clamp(0.5 + 0.5 * s / aa, 0.0, 1.0);
        var base_col: vec3<f32> = vec3<f32>(0.5,0.5,0.5);
        if (nearest_cluster < metaballs.cluster_color_count) {
            base_col = metaballs.cluster_colors[nearest_cluster].rgb;
        }
        return vec4<f32>(base_col, mask);
    }

    // Lighting: single directional + environment reflection approximation (Normal mode only).
    let L = normalize(vec3<f32>(0.6, 0.5, 1.0));
    let V = normalize(vec3<f32>(0.0, 0.0, 1.0));
    let H = normalize(L + V);
    let ndotl = max(dot(n, L), 0.0);
    let ndotv = max(dot(n, V), 0.0);
    let ndoth = max(dot(n, H), 0.0);
    let rough = clamp(metaballs.roughness, 0.04, 1.0);
    let alpha = rough * rough; // GGX alpha
    // GGX NDF
    let a2 = alpha * alpha;
    let denom = (ndoth * ndoth) * (a2 - 1.0) + 1.0;
    let D = a2 / (3.14159 * denom * denom);
    // Smith G (Schlick-GGX)
    let k = (alpha + 1.0);
    let k2 = (k * k) / 8.0;
    let Gv = ndotv / (ndotv * (1.0 - k2) + k2);
    let Gl = ndotl / (ndotl * (1.0 - k2) + k2);
    let G = Gv * Gl;
    // Fresnel Schlick
    let F0_dielectric = vec3<f32>(0.04, 0.04, 0.04);
    let F0 = mix(F0_dielectric, base_col, metaballs.metallic);
    let F = F0 + (1.0 - F0) * pow(1.0 - ndotv, 5.0);
    let spec = (D * G * F) / max(4.0 * ndotv * ndotl + 1e-5, 1e-5);
    // Diffuse term suppressed by metallic
    let diffuse = base_col * (1.0 - metaballs.metallic) * ndotl;
    // Simple environment reflection: use n.z & a horizon tint.
    let env_up = vec3<f32>(0.85, 0.90, 1.0);
    let env_down = vec3<f32>(0.05, 0.06, 0.07);
    let env = mix(env_down, env_up, 0.5 + 0.5 * n.z) * metaballs.env_intensity;
    let color = diffuse + spec * metaballs.spec_intensity + env * F;
    // Tone map (simple Reinhard) and gamma-ish correction.
    let mapped = color / (color + 1.0);
    let final_rgb = pow(mapped, vec3<f32>(0.4545));
    return vec4<f32>(final_rgb, mask);
}
