// Present fragment shader for Material2d.
// Follows Bevy 0.16 2D material group conventions: material bindings at group(2).
// Vertex shader is the default mesh2d vertex shader; we just sample provided UVs.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var present_sampler: sampler;

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
  // Sample field stored in RED channel.
  let c = textureSampleLevel(present_tex, present_sampler, v.uv, 0.0);
  let field = c.r; // Unnormalized scalar field value.

  // We take an iso-surface at 50% (0.5). Since the field isn't normalized, this
  // assumes prior logic scaled it into a useful 0..1 range. If not, adjust compute
  // stage or introduce a normalization factor uniform.
  let iso: f32 = 0.9;

  // Anti-aliased edge: use fragment-local derivative width.
  // fwidth gives an estimate of rate-of-change across the pixel; widen band for stability.
  let w = max(fwidth(field), 1e-4);
  let mask = smoothstep(iso - w, iso + w, field);

  // Inside = 1, outside = 0 after smoothing.
  let inside = mask;

  // Simple grayscale visualization (white blob on black background).
  let col = vec3<f32>(inside);
  return vec4<f32>(col, 1.0);
}
