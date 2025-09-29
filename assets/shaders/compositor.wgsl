#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct CompositorUniforms {
    settings: vec4<f32>,
    enabled_low: vec4<f32>,
    enabled_high: vec4<f32>,
    blend_modes_low: vec4<f32>,
    blend_modes_high: vec4<f32>,
}

@group(2) @binding(0) var<uniform> compositor: CompositorUniforms;
@group(2) @binding(1) var background_tex: texture_2d<f32>;
@group(2) @binding(2) var background_sampler: sampler;
@group(2) @binding(3) var game_tex: texture_2d<f32>;
@group(2) @binding(4) var game_sampler: sampler;
@group(2) @binding(5) var metaballs_tex: texture_2d<f32>;
@group(2) @binding(6) var metaballs_sampler: sampler;
@group(2) @binding(7) var effects_tex: texture_2d<f32>;
@group(2) @binding(8) var effects_sampler: sampler;
@group(2) @binding(9) var ui_tex: texture_2d<f32>;
@group(2) @binding(10) var ui_sampler: sampler;

fn layer_enabled(index: u32) -> f32 {
    var value = 0.0;
    switch (index) {
        case 0u: { value = compositor.enabled_low.x; }
        case 1u: { value = compositor.enabled_low.y; }
        case 2u: { value = compositor.enabled_low.z; }
        case 3u: { value = compositor.enabled_low.w; }
        case 4u: { value = compositor.enabled_high.x; }
        default: {}
    }
    return value;
}

fn layer_blend(index: u32) -> u32 {
    var value = 0.0;
    switch (index) {
        case 0u: { value = compositor.blend_modes_low.x; }
        case 1u: { value = compositor.blend_modes_low.y; }
        case 2u: { value = compositor.blend_modes_low.z; }
        case 3u: { value = compositor.blend_modes_low.w; }
        case 4u: { value = compositor.blend_modes_high.x; }
        default: {}
    }
    return u32(value + 0.5);
}

fn apply_blend(base: vec4<f32>, layer: vec4<f32>, mode: u32) -> vec4<f32> {
    let alpha = clamp(layer.a, 0.0, 1.0);
    var result = base;
    switch (mode) {
        default: {
            let blended = mix(result.rgb, layer.rgb, alpha);
            let new_alpha = max(result.a, layer.a);
            result = vec4<f32>(blended, new_alpha);
        }
        case 1u: {
            let blended = result.rgb + layer.rgb * alpha;
            let new_alpha = max(result.a, layer.a);
            result = vec4<f32>(blended, new_alpha);
        }
        case 2u: {
            let multiplied = result.rgb * layer.rgb;
            let blended = mix(result.rgb, multiplied, alpha);
            let new_alpha = max(result.a, layer.a);
            result = vec4<f32>(blended, new_alpha);
        }
    }
    return result;
}

fn sample_layer(texture: texture_2d<f32>, sampler_inst: sampler, uv: vec2<f32>) -> vec4<f32> {
    return textureSample(texture, sampler_inst, uv);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if layer_enabled(0u) > 0.5 { color = sample_layer(background_tex, background_sampler, uv); }
    if layer_enabled(1u) > 0.5 { let layer = sample_layer(game_tex, game_sampler, uv); color = apply_blend(color, layer, layer_blend(1u)); }
    if layer_enabled(2u) > 0.5 { let layer = sample_layer(metaballs_tex, metaballs_sampler, uv); color = apply_blend(color, layer, layer_blend(2u)); }
    if layer_enabled(3u) > 0.5 { let layer = sample_layer(effects_tex, effects_sampler, uv); color = apply_blend(color, layer, layer_blend(3u)); }
    if layer_enabled(4u) > 0.5 { let layer = sample_layer(ui_tex, ui_sampler, uv); color = apply_blend(color, layer, layer_blend(4u)); }
    let exposure = compositor.settings.x;
    color = vec4<f32>(color.rgb * exposure, color.a);
    color = clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
    if compositor.settings.y > 0.5 {
        let border = step(uv.x, 0.01) + step(uv.y, 0.01) + step(0.99, uv.x) + step(0.99, uv.y);
        if border > 0.0 { return vec4<f32>(1.0, 0.2, 0.2, 1.0); }
    }
    return color;
}
