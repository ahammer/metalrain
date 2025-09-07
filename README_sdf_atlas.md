# SDF Atlas Workflow

This document describes how to generate and integrate the SDF (signed distance field) shape atlas used by the metaballs + SDF hybrid renderer.

## Overview
The runtime expects two files under `assets/shapes/` (paths currently hard‑coded):

- `sdf_atlas.png` – A packed texture atlas containing per‑shape distance field tiles (square tiles, uniform size).
- `sdf_atlas.json` – Metadata describing each shape (versioned schema v1).

If these files are absent or the feature is disabled in config, the renderer falls back to analytic circle metaballs.

## Config Toggle
`GameConfig.sdf_shapes` (RON: `sdf_shapes`) controls feature usage:
```
sdf_shapes: (
  enabled: true,
  force_fallback: false,   // When true forces ignoring atlas even if present
  max_gradient_samples: 2, // 0 disables gradient, >0 enables finite difference
)
```

`max_gradient_samples` is capped at 4 (validation warning if higher). Gradient sampling is a small forward‑difference approximation used for shading hints.

## JSON Schema (version = 1)
Example:
```json
{
  "version": 1,
  "distance_range": 8.0,
  "tile_size": 64,
  "atlas_width": 256,
  "atlas_height": 256,
  "channel_mode": "sdf_r8",
  "shapes": [
    {
      "name": "circle",
      "index": 0,
      "px": { "x":0, "y":0, "w":64, "h":64 },
      "uv": { "u0":0.0, "v0":0.0, "u1":0.25, "v1":0.25 },
      "pivot": { "x":0.5, "y":0.5 },
      "advance_scale": null,
      "metadata": {}
    }
  ]
}
```
Fields:
- `distance_range`: Signed distance normalization span (heuristic default = tile_size * 0.125 if omitted).
- `tile_size`: Square tile dimension in pixels.
- `atlas_width` / `atlas_height`: Full atlas dimensions in pixels.
- `channel_mode`: `sdf_r8` (current), planned future `msdf_rgb`, `msdf_rgba`.
- `shapes`: Ordered list; each `index` should match its position.
- `pivot`: Normalized pivot inside the tile (0..1) used for shape anchoring or future layout.

A sentinel dummy entry is reserved on the GPU at index 0 (system injects); any shape_index == 0 in a ball reverts to analytic circle distance.

## Generation Tool
A helper binary `sdf_atlas_gen` converts a registry JSON + atlas PNG into the runtime JSON schema.

Build & run (example):
```
cargo run --bin sdf_atlas_gen -- \
  --registry distance-field-generator/sample_sdf_registry.json \
  --atlas-png assets/shapes/sdf_atlas.png \
  --out-json assets/shapes/sdf_atlas.json \
  --tile-size 64
```
Optional flags:
- `--atlas-width`, `--atlas-height` override PNG dimensions (otherwise inferred).
- `--distance-range <f32>` override heuristic.
- `--channel-mode sdf_r8|msdf_rgb|msdf_rgba` (only `sdf_r8` shader path active right now).

### Expected Registry Input
Either:
1. Raw array `[...]` of entries, or
2. Object `{ "shapes": [ ... ] }`.

Entry fields:
```
name, index, x, y, w, h, u0, v0, u1, v1, (optional pivot_x, pivot_y)
```
All tiles must currently be square and identical size (`w == h == tile_size`). Warnings are emitted if mismatched.

### Distance Range
The shader normalizes signed distance using `distance_range` to map texture distance into field contribution space. Choose a value roughly equal to the pixel span of reliable distance accuracy (outer band before clamp). The heuristic (tile_size * 0.125) is conservative; tweak if edges appear too soft or too hard.

## Runtime Loading
1. Startup system (`SdfAtlasPlugin`) attempts to read both files.
2. On success it builds a GPU storage buffer (`SdfShapeGpuMeta`) with per‑shape UV bounds & pivot.
3. Uniform lane v5 is updated:
   - x: enabled flag
   - y: distance_range
   - z: channel_mode enum (0=r8,1=rgb,2=rgba)
   - w: max_gradient_samples
4. Shader branches per ball: if packed shape index (high 16 bits of `GpuBall.w`) != 0 and SDF enabled, it samples the atlas; else analytic circle math.

## Packing Shape Index
High 16 bits: shape index, low 16 bits: color group. Tools that author balls must replicate this packing: `((shape_index as u32) << 16) | (color_group as u32)`.

## Future Extensions
- Multi-channel MSDF decoding (median/precision improvements).
- Central difference gradient (current is forward diff) with configurable epsilon.
- Atlas dimension uniform to refine distance normalization per shape.
- Optional compression / KTX2 path.

## Troubleshooting
| Symptom | Cause | Fix |
|---------|-------|-----|
| Shader still shows circles | Atlas missing or `sdf_shapes.enabled=false` | Add files / enable config |
| All shapes same scale | Incorrect index packing | Verify high 16 bits contain shape index |
| Jagged edges | distance_range too small | Increase explicit `--distance-range` |
| Soft / inflated edges | distance_range too large | Decrease value |
| Gradient disabled | `max_gradient_samples` = 0 | Raise to 1 or 2 |

## Notes
- The loader logs with target `sdf`; filter via `RUST_LOG=sdf=info` for focused output.
- Increasing tile_size raises VRAM and upload cost; profile before scaling.

---
Generated documentation (keep updated if schema or uniform semantics change).
