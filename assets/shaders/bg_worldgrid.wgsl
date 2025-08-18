// World-grid background shader.
// Drawn via a full-screen quad without clearing; acts as implicit background.
// Exposes configurable cell size in world units.

struct BgUniform {
    window_size: vec2<f32>,
    cell_size: f32,
    line_thickness: f32,
    dark_factor: f32,
    _pad: f32,
    // Future: colors etc.
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
    let half_size = bg.window_size * 0.5;
    out.world_pos = position.xy * half_size;
    return out;
}

// Simple checker + grid lines:
// - Alternating light/dark squares based on floor(world_pos / cell_size)
// - Grid lines along cell boundaries using distance to nearest integer coordinate.
@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    let cell = bg.cell_size;
    let p = in.world_pos / cell;
    let ip = floor(p);
    let checker = f32(u32(ip.x + ip.y) & 1u);
    // Brighter range for visibility
    let base_light = mix(0.30, 0.42, checker);
    let f = fract(p);
    let dist_x = min(f.x, 1.0 - f.x);
    let dist_y = min(f.y, 1.0 - f.y);
    let line_x = step(dist_x, bg.line_thickness);
    let line_y = step(dist_y, bg.line_thickness);
    let line_mask = clamp(line_x + line_y, 0.0, 1.0);
    // Lines tinted slightly bluish
    let line_color = vec3<f32>(0.55, 0.60, 0.75);
    let base = vec3<f32>(base_light);
    let color = mix(base, line_color, line_mask * 0.85);
    return vec4<f32>(color, 1.0); // fully opaque background
}
