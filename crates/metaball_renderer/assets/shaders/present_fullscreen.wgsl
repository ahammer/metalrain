// Present fragment shader – simplified solid fill version
//
// Goal: remove bevel, edge tinting, specular, fresnel, highlight bands, and shadow logic.
// Only keep:
//   * Iso evaluation + smooth edge AA
//   * Optional per‑pixel albedo override (if albedo tex supplies coverage)
//   * Subtle background vertical gradient (can be removed if desired)
// Result: metaball silhouette filled with a constant (or albedo) color, no lighting.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var albedo_tex: texture_2d<f32>;
@group(2) @binding(2) var present_sampler: sampler;
@group(2) @binding(3) var<uniform> present_params: PresentParams;

struct PresentParams {
    scale_offset: vec4<f32>, // (scale_u, offset_u, scale_v, offset_v)
}

// Iso-surface & edge AA
const ISO: f32             = 0.50;
const EDGE_BAND: f32       = 1.50;        // widen AA band a bit for smooth edge
const USE_DERIV_EDGE: bool = true;        // derivative-based width (fast & stable)

// Solid fill color fallback (used if albedo texture has no coverage)
const SOLID_COLOR: vec3<f32> = vec3<f32>(0.60, 0.62, 0.70);

// Background vertical gradient (optional aesthetic)
const BG_TOP: vec3<f32> = vec3<f32>(0.09, 0.10, 0.13);
const BG_BOT: vec3<f32> = vec3<f32>(0.03, 0.035, 0.06);

// Utility functions
fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a * (1.0 - t) + b * t;
}
fn sample_packed(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(present_tex, present_sampler, uv, 0.0);
}
fn sample_albedo(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(albedo_tex, present_sampler, uv, 0.0);
}
// (All previous bevel / lighting helpers removed.)

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2(v.uv.x,1.0-v.uv.y);
    // Apply CPU-provided cropping params to keep 1:1 aspect while filling screen.
    // (scale_u, offset_u, scale_v, offset_v)
    let so = present_params.scale_offset;
    let sample_uv = vec2<f32>(so.y + uv.x * so.x, so.w + uv.y * so.z);

    let dims = vec2<f32>(f32(textureDimensions(present_tex, 0).x),
                         f32(textureDimensions(present_tex, 0).y));

    let packed       = sample_packed(sample_uv);
    let field        = packed.r;

    var w = max(fwidth(field) * EDGE_BAND, 1e-4);
    if (!USE_DERIV_EDGE) { w = 0.01; }

    let inside_mask = smoothstep(ISO - w, ISO + w, field);

    // Base fill color: albedo (if any) else constant
    let albedo = sample_albedo(sample_uv);
    let fill_rgb = select(SOLID_COLOR, albedo.rgb / max(albedo.a, 1e-6), albedo.a > 0.001);

    let blob_rgb = fill_rgb; // no lighting / shading

    // Background gradient
    let bg = lerp(BG_BOT, BG_TOP, clamp(uv.y, 0.0, 1.0));

    let out_rgb = lerp(bg, blob_rgb, inside_mask);
    return vec4<f32>(out_rgb, 1.0);
}
