// Present fragment shader for Material2d.
// Follows Bevy 0.16 2D material group conventions: material bindings at group(2).
// Vertex shader is the default mesh2d vertex shader; we just sample provided UVs.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
  return textureSampleLevel(present_tex, present_sampler, v.uv, 0.0);
}
