// World-grid background shader.
// Drawn via a full-screen quad without clearing; acts as implicit background.
// Exposes configurable cell size in world units.

struct BgUniform {
    v0: vec4<f32>; // (window_size.x, window_size.y, cell_size, line_thickness)
    v1: vec4<f32>; // (dark_factor, reserved1, reserved2, reserved3)
};

@group(2) @binding(0)
var<uniform> bg: BgUniform;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
};

@vertex
fn vertex(@location(0) position: vec3<f32>) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // Map NDC [-1,1] to world coords using window size (camera assumed scale=1 mapping px to world units at 1:1 for now)
    let window_size = bg.v0.xy;
    let half_size = window_size * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

// Simple checker + grid lines:
// - Alternating light/dark squares based on floor(world_pos / cell_size)
// - Grid lines along cell boundaries using distance to nearest integer coordinate.
@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let cell = bg.v0.z;
    let p = in.world_pos / cell;
    let ip = floor(p);
    let checker = f32(u32(ip.x + ip.y) & 1u);
    // Brighter range for visibility
    let base_light = mix(0.30, 0.42, checker);
    let f = fract(p);
    let dist_x = min(f.x, 1.0 - f.x);
    let dist_y = min(f.y, 1.0 - f.y);
    let line_x = step(dist_x, bg.v0.w);
    let line_y = step(dist_y, bg.v0.w);
    let line_mask = clamp(line_x + line_y, 0.0, 1.0);
    // Lines tinted slightly bluish
    let line_color = vec3<f32>(0.55, 0.60, 0.75);
    let base = vec3<f32>(base_light);
    let color = mix(base, line_color, line_mask * 0.85);
    let dark_factor = bg.v1.x;
    return vec4<f32>(color * (1.0 - dark_factor) + dark_factor * vec3<f32>(0.0,0.0,0.0), 1.0);
}
