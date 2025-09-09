# SDF Shape Atlas JSON Schema (v1)

This project uses a simple versioned JSON schema to describe a Signed Distance Field (SDF) shape atlas
paired with a PNG texture. The texture encodes single‐channel (R8) or multi‐channel (future MSDF) SDF
values for each shape tile.

## File Pair
- Texture: `assets/shapes/sdf_atlas.png` (planned path; grayscale 8‑bit or RGBA for MSDF later)
- Metadata: `assets/shapes/sdf_atlas.json`

## Top-Level Structure
```jsonc
{
  "version": 1,                // Schema version for forward compatibility
  "distance_range": 8.0,       // (Optional) Max SDF distance encoded in pixels from edge (used to normalize)
  "tile_size": 128,            // Square tile dimension in pixels (all shapes share same size); >0
  "atlas_width": 1024,         // Atlas texture width  in pixels (must be multiple of tile_size)
  "atlas_height": 1024,        // Atlas texture height in pixels (must be multiple of tile_size)
  "channel_mode": "sdf_r8",    // One of: "sdf_r8" | "msdf_rgb" | "msdf_rgba" (future expansion)
  "shapes": [
    {
      "name": "circle_filled",     // Unique identifier; used for debug or semantic mapping
      "index": 0,                   // Sequential index; MUST match position in array (redundant safety)
      "px": { "x":0, "y":0, "w":128, "h":128 }, // Pixel rectangle inside atlas (w,h == tile_size)
      "uv": { "u0":0.0, "v0":0.0, "u1":0.125, "v1":0.125 }, // Precomputed normalized UV rect
      "pivot": { "x":0.5, "y":0.5 },             // Normalized pivot within the tile (0..1) center default
      "advance_scale": 1.0,                         // Reserved: scale factor for layout / spacing
      "metadata": {}                                // Free-form future extension (per-shape flags)
    }
  ]
}
```

## Validation Rules
1. `version` must equal `1` for this loader.
2. `tile_size > 0`.
3. `atlas_width % tile_size == 0` and `atlas_height % tile_size == 0`.
4. `atlas_width / tile_size * atlas_height / tile_size >= shapes.len()` (enough tiles).
5. Each shape rectangle must lie fully inside the atlas and dimensions equal `tile_size`.
6. `index` must equal the zero-based position in `shapes` array.
7. `uv` values must satisfy `0 <= u0 < u1 <= 1` and `0 <= v0 < v1 <= 1`.
8. All `name` values unique.
9. If present, `distance_range > 0`. If absent, a default of `tile_size * 0.125` MAY be assumed.

## Runtime Packing Strategy
Each ball may reference a `shape_index (u16)` which is packed together with the color group id (u16)
into a single `u32` lane (currently the `cluster_slot` / group id lane). Layout proposal:
```
 packed_u32 = (shape_index << 16) | color_group
 shape_index = (packed_u32 >> 16) & 0xFFFF
 color_group = packed_u32 & 0xFFFF
```
A future clickable flag could use the highest bit of `shape_index` (limiting shapes to 32767) if required.

## Shader Binding Additions (planned)
```
@group(2) @binding(7) var sdf_atlas_tex : texture_2d<f32>;
@group(2) @binding(8) var sdf_atlas_samp: sampler;
```
Uniform lanes (reuse existing vectors):
- `metaballs.v5.x` -> `sdf_enabled` (1.0 / 0.0)
- `metaballs.v5.z` -> `sdf_channel_mode` (0 = none, 1 = sdf_r8, 2 = msdf_rgb, 3 = msdf_rgba)
- `metaballs.v5.w` -> `max_gradient_samples` (integer as float; clamps defensive)

## Field Reconstruction
For single‐channel SDF:R8: sample `.r` and remap from `[0,1]` to signed distance in pixels around shape
center using `distance_range`:
```
let d_norm = textureSampleLevel(sdf_atlas_tex, sdf_atlas_samp, uv, 0.0).r;
let signed_px = (d_norm * 2.0 - 1.0) * distance_range; // negative inside (if generation matches convention)
```
The field contribution fed into metaball style pipeline will convert distance to a normalized field value.

## Gradient
Central difference (guarded by max samples / flag):
```
let eps = 1.0 / f32(tile_size);
let dL = sample(u - eps, v).r; let dR = sample(u + eps, v).r;
let dD = sample(u, v - eps).r; let dU = sample(u, v + eps).r;
let grad = vec2<f32>(dR - dL, dU - dD) * (distance_range * 2.0);
```

## Versioning & Future
- `version:2` may introduce per-shape flags, variable tile sizes, or a storage buffer with shape metadata.

---
Document intentionally concise but complete for current prototype stage.
