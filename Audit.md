# Audit: Bridging Ball Simulation into Fluid Simulation via MRT Textures (No Storage Buffers)

Date: 2025-08-20
Target Branch: main
Author: Automated planning (Copilot)
Scope: Replace CPU impulse + storage buffer injection path with deterministic texture-driven coupling from balls to fluid sim using only render targets (compatible with Web/WebGL2 fallback constraints where storage buffers / compute may be limited). One‑way coupling (balls -> fluid only). No new external dependencies.

## 1. Objectives
- Source all velocity + dye disturbances for the fluid simulation from per-frame render-generated field textures derived from ball state.
- Eliminate `FluidImpulseQueue`, GPU impulse storage buffers, and related config fields (impulse_*).
- Introduce two accumulation render targets ("ball field MRT"):
  1. VelocityAccum (RGBA16F): encodes accumulated weighted velocity and weight.
  2. ColorAccum (RGBA8UnormSrgb default, optional RGBA16F): encodes accumulated weighted color and weight.
- Provide a new compute (or combined) ingestion pass that samples these textures before existing fluid steps, injecting velocity and dye—fully replacing `apply_impulses` logic (and possibly merging with it).
- Maintain Web compatibility (no reliance on storage buffers for ingestion). All data flows via sampled / storage textures already supported in baseline WebGPU; degrade gracefully for WebGL2 (where compute may not exist) by keeping current gating (future consideration—out of current scope, but design should not preclude it).

## 2. Current State Summary
- Fluid sim (`fluid_sim.rs`) uses compute passes with a bind group layout consisting entirely of storage textures + uniform + (now) storage buffer impulses (bindings 8 & 9). `apply_impulses` loops over a storage buffer of impulses.
- Ball wake impulses are produced CPU-side in `fluid_impulses.rs` by iterating balls, computing a radius & strength heuristic, pushing into `FluidImpulseQueue`, extracting, packing into a storage buffer.
- Dye deposition also occurs in `advect_dye` via the same impulse list.
- No existing path renders balls into auxiliary offscreen textures.

## 3. Gap Analysis
| Aspect | Current | Target | Delta |
|--------|---------|--------|-------|
| Impulse representation | CPU queue -> storage buffer | Implicit encoded fields in textures | Remove queue + buffers; add MRT pass |
| Data transport | Storage buffer iteration (compute) | Sampled 2D textures | Extend pipeline layout or add separate pipeline |
| Dependency ordering | Simple pass list | Needs ordering: Ball MRT draw -> ingestion compute -> rest | Add render graph node & pass insertion |
| Config controls | Many impulse_* fields | Fewer: injection_scale, dye_strength, maybe precision toggle | Remove unused fields, add new ones |
| Web constraints | Already using storage textures; storage buffer for impulses | No storage buffers | Remove storage buffer bindings entirely |

## 4. Constraints & References
- Web limits (Docs.rs `WgpuLimits`): WebGL2 fallback disallows storage buffers & compute; WebGPU default supports needed sampled + storage textures. We will strictly avoid requiring storage buffers for coupling (refs: docs.rs `downlevel_webgl2_defaults`, WebGPU article on bevy.org).
- Max color attachments default: 8 (ample for dual MRT).
- Need additive blending on float formats; ensure chosen formats are renderable + blendable across platforms (RGBA16Float typically is; verify at runtime and fallback to RGBA32Float if necessary on native; for web maintain simple path and document possible precision downgrade to RGBA8 if float16 blending unsupported).

## 5. High-Level Design
### 5.1 New Resources
- `BallFieldResources` (resource):
  - `velocity_accum: Handle<Image>` (RGBA16Float, usage: RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC)
  - `color_accum: Handle<Image>` (RGBA8UnormSrgb or RGBA16Float)
  - Option: sampler handle if required distinct filtering (nearest).
- Resolution: identical to fluid grid (`FluidSimSettings.resolution`). Reallocate on fluid resolution change (system similar to existing reallocation for fluid textures).

### 5.2 Ball Field Render Pass (MRT)
- Implement `BallFieldRenderPlugin`.
- Create an offscreen camera (or custom render graph node) rendering only ball visualization primitives to the two attachments. Options:
  - (Preferred) Custom render graph node: Bypasses overhead of extra main camera, affords explicit ordering.
  - Use `RenderLayers` to ensure only balls contribute to MRT.
- Per-ball draw: Either reuse circle mesh scaled to diameter, or switch to a quad + fragment distance field to reduce vertex cost. Start with existing circle mesh for simplicity.
- Fragment Shader Responsibilities:
  - Compute normalized distance d/r for pixel inside ball; produce falloff weight w = (1 - (d/r)^2)^2 (C2 smooth, near-compact).
  - Fetch ball world velocity (from uniform or per-instance attribute). Since current draw path doesn't supply velocity, we may introduce a lightweight CPU-built instance buffer OR encode velocity via a second pass (simpler initial approach: CPU updates a per-entity component used by a custom extraction stage adding per-instance data). Minimal approach: store velocity in a texture-less vertex attribute via instancing (requires custom pipeline). Simpler interim: sample velocity from a uniform array limited by max balls; reuse existing metaballs uniform method if acceptable. For planning: choose instanced buffer with position, radius, velocity, color to avoid uniform bloat.
  - Convert world velocity to grid velocity: `vel_grid = vec2(vx * grid_w / window_width, vy * grid_h / window_height)`.
  - Outputs:
    - RT0 (VelocityAccum RGBA16F): (vel_grid * w, w, 0)
    - RT1 (ColorAccum RGBA8 / 16F): (color.rgb * w, w)
- Blending (additive / pre-multiplied):
  - VelocityAccum: Color blend add (srcFactor=One, dstFactor=One) for RGB & w channel.
  - ColorAccum: Add for RGB & Alpha.
- Clear each frame to zero.

### 5.3 Ingestion Compute Pass
- New compute shader entry point `ingest_ball_fields` runs before existing `advect_velocity` (and replaces `apply_impulses`).
- Inputs (sampled): velocity_accum, color_accum, plus velocity_in/out, dye_in/out storage textures.
- For each cell (gid):
  - Read accum texels at integer coords (no filtering). Let `acc_v = (vx_sum, vy_sum, w)`; if w > eps:
    - `vel_add = (vx_sum, vy_sum)/w * injection_scale;`
    - `new_vel = read(velocity_in) + vel_add`.
    - For dye: read color accum `(cr_sum, cg_sum, cb_sum, w_c)`; average color = sum / max(w_c, eps); deposit: `dye_out = clamp(dye_in.rgb + avg_color * dye_strength * w_c, 0..1)`.
  - Else: copy velocity_in to velocity_out & dye_in to dye_out.
- Set ping front states analogous to existing apply_impulses; subsequent passes work unchanged.
- Remove old `apply_impulses` from pass graph.

### 5.4 Pipeline & Layout Changes
Option A (Separate Layout):
- Add second bind group layout for ingestion: sampled textures + existing storage targets.
Option B (Extend Existing Layout):
- Append bindings 10..12: `@binding(10) velocity_accum_tex : texture_2d<f32>; @binding(11) color_accum_tex : texture_2d<f32>; @binding(12) nearest_sampler : sampler;`
- Pros: fewer bind groups; Cons: need to recreate pipelines / break existing layout stability.
Decision: Use separate ingestion pipeline with its own layout to avoid touching stable existing pipelines (minimizes regression risk). Only new pipeline + layout addition.

### 5.5 Coordinate Mapping
- Offscreen camera set so world space aligns with fluid grid center points as closely as practicable: use orthographic projection covering window dimensions; ingestion pass samples by integer pixel so minor sub-pixel differences acceptable.
- UV formula in compute: `uv = (vec2(gid) + 0.5) / vec2(grid_size)`.

### 5.6 Config Additions / Removals
Remove (or deprecate) in `GameConfig.fluid_sim`:
- `impulse_min_speed_factor`, `impulse_radius_scale`, `impulse_strength_scale`, `impulse_radius_world_min`, `impulse_radius_world_max`, `impulse_debug_strength_mul`, `impulse_debug_test_enabled`, `force_strength` (if solely used for impulses; verify gravity usage elsewhere). Keep generic `dissipation`, etc.
Add:
- `ball_field`: `{ injection_scale: f32, dye_strength: f32, color_high_precision: bool }`.

### 5.7 Debug & Diagnostics
- Add counters: average weight coverage (% cells with w>0), max w, injection energy (sum |vel_add|).
- Overlay display updates.

## 6. Data Encoding & Formats
| Target | Format | Channels | Rationale |
|--------|--------|----------|-----------|
| VelocityAccum | RGBA16Float | (vx_sum, vy_sum, weight, reserved) | High precision accumulation, moderate bandwidth, blendable |
| ColorAccum | RGBA8UnormSrgb (default) or RGBA16Float (optional) | (premul_color_sum, weight) | sRGB-friendly output / lighter memory; upgrade path for precision |

Weight normalization done in compute. Clamp weight to >= eps (1e-5) to avoid division by zero.

## 7. Pass Ordering (Per Frame)
1. Ball Physics (Rapier / movement).
2. BallField MRT Render (draw balls into accumulation targets; cleared beforehand).
3. Fluid Compute (Render world Prepare):
   - `ingest_ball_fields` (samples accum -> writes new velocity/dye front/back).
   - `advect_velocity` ... existing sequence (minus old `apply_impulses`).
4. Display (fullscreen dye quad / metaballs, etc.).

## 8. Migration Plan (Phased)
### Phase 0: Prep
- Document design (this audit). Add stub config section for `ball_field` (no usage yet).

### Phase 1: Resource Allocation
- Create `BallFieldResources` with two images sized to fluid resolution.
- Reallocate on resolution change.

### Phase 2: Render Pass Scaffolding
- Add `BallFieldRenderPlugin` registering a custom node (or offscreen camera) that clears attachments only (no drawing). Validate lifetime & ordering (log once). Add minimal test verifying creation.

### Phase 3: Ball Rendering + MRT Encoding
- Implement shader + pipeline specialized for instanced circles.
- Add per-instance buffer (position, radius, velocity, color, maybe packed into Vec4s) extracted each frame.
- Enable additive blending.
- Visual debug path (optional) to display accumulation textures on key toggle.

### Phase 4: Ingestion Compute
- Add ingestion pipeline + WGSL (new file or extend existing fluid shader with feature guard).
- Insert ingestion in pass graph (replace `apply_impulses`).
- Validate on native (print injection stats; ensure stable dye motion).

### Phase 5: Decommission Old Impulse Path
- Remove `fluid_impulses.rs` plugin registration & related storage buffers + bindings 8/9 in layout.
- Prune config fields; migration handling: treat missing old fields gracefully.

### Phase 6: Tuning & Config Exposure
- Wire `injection_scale`, `dye_strength`, precision toggle.
- Add overlay metrics (coverage, energy).

### Phase 7: Web Testing
- Build wasm target w/ webgpu; confirm no storage buffer binding references; fallback detection if webgl2 backend active -> skip ingestion gracefully (document limitation).

### Phase 8: Optimization & Polish
- Optional: unify color + velocity accumulation into single RGBA16F (vx, vy, dye_luma_or_weight, weight) + second texture for color refinement if needed.
- Optional downsampled accumulation (super-sample then shrink) for smoother influence.

## 9. Risk Matrix & Mitigations
| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Float16 blend unsupported on some web GPUs | Accum artifacts | Medium | Detect format support; fallback to RGBA32F (native) or RGBA8 path |
| Overdraw cost with many large balls | Perf drop | Medium | Limit influence radius, use smaller quad + analytic falloff, frustum culling |
| Weight saturation (bright dye wash) | Visual quality | Medium | Cap per-pixel weight; apply tone mapping or normalize before deposition |
| Ordering bug (ingestion runs before MRT draw completes) | Wrong injections | Low | Explicit graph edge: ingestion depends_on MRT node |
| Memory overhead (extra 2 large textures) | VRAM pressure | Low | Allow lower resolution override for ball field separate from fluid grid |
| Precision loss in RGBA8 color accumulation | Subtle banding | Medium | Config toggle for RGBA16F color path |
| Removal of impulse config breaks existing RON files | User friction | High | Provide transitional loader accepting both; deprecate old fields with log warnings |

## 10. Acceptance Criteria
- No references to `FluidImpulseQueue`, `GpuImpulse`, or impulse storage buffers in final pipeline.
- Fluid dye & velocity respond to ball movement (stopping balls stops new injection; existing dye continues advecting).
- Two new textures allocated & visible in diagnostic logs at expected resolution.
- `ingest_ball_fields` executes exactly once per frame when background=FluidSim.
- Web (wasm32) build runs without storage buffer feature usage (verify by log / inspector).
- Config toggles modify injection magnitude in real time (after hot reload if implemented later).

## 11. Open Questions
- Should dye deposition happen pre-advection (current plan) or post-advection for more trailing effect? (Initial: pre.)
- Combine ingestion with boundary enforcement to reduce passes? (Probably not needed yet.)
- Need cluster color coherence? Currently ball color direct; could average cluster.
- Provide temporal smoothing to reduce flicker for fast small balls? Potential future enhancement.

## 12. Test Strategy
- Unit: Ensure new resource allocation matches FluidSimSettings resolution; pass ordering test verifying ingestion inserted.
- Integration (headless): Step simulation with a single moving ball; assert non-zero changes in sampled center velocity/dye after a few frames (using read-back on native only behind test feature).
- Performance baseline: Count workgroups & draw calls added (diagnostic print) vs baseline (< +1 draw pass +1 compute pass).

## 13. Implementation Notes
- Favor separate ingestion pipeline to avoid re-layout cost.
- Keep fluid WGSL modular: new section `// ingestion` guarded by `#ifdef`-style shader defs (Bevy shader defs) for easier iteration.
- Use nearest sampling (no filtering) for accumulation read to keep energy localized.
- If multiple frames should accumulate lingering influence, either (a) do not clear MRT each frame and use decay factor in ingestion, or (b) clear and rely on ball presence only (initial approach: clear for determinism).

## 14. De-scoping / Future Work
- Reverse coupling (fluid -> ball forces) explicitly out of scope.
- Multi-resolution or tile-based culling for huge ball counts deferred.
- GPU-driven instancing / indirect draws deferred.

## 15. Summary
This plan migrates from a CPU impulse buffering model to a purely texture-driven field ingestion that aligns with web constraints and simplifies the injection path. It leverages additive MRT rendering for natural accumulation, reduces CPU-GPU synchronization complexity, and paves the way for further visual refinement (e.g., temporal smoothing, multi-resolution injection) without reintroducing storage buffer dependencies.

---
Prepared automatically; review & adjust before implementation.
