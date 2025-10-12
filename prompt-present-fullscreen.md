# Prompt: Enhance the `present_fullscreen.wgsl` Shader

## High-Level Goal

Your task is to transform the `present_fullscreen.wgsl` shader from a simple density visualizer into a polished, visually appealing "liquid metal" or "lava lamp" effect.

The key is to take the raw, linear density data from the compute shader's output texture and apply a series of rendering techniques entirely within the presentation fragment shader.

**CRITICAL CONSTRAINT**: You MUST NOT change the contract with the compute shader. The input is a 2D texture (`t_field`) where the `.r` channel contains a linear density value. All visual enhancement must be derived from this single data source.

---

## Plan of Action & Visual Impact

Follow these steps in order. Each step builds upon the last and explains *why* the change improves the visual output.

### Phase 1: Remap the Density Curve (Ease-In/Out)

**What to do:**
1.  The raw density from `textureSample(t_field, ...).r` has a linear falloff. This looks flat and unnatural.
2.  Create a `quintic_smoothstep` function: `t * t * t * (t * (t * 6.0 - 15.0) + 10.0)`. This function provides a high-quality ease-in and ease-out curve.
3.  Apply this function to the `raw_density` immediately after sampling it. Use `saturate()` on the input to ensure it's clamped to the `[0.0, 1.0]` range.
4.  All subsequent calculations in the shader MUST use this new, remapped `density` value, not the raw one.

**Visual Impact:**
This is the most important step. It will transform the metaballs from fuzzy, linear gradients into objects with a defined, "gel-like" surface and a sharp, pleasing falloff at the edges.

### Phase 2: Faux 3D Lighting via Normal Approximation

**What to do:**
1.  To give the 2D effect a sense of volume, you need to simulate lighting. This requires a surface normal.
2.  Approximate the normal by calculating the gradient of the density field. Sample the density texture at four neighboring points (to the right, left, top, and bottom of the current fragment's UV).
3.  **CRITICAL**: When you sample the neighbors, you MUST apply the same `quintic_smoothstep` remapping from Phase 1 to each sample. This ensures the lighting reflects the new, curved surface, not the old linear one.
4.  Calculate the difference between the horizontal samples (`dx`) and vertical samples (`dy`) to create a 2D normal vector: `normal = normalize(vec2(dx, dy))`.
5.  Define a static light direction (e.g., `normalize(vec2(0.8, 0.9))`).
6.  Calculate a diffuse lighting term: `diffuse = saturate(dot(normal, light_dir))`. Add a small ambient term (e.g., `diffuse * 0.7 + 0.3`) to prevent shadows from being pure black.

**Visual Impact:**
This will make the metaballs look like 3D spheres with highlights and shadows, giving them depth and form.

### Phase 3: Gradient Coloring

**What to do:**
1.  A single color is flat. Use the remapped `density` to map to a color gradient.
2.  Define two colors representing the "core" and "edge" of the liquid (e.g., a deep blue and a bright cyan).
3.  Use the `mix()` function to blend between these two colors. The interpolant (`t`) should be the remapped `density`, further shaped by `smoothstep` to control the gradient's transition (e.g., `smoothstep(0.2, 0.8, density)`).
4.  Multiply the resulting `base_color` by the `diffuse` lighting term from Phase 2.

**Visual Impact:**
This adds richness and reinforces the sense of volume. The color transition will follow the shape of the metaballs, making them more dynamic.

### Phase 4: Final Composition & Optimization

**What to do:**
1.  The background should be black or a very dark color.
2.  Use the final remapped `density` as an alpha value to blend the final lit color over the background.
3.  **CRITICAL OPTIMIZATION**: At the beginning of the shader, after calculating the remapped `density`, check if it's below a small threshold (e.g., `0.01`). If it is, `discard` the fragment.

**Visual Impact:**
Discarding empty pixels significantly improves performance by avoiding unnecessary lighting and color calculations for the vast majority of the screen. It also cleans up any faint, noisy artifacts in the background.

---

## Final Shader Structure Example

Your final fragment shader should have a logical flow like this:

```wgsl
@fragment
fn fs(...) -> @location(0) vec4<f32> {
    // 1. Sample raw density
    let raw_density = textureSample(...).r;

    // 2. Apply easing curve (Phase 1)
    let density = quintic_smoothstep(saturate(raw_density));

    // 3. Optimization: Discard empty fragments (Phase 4)
    if (density < 0.01) {
        discard;
    }

    // 4. Calculate normals using remapped density (Phase 2)
    // ... sample neighbors, apply quintic_smoothstep to each ...
    let normal = ...;

    // 5. Calculate lighting (Phase 2)
    let diffuse = ...;

    // 6. Calculate gradient color (Phase 3)
    let base_color = mix(...);

    // 7. Final color calculation
    let final_color = base_color * diffuse;

    return vec4(final_color, 1.0);
}
```
