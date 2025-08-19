# Audit: Rendering Composition & Fluid Simulation Architecture

Date: 2025-08-19
Scope Version: Repository state on main at time of writing.

---
## 1. Purpose & Scope
This audit evaluates the current rendering & composition architecture with emphasis on the 2D fluid simulation system and its prospective coupling with the ball/physics system. Objectives:
- Assess architectural cleanliness, modularity, and extensibility.
- Identify blockers to introducing physics → fluid feedback (forces, dye, obstacles) and eventual fluid → physics influence (drag, flow advection).
- Recommend concrete, phased improvements with measurable acceptance criteria.

Out of scope: UI overlay styling, non-fluid gameplay logic, build pipeline.

---
## 2. Executive Summary
The fluid simulation works and is visually integrated, but internal organization is monolithic: a single, large compute driver performs sequential passes, recreates bind groups, and copies textures back to maintain a stable front buffer. There is no formal API for external force injection or obstacle representation, limiting future two‑way coupling. Logging verbosity, potential shader/Rust drift, and unnecessary per-pass texture copies add noise and overhead. A modest refactor introducing an impulse queue abstraction, a lightweight pass graph, and true ping‑pong resource management will unlock clean physics integration while reducing complexity.

Key Immediate Wins:
1. Introduce a `FluidImpulseQueue` resource + extraction (no shader changes initially).
2. Refactor compute into enumerated pass kinds (data-driven order) and remove redundant texture copy-backs.
3. Shrink velocity texture format & cache bind groups (reduce bandwidth + allocations).
4. Add tests asserting divergence reduction & uniform size invariants.
5. Extend shader to process multiple impulses, paving the way for ball wake effects.

---
## 3. Current State Overview
- Fluid simulation plugin handles allocation, extraction, compute dispatch, and display in one file.
- Compute sequence: add_force → advect_velocity → divergence → jacobi iterations → project → boundaries → advect_dye.
- After many passes, results are copied back to a front texture so the display material samples a consistent handle.
- Single force position & parameters in uniform; no multi-impulse support.
- No resource or event path for per-ball influence injection.
- Logging at `info!` inside compute loop (noisy in release/dev).
- Tests limited to configuration/struct size; no behavior/correctness tests.

Strengths:
- Clear startup initialization & resource allocation patterns.
- Separation of main world vs render world (extraction) already in place.
- Config-driven parameters enable user tuning without recompilation.
- Metaballs & background integrated cleanly as separate plugins (good model to emulate).

Limitations / Smells:
- Monolithic system function (hard to extend or selectively profile).
- Recreating bind groups each pass introduces overhead.
- Overuse of copy operations instead of pure ping-pong indexing.
- Velocity texture stores unused channels (RGBA vs RG).
- Lack of stable abstraction for external forces/dye/obstacles.
- Shader entry points & Rust pipeline references can drift (no validation tests).

---
## 4. Architectural Gaps Blocking Physics ↔ Fluid Coupling
| Gap | Impact | Needed Capability |
|-----|--------|-------------------|
| No impulse abstraction | Can't batch ball wake forces | Queue + GPU buffer for impulses |
| Single-force uniform | Unscalable for multiple balls | Storage buffer & count uniform |
| No obstacle mask | Balls can't act as boundaries | R8 texture mask & boundary-aware kernels |
| Copy-back pattern | Harder to insert new passes | True ping-pong with front index swap |
| Monolithic dispatch | Hard to conditionally add passes | Pass enum/graph iteration |
| No divergence residual metric | Hard to tune iterations | Reduction or sampled metric |
| No fluid→ball sampling | Can't apply fluid drag | Downsampled field or per-ball GPU sampling pass |

---
## 5. Detailed Findings
### 5.1 Compute Orchestration
A single driver in `RenderSet::Prepare` performs all pass dispatches. This inflates responsibility (scheduling, resource aliasing, logging, parameter translation). Splitting into: (a) pass graph construction, (b) pass execution, (c) diagnostics aggregation improves clarity.

### 5.2 Resource Management
Repeated texture copies: after each write, content is copied back to a fixed front texture. A classic ping-pong scheme (track (read, write) indices; swap at end) avoids bandwidth overhead and code duplication. For display, update the material's handle to the current "front" each frame.

### 5.3 Data Formats
Velocity stored with 4 channels wastes memory/bandwidth. Unless future features need extra channels (e.g., temperature, vorticity), prefer `Rg16Float`.

### 5.4 Extensibility for Forces
Current uniform supports exactly one force position and scalar parameters. Ball wakes, explosions, or drag injection require a variable-length list. A fixed-capacity storage buffer (e.g., 256 impulses) with overflow count is sufficient.

### 5.5 Obstacles & Boundaries
No ability to express dynamic obstacles (moving balls). Without an obstacle field, fluid passes treat space as empty. Introducing an occupancy / boundary normal mask enables no‑slip or partial slip conditions, improving realism and enabling fluid-driven drag.

### 5.6 Diagnostics & Testing
Absence of numerical tests means regressions in divergence correction or advection stability may slip in. Need minimal headless tests at tiny resolution verifying divergence reduction and deterministic evolution under fixed seeds.

### 5.7 Shader ↔ Host Parity
Risk of stale pipeline references; no automated assertion enumerating expected WGSL entry points vs pipeline setup.

### 5.8 Logging & Performance
Per-pass `info!` logs saturate output. Replace with `trace!` gated by feature or compile-time conditional; optionally add GPU timestamp queries for profiling when a debug feature is enabled.

### 5.9 Two-Way Coupling Pathway
Downstream (fluid→balls) requires sampling velocity field near ball positions. Approaches:
- GPU: Compute pass writes sampled velocities into a per-ball buffer; extract back next frame.
- CPU: Async map a downsampled velocity texture every N frames (1-frame latency acceptable) and sample bilinearly in a system.

---
## 6. Recommended Target Architecture
Components:
1. `FluidCore` (resource structs, settings, front/back indexing, status enum).
2. `FluidImpulses` (queue in main world + extracted GPU buffer in render world).
3. `FluidPassGraph` (ordered list of pass kinds derived from current config/state).
4. `FluidDriver` (executes graph each frame; manages bind group cache & swaps front/back indices).
5. `FluidDiagnostics` (timings, divergence residual, overflow counters, iteration stats).
6. `FluidObstacles` (optional R8 occupancy texture & builder system for ball geometry).

Data Flow (per frame):
Main World: collect impulses → extraction copies queue & optional ball metadata → Render World: update buffers → execute pass graph → update display material handle → (optional) sample fluid → write per-ball velocities → next frame main world consumes.

---
## 7. Proposed Data Structures (Sketch)
```rust
pub enum FluidPassKind { ApplyImpulses, AdvectVelocity, ComputeDivergence, Jacobi(u32), Project, EnforceBoundaries, AdvectDye }

pub struct FluidPassGraph { pub passes: SmallVec<[FluidPassKind; 8]> }

#[derive(Resource)]
pub struct FluidFrontBack { pub velocity_front: usize, pub dye_front: usize /* plus pressure if needed */ }

#[repr(C)]
pub struct GpuImpulse { pub pos: [f32;2], pub radius: f32, pub kind: u32, pub strength: f32, pub dir: [f32;2], pub _pad: [f32;2] }
```

---
## 8. Shader Additions (Concept)
Extend uniform with `impulse_count`, add storage buffer of impulses. Replace single-force logic within `add_force` pass (rename `apply_impulses`). For obstacles, add sampled mask check gating advection and boundary conditions.

---
## 9. Testing Strategy
| Test | Scenario | Assertion |
|------|----------|-----------|
| Uniform Layout | Build-time | Size & alignment stable |
| Divergence Reduction | 16x16, random initial velocity | post-projection divergence norm < pre |
| Dye Advection Stability | Inject dye pulse | Total dye mass within epsilon after N frames (with low dissipation) |
| Impulse Application | Single impulse center cell | Velocity magnitude increases locally, decays with radius |
| Impulse Overflow | >Max impulses queued | Overflow counter increments, processed count == max |
| Pass Graph Order | Graph builder | Contains expected sequence for current config |

---
## 10. Incremental Roadmap & Acceptance Criteria
### Phase 1: Hygiene & Baseline (Low Risk)
- Remove stale pipeline references (e.g., unused inject variants).
- Downgrade per-pass logging to `trace!` (feature `fluid_debug_passes`).
- Introduce `FluidSimStatus` resource with states: Disabled, WaitingPipelines, Running.
Acceptance: Build passes; debug overlay (if feature) shows status transitions; no functional change in visuals.

### Phase 2: Impulse Queue (CPU) (Low Risk)
- Add `FluidImpulseQueue` resource + system collecting placeholder impulses (e.g., from mouse or a single test ball).
- Extract queue into render world (just cloning; no shader usage yet).
Acceptance: Queue visible in debug overlay with count.

### Phase 3: Pass Graph Refactor & Ping-Pong
- Status: (PARTIAL) `FluidPass` enum + pass iteration in place; velocity & pressure ping-pong via `FluidPingState` (copies removed). Dye still copy-backed.
- NEXT: Eliminate dye copy by swapping display material's dye handle to current front each frame; unify front/back handling.
Acceptance (Why it matters):
	* Structural clarity (future passes insert with minimal churn).
	* Reduced GPU bandwidth (copy removal) without altering visuals.
	* Provides stable abstraction layer required before multi-impulse logic.

### Phase 4: Multi-Impulse & Dye Deposition (Pulled Forward)
- Implement GPU impulse storage buffer & count; replace single-force pass with loop (`apply_impulses`).
- Inject dye (constant or per-impulse color) so ball motion becomes visibly traceable.
- Clamp impulses to capacity; track overflow.
Acceptance (Visible Goal):
	* First tangible physics→fluid coupling (wakes/trails visible).
	* Demonstrates correctness of pass graph & ping-pong abstractions.
	* Establishes data path later reused for directional forces & drag.

### Phase 5: Bind Group Cache & Velocity Format Optimization (Moved Later)
- Switch velocity texture to `Rg16Float` (halve velocity bandwidth).
- Introduce bind group cache keyed by (vel_front_is_a, pres_front_is_a, pass_kind) to avoid per-pass creation.
Acceptance (Performance Focus):
	* Lower GPU memory & bandwidth for sustained scalability.
	* Reduced CPU overhead (bind group reuse) confirmed via lowered allocation counts in traces.

### Phase 6: Obstacles (Optional Path)
- Create obstacle mask texture; rasterize balls (compute or CPU -> upload) each frame.
- Modify divergence & projection passes to enforce boundary (zero normal component at obstacles).
Acceptance: Fluid flows around ball regions; divergence test passes with obstacles present.

### Phase 7: Fluid → Ball Drag
- Downsample velocity texture (dedicated compute pass) or sample per-ball on GPU; write results to buffer; extract next frame.
- Apply drag force system using sampled velocity data.
Acceptance: Balls exhibit directional drag aligning toward local flow; toggleable via config.

### Phase 8: Diagnostics & Metrics
- Add optional GPU timestamp queries (feature gated) to record per-pass duration.
- Add divergence residual estimate (sum |div|) after projection.
Acceptance: Overlay displays timings & residual; residual shrinks below threshold each frame.

### Phase 9: Extended Tests & Docs
- Implement full test matrix; add README section or extend `copilot-instructions.md` with new API usage.
Acceptance: All new tests pass in CI; documentation updated; developer can enqueue impulses with ≤5 lines of code.

---
## 11. Actionable Checklist (Expanded)

Legend: [x] done, [>] in progress/partial, [ ] pending, (opt) optional scope

### Phase 1: Clean Logging & Status (DONE)
	- [x] Add `FluidSimStatus` enum & resource
	- [x] Gate verbose logs behind `fluid_debug_passes` feature
	- [x] Replace noisy per-pass info logs with feature/trace macro
	- Acceptance: No visual change; status transitions observable

### Phase 2: Impulse Queue (CPU) (DONE)
	- [x] Define `FluidImpulse`, `FluidImpulseQueue`
	- [x] Collect placeholder per-ball (or simulated) impulses
	- [x] Extract queue to render world (no GPU usage yet)
	- Acceptance: Queue length visible in debug (future overlay) / logs

### Phase 3: Pass Graph & Ping-Pong (PARTIAL)
	- [x] Introduce `FluidPass` enum & iteration driver
	- [x] Add `FluidPingState` (velocity/pressure indices) & remove their copy-backs
	- [ ] Swap dye front by updating material handle each frame (remove dye copy)
	- [ ] Consolidate front/back handling into single helper (reduce duplication)
	- [ ] Adjust diagnostics to report copy savings (optional lightweight counter)
	- Acceptance: Zero functional differences; GPU copies reduced; architecture ready for multi-impulse

### Phase 4: Multi-Impulse GPU Application + Dye Deposition (VISIBLE FEATURE)
	- [ ] Define `GpuImpulse` struct & max count constant
	- [ ] Add storage buffer + count uniform (or pack count in existing uniform padding)
	- [ ] Extend extraction: pack impulses -> mapped buffer write each frame
	- [ ] Update bind group layout with impulse storage binding
	- [ ] WGSL: Replace `add_force` with `apply_impulses` looping impulses
	- [ ] Implement radial velocity injection (falloff: (1 - r/R)^n)
	- [ ] Inject dye (constant color or simple per-impulse hue)
	- [ ] Clamp & track overflow (warn if overflowed)
	- [ ] Add unit test: `GpuImpulse` size/alignment stable
	- Acceptance: Balls create visible wakes/trails; disabling impulses removes effect; no crashes with 0 impulses

### Phase 5: Bind Group Cache & Velocity Format Shrink
	- [ ] Convert velocity textures to `Rg16Float`
	- [ ] Update WGSL texture declarations & sampling
	- [ ] Implement small bind group cache (HashMap or fixed array keyed by pass + front bits)
	- [ ] Remove per-pass bind group recreation path
	- [ ] Add test ensuring SimUniform unaffected & pipeline count same
	- Acceptance: Reduced VRAM & CPU allocations (informal log / perf snapshot)

### Phase 6: Obstacle Mask (opt)
	- [ ] Allocate R8 obstacle texture
	- [ ] Rasterize balls each frame (CPU upload or compute)
	- [ ] Modify divergence & projection to treat obstacles as solid (zero normal velocity)
	- Acceptance: Flow diverts around ball regions (visual inspection)

### Phase 7: Fluid → Ball Drag
	- [ ] Downsample velocity field (compute) or create sampling pass
	- [ ] Sample per-ball velocities → buffer → extract to main world
	- [ ] Apply drag system (configurable coefficient)
	- Acceptance: Balls slow when moving against flow; toggle off reverts behavior

### Phase 8: Diagnostics & Residuals
	- [ ] Optional GPU timestamp queries (feature gated)
	- [ ] Compute divergence residual after projection (L1 or L2 metric)
	- [ ] Overlay: display workgroups, residual, impulse count, overflow
	- Acceptance: Residual decreases frame-over-frame; metrics visible when enabled

### Phase 9: Tests & Docs Finalization
	- [ ] Divergence reduction test (tiny grid)
	- [ ] Dye mass conservation (low dissipation scenario)
	- [ ] Impulse application locality test
	- [ ] Pass graph order test (config variant)
	- [ ] README / instructions update for new API (impulse enqueue snippet)
	- Acceptance: All tests green; documented usage path < 5 lines for adding an impulse

Each phase should leave the system buildable, visually stable (except intended new effects), and documented.

---
## 12. Risk & Mitigation Snapshot
| Risk | Mitigation |
|------|------------|
| Shader/host mismatch | Add unit test enumerating entry points & checking uniform sizes |
| Performance regression post-refactor | A/B frame time sampling before/after each phase |
| Complexity creep in pass graph | Keep enum simple; avoid DAG until needed |
| GPU buffer overflow for impulses | Clamp count; overflow counter & periodic warning |
| Async mapping stalls (drag sampling) | Use downsampled texture & double-buffer result |

---
## 13. Future Enhancements (Post Roadmap)
- Vorticity confinement pass for higher visual energy.
- Dye blur/bloom compute pass for stylistic trails.
- Configurable viscosity (diffusion) & pressure iteration auto-tuning (stop when residual < epsilon).
- Multi-resolution (coarse grid for pressure solve, fine for dye) to trade quality vs perf.

---
## 14. Summary
The current fluid system provides a solid baseline but centralizes too many responsibilities and lacks a formal extension surface. A measured sequence of low-to-medium risk refactors will: (1) establish a clean API for external influences, (2) reduce redundant GPU work, and (3) enable two-way coupling with ball physics. Executing the outlined roadmap yields a lean, testable, and extensible simulation core ready for richer gameplay interactions.

---
*End of Audit*
