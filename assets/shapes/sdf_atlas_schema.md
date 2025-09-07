SDF Atlas JSON Schema (Version 1)
=================================

This document describes the structure of the `sdf_atlas.json` file produced by the procedural
builders (`sdf_atlas_build`, `sdf_atlas_gen`) and consumed at runtime.

Root Object Fields
------------------
version (u32)
  Schema / format version. Currently always 1.

distance_range (f32)
  The maximum absolute signed distance (in *pixels*) encoded in the atlas. Values in the PNG are
  normalized such that 128 represents the surface (distance == 0). The encoding used by the
  builder is: encoded = 0.5 - clamp(sd_px, -distance_range, distance_range)/distance_range.

tile_size (u32)
  Width/height in pixels of each square tile. All shapes occupy tiles aligned to this grid.

atlas_width / atlas_height (u32)
  Pixel dimensions of the full atlas PNG.

channel_mode (string)
  One of: `sdf_r8`, `msdf_rgb`, `msdf_rgba`. Currently only `sdf_r8` is emitted by the procedural builder.

shapes (array<ShapeEntry>)
  List of shape metadata records. Indices start at 1; 0 is reserved as a sentinel in runtime code.

ShapeEntry
----------
name (string)
  Symbolic name (e.g. `circle`, `triangle`, `glyph_A`, `glyph_a`). Alphanumeric glyphs include
  digits 0-9, uppercase A-Z, and lowercase a-z when generated via `sdf_atlas_build`.

index (u32)
  Sequential index (1-based) used for GPU lookup / indirection.

px (Rect)
  Pixel rectangle inside the atlas: { x, y, w, h }. w and h equal `tile_size`.

uv (UvRect)
  Normalized (0..1) UV coordinates for the rectangle: { u0, v0, u1, v1 }.

pivot (Vec2)
  Normalized pivot inside the tile (0..1). Currently always {0.5, 0.5} (center) from the procedural builder.

advance_scale (optional f32)
  Placeholder for future text layout usage. Usually null/omitted at present.

metadata (object)
  Opaque key/value map for per-shape extensions. Current recognized fields:

  padding_px (u32)
    Added in September 2025. Number of pixels of uniform padding on all sides that were RESERVED
    when sampling the signed distance for this tile. The shape geometry was mapped only into the
    inner (tile_size - 2*padding_px) square region. Padding allows the distance field to fade to
    the background (black) without clipping and prevents warping/distortion by preserving a square
    mapping region for every glyph/shape.

    Notes:
    * All shapes in a generated atlas share the same padding value (currently global CLI argument
      --padding-px from `sdf_atlas_build`). It is duplicated per entry for simplicity / future
      per-shape control.
    * Runtime consumers that reconstruct signed distance may optionally treat the effective
      content size as (tile_size - 2*padding_px) for tight layout calculations.

Uniform Glyph Scaling & Centering
---------------------------------
Glyph outlines are scaled uniformly using the maximum of their bounding box width/height, centered
within the logical content square (tile minus padding). This removes prior aspect distortion.
Y is flipped during outline sampling to correct for font coordinate system orientation. The result
is that all glyphs are:
  * Centered
  * Uniformly scaled (no warping)
  * Surrounded by explicit empty border padding

Forward Compatibility
---------------------
New metadata keys may be added. Consumers should ignore unknown keys. The root `version` will only
be bumped for structural / semantic breaking changes.

Example Snippet
---------------
{
  "version": 1,
  "distance_range": 32.0,
  "tile_size": 64,
  "atlas_width": 512,
  "atlas_height": 512,
  "channel_mode": "sdf_r8",
  "shapes": [
    {
      "name": "glyph_A",
      "index": 5,
      "px": { "x": 128, "y": 0, "w": 64, "h": 64 },
      "uv": { "u0": 0.25, "v0": 0.0, "u1": 0.375, "v1": 0.125 },
      "pivot": { "x": 0.5, "y": 0.5 },
      "metadata": { "padding_px": 6 }
    }
  ]
}

CLI Additions (`sdf_atlas_build`)
---------------------------------
--padding-px <u32>
  Default: 0. Uniform pixel padding inserted around every tile's drawable content region.
  Constraint: 2*padding_px < tile_size.

Recommended Values
------------------
For typical single-channel SDF usage with smooth silhouettes choose padding between 6 and 12 pixels
for tile_size 64–128 to ensure the surface (value 128) has room to fall off into a fully exterior
band, particularly for large, nearly tile-filling shapes.

Change Log (schema related)
---------------------------
2025-09-07: Added `metadata.padding_px`, uniform glyph scaling & centering description, and lowercase a-z glyph support.
