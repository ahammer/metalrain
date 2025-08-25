# Metaball Surface Noise Feature

Organic animated high‑frequency modulation applied to metaball borders. Independently configurable from background procedural noise and skip‑able at zero cost when disabled.

## Config (`game.ron`)

```
surface_noise: (
    enabled: true,      // Master toggle (0 cost when false)
    mode: 0,            // 0 = add to field (alters local thickness), 1 = iso shift (moves contour only)
    amp: 0.08,          // 0..0.5 (clamped). Typical subtle range 0.03–0.15
    base_scale: 0.008,  // Inverse spatial scale (higher => finer detail). Must be > 0
    warp_amp: 0.3,      // Domain warp strength (0 disables warp branch)
    warp_freq: 1.2,     // Warp frequency multiplier
    speed_x: 0.20,      // Animation velocity X
    speed_y: 0.17,      // Animation velocity Y
    octaves: 4,         // 0..6 (0 = disabled fast path). >4 increases cost with diminishing returns
    gain: 0.55,         // fBm amplitude decay per octave
    lacunarity: 2.05,   // Frequency growth per octave
    contrast_pow: 1.10, // Post shaping (gamma-like)
    ridged: false,      // Ridged variant (mirrors background noise style)
)
```

All fields have serde defaults; older configs remain valid.

## Modes

- Mode 0 (field add): `best_field += amp*(n-0.5)`. Slight local thickening / thinning.
- Mode 1 (iso shift): `effective_iso = iso + amp*(n-0.5)`. Leaves field energy intact; only contour moves.

## Validation & Clamps

| Field | Clamp / Rule | Reason |
|-------|--------------|--------|
| amp | [0, 0.5] | Prevent extreme aliasing |
| base_scale | > 0 required | Avoid division / degenerate scale |
| octaves | 0..6 (warn if >6) | Bound loop & perf |
| octaves==0 + enabled | Warn | No visual effect (prefer enabled=false) |

## Performance

- Disabled OR amp≈0: early branch -> zero extra work (compiler can DCE).
- Active cost: one fBm evaluation (≤6 octaves) + optional 2 value noise samples for domain warp.
- Default (4 octaves, warp) adds O(1) small ALU block per fragment; measured target regression <5% on typical GPU.
- Recommendations:
  - Keep `octaves ≤ 4` for most scenes.
  - Increase `base_scale` (finer) instead of pushing octaves for crisp detail.
  - Set `warp_amp=0` to shave a couple of noise calls if needed.

## Visual Tuning Tips

| Goal | Adjust |
|------|--------|
| Subtler motion | Lower `amp`, maybe reduce `octaves` |
| Finer detail | Raise `base_scale` (NOT amp) |
| Smoother slow billow | Lower `base_scale`, maybe lower `speed_*` |
| Stronger ridges | Set `ridged=true`, maybe increase `contrast_pow` slightly (≤1.3) |
| Avoid popping (mode 1) | Reduce `amp` if iso near extremes |

## Edge Cases

| Case | Effect | Mitigation |
|------|--------|------------|
| Large amp + mode 1 | Noticeable contour popping | Reduce amp or switch to mode 0 |
| Very high octaves (6) | ALU cost increase | Use only for showcase shots |
| base_scale extremely small | Broad low‑freq wobble (not "high freq") | Increase base_scale, reduce amp |

## Implementation Notes

- Separate UBO at `@group(2) @binding(2)` (`SurfaceNoiseParamsUniform`) preserves `MetaballsData` binary layout.
- WGSL helper `surface_noise_scalar` mirrors background noise approach (value noise + optional warp + fBm).
- Single evaluation per fragment; result reused for either field add or iso shift path.
- Early guard:
  ```
  if (surface_noise.enabled == 1u && surface_noise.amp > 0.00001 && surface_noise.octaves > 0u) { ... }
  ```
- Domain warp skipped when `warp_amp == 0`.

## Integration Points

1. Config structures: `SurfaceNoiseConfig` added to `GameConfig`.
2. Uniforms: `SurfaceNoiseParamsUniform` with 16‑byte alignment.
3. Shader: Added surface noise uniform + scalar noise path; minimal edits around mask computation.
4. CPU update: Populated each frame in `update_metaballs_unified_material`.

## Backward Compatibility

- All new fields have defaults (`serde(default)`).
- Existing pipelines, serialized assets, and uniform layouts unaffected.
- Disabling feature reverts visuals to prior results (floating point equivalent aside from expected noise path removal).

## Suggested QA Checklist

1. Observe subtle animated ripples at defaults.
2. Toggle `enabled=false` -> verify previous smooth edges.
3. Switch `mode` between 0 and 1 (expect thickness modulation vs pure contour shift).
4. Stress test: `amp=0.25`, `octaves=6` – ensure no flicker / major perf dip.
5. Verify WASM build (binding order preserved) loads without shader compile errors.
