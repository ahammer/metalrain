---
mode: agent
description: 'Historical (deprecated) secondary metaballs bevel + shadow shader prompt — superseded by unified dual-axis metaballs shader; background quad path removed.'
---

# (DEPRECATED) Metaballs Secondary Shader (Bevel + Shadow) Prompt

## Status
This prompt is retained ONLY for historical context. The project now uses a single unified metaballs shader that internally supports multiple foreground (shading) and background modes. The external background quad, separate bevel material, and secondary shader pathway described below have been REMOVED. Do NOT reintroduce duplicate materials or an external background pass without a clear performance / feature justification.

All requirements referencing:
- `BackgroundQuad`
- Hiding / showing a background quad when switching to bevel mode
- A separate `MetaballsBevelMaterial` or secondary WGSL file
are obsolete and intentionally eliminated.

## Historical Intent (For Reference Only)
Originally, this specification introduced:
1. A secondary WGSL shader (`metaballs_bevel.wgsl`) featuring bevel lighting + drop shadow.
2. PageUp/PageDown cycling between Classic and Bevel modes.
3. Hiding the legacy world-grid / background quad while in bevel mode.
4. Spawning parallel quad entities (classic + bevel) and toggling their visibilities.
5. A solid neutral grey opaque background + internal shadow compositing.

## Current Architecture (Superseding This)
- Single shader: `assets/shaders/metaballs_unified.wgsl`
- Foreground modes: ClassicBlend, Bevel, OutlineGlow (extensible)
- Background modes (internal, opaque): SolidGray, ProceduralNoise, VerticalGradient
- Uniform layout unchanged in size; semantic lanes repurposed for dual-axis mode indices
- No external fullscreen background pass; no secondary material asset required
- Mode cycling uses independent foreground/background resources and updates a single material uniform

## Rationale for Deprecation
- Duplicate accumulation logic increased maintenance cost.
- Visibility toggling of multiple quad entities added ECS overhead with no material benefit.
- Shadow + bevel effects can be implemented as a foreground mode branch inside the unified shader (sharing accumulated field/gradient data).
- Background handling moved fully in-shader, eliminating conditional entity visibility logic.

## Migration Notes
If you encounter old code branches or forks still following this deprecated model:
1. Remove extra metaball quad entities; keep a single quad using the unified material.
2. Fold bevel lighting & shadow logic into the foreground mode switch (reusing gradients).
3. Replace any references to `MetaballRenderMode::{Classic, Bevel}` with the new `MetaballForegroundMode`.
4. Delete any residual `metaballs_bevel.wgsl` file (should no longer exist in mainline).
5. Purge systems whose sole purpose was toggling background component visibility.

## DO NOT Implement (Legacy Instructions Below)
The following historical instructions are intentionally NOT to be (re)implemented unless reinstated via a new design decision:
- Creating `metaballs_bevel.wgsl`
- Spawning both classic + bevel quads
- Hiding background quads via `BackgroundQuad` markers
- Maintaining a separate `MetaballRenderMode` enum
- Shadow sampling via a second O(N) accumulation pass

## Future Extension Guidance (Within Unified Shader)
- Add bevel shadow or soft AO as an optional foreground branch using existing field + gradient (avoid second accumulation loop).
- Integrate parameterized rim / glow effects with minimal ALU (< ~15 ops).
- Consider feature flags to strip seldom-used foreground/background branches for release builds to reduce shader compile time.

## Removal Verification Checklist (All Met)
- No `BackgroundQuad` component definitions remain.
- No secondary metaballs shader asset exists.
- No mode cycling logic references PageUp/PageDown for a monolithic render mode (keys now mapped to foreground/background axes).
- No unused material asset types for metaballs remain.

## Historical Content (Redacted)
The original detailed step-by-step implementation spec has been intentionally omitted here to avoid accidental reintroduction. Refer to Git history if a forensic review is required.

## Conclusion
Maintain the unified approach. Any proposal to reintroduce multi-pass or multi-material duplication must present quantified benefits (profiling data, new visual features unattainable via branching).
