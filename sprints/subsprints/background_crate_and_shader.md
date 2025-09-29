# Sub‑Sprint: Background Crate & Shader

## Goal

Introduce a dedicated background rendering subsystem (its own crate) that renders into the `Background` compositing layer before GameWorld content, enabling atmospheric gradients and simple procedural visuals now, and parallax / richer effects later.

## Current Context

* Layer stack (see `game_rendering` crate) already reserves a Background layer; currently it just clears to a flat color.
* Compositor material has a slot for background texture; unused / empty most of the time.
* Future visuals (parallax stars, ambiance) will need a clean, isolated pipeline.

## In Scope (This Sprint)

* New crate: `background_renderer`.
* Basic GPU shader producing:
  * Mode 0: Solid Color.
  * Mode 1: Vertical Gradient.
  * Mode 2: Radial Gradient.
  * Mode 3: Animated Gradient (time‑varying blend / subtle hue shift).
* Uniform resource & config API exposed to gameplay code (`BackgroundConfig`).
* Integration with `GameRenderingPlugin` so compositor samples populated background texture.
* Demonstration in `compositor_test` (and later `physics_playground`).

## Out of Scope

* Parallax, starfields, particles, Perlin noise (can be a follow‑up; keep shader minimal now).
* Day/night cycle, HDR exposure adaptation.
* Asset / image file backgrounds.

## Architecture & Design

Crate layout draft:

```text
crates/background_renderer/
  Cargo.toml
  src/
    lib.rs                # Plugin + public exports
    material.rs           # BackgroundMaterial definition (Material2d)
    config.rs             # BackgroundMode / BackgroundConfig resources
    systems.rs            # Time accumulation + dynamic updates
    shader/background.wgsl
```

### Data Model

```rust
pub enum BackgroundMode { Solid, VerticalGradient, RadialGradient, AnimatedGradient }
pub struct BackgroundConfig {
    pub mode: BackgroundMode,
    pub color_a: LinearRgba,
    pub color_b: LinearRgba,
    pub animation_speed: f32, // only used when animated
}
```

`BackgroundMaterial` uniform mirrors these (packed / aligned for WGSL). Time value accumulated per frame when Animated.

### Rendering Strategy

* Use `Material2d` with a persistent full‑screen quad (simplest). Camera or orthographic transform ensures coverage.
* Quad is rendered into the Background layer render target that compositor later samples.
* On window resize, either rely on camera scaling or (if explicit vertex positions used) update quad size.

### Integration Points

* `GameRenderingPlugin` adds `BackgroundRendererPlugin` early so the background texture exists when compositor runs.
* Feature flag optional: `background_renderer` (decide if always on; keep default on for visibility).

## Tasks

1. Create crate skeleton & add to workspace members.
2. Define `BackgroundMode` + `BackgroundConfig` with sensible `Default` (vertical gradient dusk colors).
3. Implement `BackgroundMaterial` (fields: `mode: u32`, `color_a: Vec4`, `color_b: Vec4`, `time: f32`).
4. Write `background.wgsl` implementing all four modes (branch on `mode`).
5. System: accumulate `time += dt * animation_speed` when Animated.
6. Startup system: spawn quad with `BackgroundMaterial`; tag with proper `RenderLayers` / or whatever layer mapping helper is used.
7. Ensure plugin ordering before compositor finalization (verify with log trace ordering).
8. Add demo usage in `demos/compositor_test` (insert `BackgroundConfig` with radial or animated gradient).
9. Optionally add key binding (e.g., `B`) in demo to cycle background modes.
10. Logging / diagnostics: trace on mode change; (optional) simple frame counter metric.
11. Crate README + short blurb update in `north-star-structure.md` explaining background layer purpose.

## Acceptance Criteria

* Compositor final frame displays gradient (not flat black) when plugin active.
* Switching `BackgroundConfig.mode` at runtime updates on next frame (visual confirmation).
* Window resize produces correctly scaled background without artifacts or panic.
* Works side‑by‑side with all other layers (no ordering issues, no overdraw problems diagnosed by obvious artifacts).
* No significant frame time regression (>0.2 ms in an otherwise idle scene) attributable to background.

## Edge Cases

* Minimized / zero-sized window: no panic (Bevy protects; ensure no custom unwraps on size).
* Rapid mode switching (multiple times per frame): stable; last write wins.
* Extremely high animation speed: shader remains numerically stable (time wrap optional: modulo large constant).

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Uniform layout mismatch | Use `#[repr(C)]` + `bevy::render::render_resource::AsBindGroup` derive to enforce layout. |
| Over-engineering early (noise, parallax) | Deliberately defer advanced effects, keep shader <100 LOC. |
| Ordering errors (background draws after compositor) | Explicit plugin ordering + log assert layer writes before composite pass. |

## Definition of Done

All tasks complete, acceptance criteria met, background visible in demo(s), documentation added, no new warnings introduced.

## Follow‑Ups (Not in This Sprint)

* Procedural noise / Perlin layer (original note) — could be mode 4 later.
* Parallax multi‑layer backgrounds.
* Day/night / color grading integration.
* Config UI to tweak colors live.
