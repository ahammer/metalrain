#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var present_tex: texture_2d<f32>;
@group(2) @binding(1) var albedo_tex: texture_2d<f32>;
@group(2) @binding(2) var present_sampler: sampler;
@group(2) @binding(3) var normals_tex: texture_2d<f32>; // sampled (prepared) but not yet used for shading
@group(2) @binding(4) var<uniform> viz_mode: u32; // 0=Normal, 1=DistanceField, 2=Normals3D, 3=Gradient2D, 4=Coverage, 5=InverseGradient

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
const ISO: f32             = 1.0;
const EDGE_BAND: f32       = 0.50;        // widen AA band a bit for smooth edge
const USE_DERIV_EDGE: bool = true;        // derivative-based width (fast & stable)

// Solid fill color fallback (used if albedo texture has no coverage)
const SOLID_COLOR: vec3<f32> = vec3<f32>(0.60, 0.62, 0.70);

// Background vertical gradient (optional aesthetic)
const BG_TOP: vec3<f32> = vec3<f32>(0.09, 0.10, 0.13);
const BG_BOT: vec3<f32> = vec3<f32>(0.03, 0.035, 0.06);

// Lighting constants (simple two-directional light rig + ambient). These are intentionally
// conservative to preserve existing color palette while adding shape definition.
const AMBIENT_INTENSITY: f32 = 0.35;
const SPECULAR_POWER: f32    = 64.0;
const SPECULAR_STRENGTH: f32 = 1.55;   // scales specular term (premultiplied by mask later)
const LIGHT0_DIR: vec3<f32>  = vec3<f32>(-0.35, 0.60, 0.72); // key
const LIGHT0_COLOR: vec3<f32> = vec3<f32>(1.00, 0.96, 0.90);
const LIGHT1_DIR: vec3<f32>  = vec3<f32>(0.80, 0.25, 0.50);  // fill / rim
const LIGHT1_COLOR: vec3<f32> = vec3<f32>(0.45, 0.55, 1.00);

// Soft shadow constants (newly added)
const SHADOW_DISTANCE: f32 = 0.005; // Controls how far the shadow is cast.
const SHADOW_OFFSET: vec2<f32> = LIGHT0_DIR.xy * SHADOW_DISTANCE; // Shadow direction derived from key light.
const SHADOW_COLOR: vec3<f32>  = vec3<f32>(0.0, 0.0, 0.0);
const SHADOW_FALLOFF_START: f32 = 0.5; // Field value where shadow begins to fade out
const SHADOW_FALLOFF_END: f32   = 1.5; // Field value where shadow is at full strength (ISO)
// Maximum opacity the shadow can contribute (before union w/ blob alpha)
const SHADOW_ALPHA: f32 = 0.6;

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
fn sample_normals(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(normals_tex, present_sampler, uv, 0.0);
}

// Computes a soft shadow occlusion factor by sampling the field at an offset.
// If the offset sample is inside a metaball, it contributes to the shadow.
// Returns:
//   A value from 0.0 (no shadow) to 1.0 (full shadow).
fn compute_soft_shadow(uv: vec2<f32>) -> f32 {
    let shadow_uv = uv + SHADOW_OFFSET;
    let shadow_field = sample_packed(shadow_uv).r;
    // Calculate occlusion: 1.0 if shadow_field is >= END, 0.0 if <= START
    let occlusion = smoothstep(SHADOW_FALLOFF_START, SHADOW_FALLOFF_END, shadow_field);
    return occlusion;
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
    return fill_rgb;
}

// Decode normal from the normals texture. The compute shader already stores normalized
// vectors in the [-1, 1] range. This function simply ensures they remain normalized
// after sampling. The previous `rgb * 2.0 - 1.0` was incorrect as it assumed [0,1] packing.
fn decode_normal(rgb: vec3<f32>) -> vec3<f32> {
    // The normals are already in the correct [-1, 1] range.
    // We just re-normalize to be safe from any potential interpolation artifacts.
    return normalize(rgb);
}

// Apply simple lighting to the metaball surface color. Lighting is only applied where mask > 0
// (the smooth silhouette region). We keep it branch‑free by math mixing. The supplied color is
// the already composited scene (bg blended with blob) when called following the requested
// pattern (out_pre_lighting -> out_rgb = add_lighting). To avoid tinting the background we also
// accept the mask so we can lerp the lit & unlit colors.
fn add_lighting(color: vec3<f32>, normal: vec3<f32>, mask: f32) -> vec3<f32> {
    let n = normalize(normal);
    let l0 = normalize(LIGHT0_DIR);
    let l1 = normalize(LIGHT1_DIR);
    let ndotl0 = max(dot(n, l0), 0.0);
    let ndotl1 = max(dot(n, l1), 0.0);
    let diffuse = ndotl0 * LIGHT0_COLOR + ndotl1 * LIGHT1_COLOR;
    let view_dir = vec3<f32>(0.0, 0.0, 1.0); // screen‑space camera facing -Z toward viewer
    let half0 = normalize(l0 + view_dir);
    let half1 = normalize(l1 + view_dir);
    let spec0 = pow(max(dot(n, half0), 0.0), SPECULAR_POWER);
    let spec1 = pow(max(dot(n, half1), 0.0), SPECULAR_POWER);
    let spec = (spec0 * LIGHT0_COLOR + spec1 * LIGHT1_COLOR) * SPECULAR_STRENGTH;
    let lit = color * (AMBIENT_INTENSITY + diffuse) + spec;
    // Prevent background lighting: blend only where mask > 0 (smooth edge preserved)
    return mix(color, lit, clamp(mask, 0.0, 1.0));
}

@fragment
fn fragment(v: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2(v.uv.x, 1.0 - v.uv.y);
    let packed       = sample_packed(uv);
    let field        = packed.r;
    let normals_sample = sample_normals(uv);
    var w = max(fwidth(field) * EDGE_BAND, 1e-4);
    if (!USE_DERIV_EDGE) { w = 0.01; }
    let inside_mask = smoothstep(ISO - w, ISO + w, field);
    let albedo = sample_albedo(uv);
    let fill_rgb = select(SOLID_COLOR, albedo.rgb / max(albedo.a, 1e-6), albedo.a > 0.001);
    var blob_rgb = compute_surface_fill(field, ISO, w, fill_rgb);
    let shadow_occlusion = compute_soft_shadow(uv);
    let shadow_strength = shadow_occlusion * (1.0 - inside_mask);
    let shadow_alpha = shadow_strength * SHADOW_ALPHA;
    let normal = decode_normal(normals_sample.rgb);
    let lit_blob_rgb = add_lighting(blob_rgb, normal, 1.0);
    let blob_alpha = inside_mask;
    let blob_rgb_premul = lit_blob_rgb * blob_alpha;
    let shadow_rgb_premul = SHADOW_COLOR * shadow_alpha;
    let out_alpha = blob_alpha + shadow_alpha * (1.0 - blob_alpha);
    let out_rgb = blob_rgb_premul + shadow_rgb_premul * (1.0 - blob_alpha);

    // Visualization mode switching
    switch viz_mode {
        case 1u: { // Distance Field (R channel)
            let vis = vec3<f32>(field, field, field);
            return vec4<f32>(vis, 1.0);
        }
        case 2u: { // 3D Normals
            // Show normals only where there's a surface
            let norm_vis = normal * 0.5 + 0.5; // Map [-1,1] to [0,1] for visualization
            return vec4<f32>(norm_vis, max(inside_mask, 0.3));
        }
        case 3u: { // 2D Gradient (G,B channels)
            let grad = vec2<f32>(packed.g, packed.b);
            return vec4<f32>(grad.x * 0.5 + 0.5, grad.y * 0.5 + 0.5, 0.5, 1.0);
        }
        case 4u: { // Coverage/Albedo
            return vec4<f32>(fill_rgb * inside_mask, inside_mask);
        }
        case 5u: { // Inverse Gradient Length (A channel)
            let inv_grad = packed.a;
            return vec4<f32>(inv_grad, inv_grad, inv_grad, 1.0);
        }
        default: { // 0 = Normal rendering
            return vec4<f32>(out_rgb, out_alpha);
        }
    }
}
