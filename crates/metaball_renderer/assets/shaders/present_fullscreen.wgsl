#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var albedo_tex: texture_2d<f32>;
@group(2) @binding(2) var present_sampler: sampler;

// Packed metaball field texture layout (rgba16float) produced by compute_metaballs.wgsl:
//   R: scalar field value  Σ (r_i^2 / d_i^2)
//      - Monotonically increases toward ball centers; iso surface threshold (ISO) defines silhouette.
//   G: normalized field gradient X component (∂field/∂x divided by |∇field|). 0 if gradient tiny.
//   B: normalized field gradient Y component. 0 if gradient tiny.
//   A: inverse gradient length 1/|∇field| (clamped) – enables approximate signed distance:
//        signed_distance ≈ (field - ISO) * inv_grad_len
//
// Present shader usage summary:
//   * Edge anti-aliasing width derives from derivatives of the scalar field (fwidth(field)).
//   * Gradient (G,B) is NOW used for simple directional lighting (added in this revision).
//     Stored gradient points toward increasing field (toward ball centers), so the outward
//     surface normal is -normalize(grad). We treat the surface as 2D and perform Lambert
//     lighting in the XY plane. For a faux 3D look we optionally fold in a Z component.
//   * inv_grad_len (A) currently unused here; kept to preserve contract and enable fast SDF-based
//     effects (outline, glow, thickness) without another pass.
//   * Albedo comes from a separate RGBA8 texture where alpha encodes coverage (premultiplied).
//
// If you change this packing, update both this comment and compute_metaballs.wgsl. Prefer adding
// features via new passes or repurposed A channel only after confirming size / precision impacts.

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
// --- Lighting Configuration ----------------------------------------------------------
// A lightweight, constant directional light to give metaballs form using the encoded normals.
// Keep constants here (no extra uniforms) to avoid changing the Rust-side material packing.
// If dynamic lighting is desired later, move these into a uniform struct appended to the
// existing header while respecting 16B alignment.
// Pre-normalized light direction (can't call normalize() in a const expr for all backends)
// Original desired vector (0.38, 0.92) length ≈ 0.99539; divide components by that length.
const LIGHT_DIR_2D: vec2<f32> = vec2<f32>(0.3815, 0.9244);
const AMBIENT: f32 = 0.35;
const DIFFUSE_STRENGTH: f32 = 0.75;
const USE_FAUX_Z: bool = true; // If true, build a pseudo 3D normal using gradient length.

// Chamfer ("flat sphere") controls: interior stays flatter, edges roll off.
// CHAMFER_WIDTH: signed-distance range (approx) over which we transition from edge curvature
//                 to flat top. Adjust to control thickness of the bevel.
// CHAMFER_CURVATURE: z component of edge normal prior to blending (higher = steeper edge).
const CHAMFER_WIDTH: f32 = 5.135;
const CHAMFER_CURVATURE: f32 = 0.6;

// Flat interior + spherical edge chamfer normal reconstruction.
fn decode_chamfered_flat_sphere_normal(
    packed_grad: vec2<f32>,
    inv_grad_len: f32,
    field_value: f32,
    iso_threshold: f32
) -> vec3<f32> {
    let grad2 = -packed_grad;          // outward
    let glen = length(grad2);
    let dir2 = select(vec2<f32>(0.0, 0.0), grad2 / glen, glen > 1e-6);
    // Approx signed distance (positive inside surface)
    let sd = (field_value - iso_threshold) * inv_grad_len;
    // Blend factor 0 at surface, 1 after chamfer width
    let t = clamp(sd / max(CHAMFER_WIDTH, 1e-6), 0.0, 1.0);
    // Edge normal: outward with curvature
    let edge_n = normalize(vec3<f32>(dir2.x, dir2.y, CHAMFER_CURVATURE));
    let flat_n = vec3<f32>(0.0, 0.0, 1.0);
    // Smooth easing favoring earlier flattening
    let w = t * t * (3.0 - 2.0 * t);
    return normalize(mix(edge_n, flat_n, w));
}

fn apply_lighting(base: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    // Project light into 3D if faux Z used; otherwise operate in plane.
    let l = normalize(select(vec3<f32>(LIGHT_DIR_2D, 0.0), vec3<f32>(LIGHT_DIR_2D.x, LIGHT_DIR_2D.y, 0.6), USE_FAUX_Z));
    let ndotl = max(dot(normal, l), 0.0);
    let lit = base * (AMBIENT + DIFFUSE_STRENGTH * ndotl);
    return lit;
}

// Computes the interior surface fill color for the metaball silhouette.
// Rationale: we will later expand this to support bevel / lighting again; isolating
// the logic now makes future experimentation cheap. For now we simply modulate
// the flat fill color by a density term so there is a very subtle gradient from
// center (higher field value) toward the iso edge.
// Parameters:
//   field    – raw scalar field sample (0..~1)
//   iso      – iso threshold defining the surface
//   width    – derivative based edge width (currently unused but kept for future AA-aware shading)
//   fill_rgb – base (albedo) color prior to shading
// Returns a color already premultiplied by a simple density factor.
fn compute_surface_fill(field: f32, iso: f32, width: f32, fill_rgb: vec3<f32>) -> vec3<f32> {
    let nfield = (field - iso) / (1.0 - iso);
    return fill_rgb; // * vec3(nfield, nfield, nfield);
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2(v.uv.x, 1.0 - v.uv.y);
    let packed       = sample_packed(uv);
    let field        = packed.r;
    var w = max(fwidth(field) * EDGE_BAND, 1e-4);
    if (!USE_DERIV_EDGE) { w = 0.01; }
    let inside_mask = smoothstep(ISO - w, ISO + w, field);
    let albedo = sample_albedo(uv);
    let fill_rgb = select(SOLID_COLOR, albedo.rgb / max(albedo.a, 1e-6), albedo.a > 0.001);
    var blob_rgb = compute_surface_fill(field, ISO, w, fill_rgb);
    // Lighting: only bother computing normals where inside_mask > ~0 (avoid background cost).
    if (inside_mask > 0.001) {
        let normal = decode_chamfered_flat_sphere_normal(packed.gb, packed.a, field, ISO); // packed.a = inv_grad_len
        blob_rgb = apply_lighting(blob_rgb, normal);
    }
    let bg = lerp(BG_BOT, BG_TOP, clamp(uv.y, 0.0, 1.0));
    let out_rgb = lerp(bg, blob_rgb, inside_mask);
    return vec4<f32>(out_rgb, 1.0);
}
