// Fullscreen inversion post-process (Phase 2 PoC)
// Binds: @group(0) @binding(0) src color texture, @binding(1) sampler
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VSOut {
    // Fullscreen triangle ( -1,-3 ), ( -1, 1 ), ( 3, 1 )
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -3.0),
        vec2(-1.0,  1.0),
        vec2( 3.0,  1.0)
    );
    let p = positions[vi];
    var out: VSOut;
    out.pos = vec4(p, 0.0, 1.0);
    out.uv = 0.5 * (p + vec2(1.0, 1.0));
    return out;
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(src_tex, src_samp, in.uv, 0.0);
    // Invert RGB only, preserve alpha
    return vec4(1.0 - color.r, 1.0 - color.g, 1.0 - color.b, color.a);
}
