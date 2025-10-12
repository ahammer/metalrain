#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Background material uniform layout auto-generated from AsBindGroup
struct BackgroundMaterialUniform {
    mode: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
    primary_color: vec4<f32>,
    secondary_color: vec4<f32>,
    params: vec4<f32>,       // angle, time, anim_speed, radial_radius
    radial_center: vec2<f32>,
    _pad4: vec2<f32>,
}

@group(2) @binding(0) var<uniform> material: BackgroundMaterialUniform;

fn rotate(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let m = mat2x2<f32>(c, -s, s, c);
    return m * uv;
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    // mesh2d UV is already (0,1) range; we flip Y for consistency with other passes if desired
    let uv = v.uv; // keep as-is

    switch material.mode {
        // Solid
        case 0u: { return material.primary_color; }
        // Linear gradient
        case 1u: {
            let centered = uv - 0.5;
            let rotated = rotate(centered, material.params.x) + 0.5;
            let t = clamp(rotated.y, 0.0, 1.0);
            return mix(material.primary_color, material.secondary_color, t);
        }
        // Radial gradient
        case 2u: {
            let d = distance(uv, material.radial_center);
            let r = material.params.w; // radial_radius
            let t = smoothstep(0.0, r, d);
            return mix(material.primary_color, material.secondary_color, t);
        }
        // Animated
        case 3u: {
            let time = material.params.y;
            let speed = material.params.z;
            let wave = sin(time * speed + uv.y * 3.14159265) * 0.5 + 0.5;
            let t = wave;
            return mix(material.primary_color, material.secondary_color, t);
        }
        default: { return material.primary_color; }
    }
}
