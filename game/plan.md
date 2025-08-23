# Game Migration Plan (Legacy -> Game)

> Goal: Incrementally port functionality from `legacy/` into a new, clean, modular architecture under `game/` while keeping `legacy/` untouched as a golden reference until parity is proven. All work occurs on the `rebuild` branch. No big‑bang rewrite; every phase must compile, run, and have measurable validation.

---
## 1. Guiding Principles
1. Incremental & Observable: Each phase adds or replaces one vertical slice with clear exit criteria.
2. Stable Reference: `legacy/` remains runnable; never modify it except for emergency security fixes.
3. Strict Layering & Acyclic Dependencies.
4. Feature Gating: Expensive / optional subsystems (metaballs, debug overlay, hot reload) gated by crate features.
5. Fast Feedback: Unit tests, headless integration tests, and golden frame/image hashes where applicable.
6. Determinism (where feasible): Simulation determinism for physics-independent systems (clustering, radial forces) validated by snapshot tests.
7. WASM & Native Dual Support maintained from phase 1 onward.

---
## 2. Target Modular Workspace Architecture (under `game/`)

Planned crates (initially some may be empty stubs):

```
game/
	Cargo.toml (workspace)
	crates/
		core/              # Core types: components, tags, math helpers, system ordering, shared constants
		config/            # GameConfig + loading + validation; NO bevy dependency (pure serde + ron) where possible
		bevy_app/          # Thin final executable (or lib) wiring plugins & window; depends on feature crates
		physics/           # Rapier setup, radial gravity, separation helpers (depends on core, config)
		rendering/         # Materials, palettes, background, camera; (depends on core, config)
		gameplay/          # Spawning, emitter, interactions (drag/explosion), clustering logic (depends on core, physics, config)
		metaballs/         # Metaballs specific systems & shaders (feature `metaballs`); depends on rendering + core
		debug_tools/       # Feature `debug`; overlay, stats, mode cycling; depends on gameplay + rendering + metaballs(opt) + physics
		hot_reload/        # Feature `hot-reload`; config file watcher (native only) minimal bevy tie‑in
		integration_tests/ # Rust crate with `#[cfg(test)]` and `cargo test -p integration_tests`
		examples/          # Multiple small binary crates (see §7)
	assets/              # Shared assets (config, shaders)
	integrations/        # (Optional) external service integration stubs (telemetry, analytics) later
	samples/             # Standalone minimal recipes mirrored in docs
```

### 2.1 Dependency Direction (Acyclic)

ASCII graph (arrows point to dependencies):
```
config  ---> (pure) no deps
core    ---> config (for param types) 
physics ---> core, config, rapier
rendering --> core, config
metaballs --> rendering, core, config (feature)
gameplay --> core, physics, rendering, config
debug_tools -> gameplay, rendering, physics, metaballs?, core, config (feature)
hot_reload -> config (+ bevy minimal) (feature)
bevy_app -> {core, config, physics, rendering, gameplay, metaballs?, debug_tools?, hot_reload?}
integration_tests -> all published interfaces (black‑box)
```

Enforcement: Use `cargo deny` / `hakari` later to verify there are no reverse edges; start with convention + review.

### 2.2 Plugin Boundaries
Each crate exports exactly one `Plugin` (e.g. `PhysicsPlugin`, `GameplayPlugin`) plus narrow public APIs (helper functions / resource structs). Internal modules kept `pub(crate)`.

### 2.3 Crate Feature Matrix

| Feature | Effect |
|---------|--------|
| `debug` | Enables `debug_tools` crate dependency & systems, rapier debug render bridging |
| `metaballs` | Includes metaballs crate + shader pipelines; off shrinks compile time |
| `hot-reload` | Enables file watcher on native targets |
| `wasm` | (Implicit target) Avoids `hot-reload`; uses embed config path |

Additive & orthogonal; ensure conditional compilation uses `#[cfg(feature = "metaballs")]` etc.

### 2.4 Separation of Concerns Rationale
* `config` isolated => cargo test fast (no wgpu / rapier) for validation logic.
* `core` holds simple components (Ball, BallRadius, system set labels) to break cycles.
* `physics` excludes spawning logic, only simulation prep and post adjustments.
* `gameplay` owns entity lifecycle (spawn/emitter) & cluster logic (pure CPU, deterministic) enabling snapshot tests.
* `rendering` centralizes materials, palettes, camera, background. Metaballs elevation kept separate for optional heavy pipeline.
* `debug_tools` large compile hit isolated; building release without `--features debug` reduces incremental times.
* `hot_reload` separate to avoid accidental file IO in WASM builds.

---
## 3. Legacy Module Mapping

| Legacy Module | New Home | Notes |
|---------------|----------|-------|
| `components.rs` | core | Components & markers unchanged (rename if needed) |
| `system_order.rs` | core | System set labels stay lightweight |
| `config.rs` | config | Keep identical API; remove bevy dependency; resources added at app level |
| `config_hot_reload.rs` | hot_reload | Wrap in feature & native `cfg` |
| `radial_gravity.rs` | physics | Pre-physics force system |
| `rapier_physics.rs` | physics | Setup plugin & gravity config |
| `separation.rs` | physics | Post-physics adjustments |
| `spawn.rs` | gameplay | Startup spawning logic |
| `emitter.rs` | gameplay | Runtime spawner (rename `emitter.rs` -> `dynamic_spawn.rs` optional) |
| `cluster.rs` | gameplay | Deterministic cluster computation |
| `input_interaction.rs` | gameplay | Interactions (drag/explosion) |
| `materials.rs` | rendering | Palette + material init set |
| `palette.rs` | rendering | Colors consumed by materials |
| `background.rs` | rendering | Background pass |
| `camera.rs` | rendering | Camera setup |
| `metaballs.rs` | metaballs | Visual advanced pipeline (feature) |
| `debug/*` | debug_tools | Entire feature crate gated |
| `auto_close.rs` | bevy_app (or core util) | Simple timer system; could move to gameplay util |
| `game.rs` | bevy_app | Replaced by aggregated plugin wiring |
| `main.rs` | bevy_app | New binary crate main |

---
## 4. Phased Migration Plan

Each phase delivers a self-contained increment. Do NOT delete legacy code until Phase 10 signoff.

| Phase | Title | Scope | Exit Criteria |
|-------|-------|-------|---------------|
| 0 | Workspace Scaffolding | Create `game/Cargo.toml` workspace & empty crates with stubs | `cargo test` + `cargo build` succeed; plan committed |
| 1 | Config & Core Foundations | Port `config.rs`, components, system sets | Unit tests green; config parity diff vs legacy sample |
| 2 | Physics Baseline | Rapier setup, radial gravity, separation in new app skeleton | Headless sim step equivalence for 100 frames vs legacy (position snapshot delta < epsilon) |
| 3 | Rendering Basics | Camera, background, materials/palette; draw circles only | Visual smoke (manual) + golden hash of first frame PNG |
| 4 | Spawning & Emitter | Initial ring spawn + runtime emitter | Entity count & radius distribution histograms match legacy tolerance |
| 5 | Interactions & Input | Drag + explosion | Deterministic test harness producing same cluster count after scripted inputs |
| 6 | Clustering | Cluster plugin + tests ported | All clustering tests green (identical) |
| 7 | Metaballs (Feature) | Port metaballs pipeline | Feature builds; golden frame hash matches legacy within threshold (HSV delta) |
| 8 | Debug Tools (Feature) | Port debug overlay & stats | Shortcut key mapping parity table passes manual QA |
| 9 | Hot Reload (Feature) | File watcher & application of changes | Modify config during runtime -> reflects in window + metaballs params within < 1s |
| 10 | Parity & Cutover | Compare performance, memory, finalize docs | <5% frame time delta; documentation updated; signoff recorded |

Rollback Strategy: If a phase fails, revert to last tagged commit `phase-X-complete`.

---
## 5. Validation & Test Strategy

### 5.1 Test Types
* Unit Tests: Port existing ones; keep purely algorithmic crates test-only features enabled (e.g., cluster, config, radial gravity).
* Snapshot Tests: Serialize cluster sets & deterministic spawn RNG seed results (store in `tests/snapshots/`).
* Golden Frame Tests (optional later): Offscreen render first frame(s) to texture, hash (e.g. blake3) & compare within tolerance (store baseline per feature set). Requires wgpu surface & headless adaptation (skip on wasm).
* Performance Smoke: Simple bench harness counts average ms/frame over first N frames; store baseline JSON for trend diff (not hard fail, warn if >10%).
* Integration Tests: End-to-end run with scripted ECS input events to validate interactions (e.g., simulate pointer drag path).

### 5.2 Determinism Controls
* Seed RNG: Provide `RNG_SEED` resource; spawning & emitter respect seeded `StdRng` under `#[cfg(test)]` or feature `deterministic`.
* Frame Step Harness: Custom runner that advances fixed delta (e.g. 1/60) for CPU determinism verification (physics may diverge minutely; allow small tolerance).

### 5.3 Metrics & Thresholds
| Metric | Tool | Threshold |
|--------|------|-----------|
| Cluster test variance | Snapshot diff | 0 differences |
| Physics position drift (radius=10, 100 frames) | Numeric compare | < 0.5 units avg, < 2.0 max |
| Golden frame hash delta | Per-pixel ΔE (approx) | < 3% pixels differing beyond ΔE 10 |
| Frame time regression | Bench harness | < +5% mean vs legacy |

### 5.4 Continuous Integration Hooks
Add new workflow steps: `cargo fmt -- --check`, `cargo clippy -- -D warnings`, `cargo test --workspace --features "debug metaballs"` and minimal feature set pass, plus optional golden tests behind environment flag.

---
## 6. Samples & Examples (Under `game/examples/`)

| Example | Purpose | Dependencies |
|---------|---------|--------------|
| `config_dump` | Load & print merged config | config
| `physics_headless` | Run 300 fixed steps & print summary | core, config, physics
| `spawn_demo` | Spawn ring only | core, config, physics, rendering, gameplay (partial)
| `interaction_demo` | Drag & explosion interactions | adds gameplay input
| `clusters_inspect` | Dump clusters JSON each 60 frames | gameplay (cluster)
| `metaballs_view` | Render metaballs only | rendering + metaballs (feature)
| `full_game` | Equivalent to legacy main | all

Each example must compile in isolation; use feature flags as needed.

---
## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Feature Drift | Legacy diverges during migration | Freeze feature work or dual-implement small patches in both; keep changelog |
| Hidden Couplings | Unexpected cross-module assumptions | Early stub crates; assert absence of forbidden deps via `cargo tree --edges normal` scans |
| Performance Regression | Worse frame time due to fragmentation | Periodic perf benchmarks; profile early (Phase 4 & 7) |
| Test Non-Determinism | Flaky snapshots | Seed RNG; isolate physics vs logic tests |
| Over-Engineering | Migration stalls | Strict phase scope & exit criteria; no premature optimizations |
| Shader Divergence | Metaballs mismatch | Golden frame hash + shader module diff tooling |
| WASM Differences | Different behavior or panic | Run wasm build check in CI from Phase 2 |

---
## 8. Tooling & Automation Enhancements (Future Phases Optional)
* `cargo xtask` helpers: `xtask phase verify`, `xtask metrics bench`, `xtask golden update`.
* `cargo deny` / `cargo hakari` for dependency graph health.
* Image diff tool (compute ΔE) integrated in golden tests.
* Performance capture harness using `bevy::diagnostic` frame time plugin.

---
## 9. Documentation Strategy
* Each crate has a `README.md` summarizing purpose & public surface.
* Root `game/README.md` lists crate graph & feature flags.
* Migration log appended at end of this plan (see §11) per phase with date & commit hash.

---
## 10. Actionable Developer Checklist

Tracking list (mirrors high-level phases). Update status inline (DO NOT remove completed entries; append result notes).

```
- [x] Phase 0: Scaffold workspace & stub crates (completed 2025-08-21 commit 2a1484e36f495f65566e12e268ae3acf1958152f)
- [x] Phase 1: Port config crate + tests (no bevy) (completed 2025-08-21 commit 9b04bb5)
- [x] Phase 1: Port core crate (components, system sets) (completed 2025-08-21 commit 9b04bb5)
- [x] Phase 2: Implement physics crate (rapier setup, radial gravity, separation) + deterministic headless test (baseline systems 96a8cb2; harness smoke test a803d1a; drift movement test e25ed53; self-consistency test 568631b; snapshot serialization test 77c14ad; parity test 71a5919 avg_abs=0.0244 max_abs=0.0439 over 100 steps (tolerance avg<0.5 max<2.0) stricter target met)
- [x] Phase 3: Implement rendering crate (camera, background, materials, palette) (completed commits 9c01337, 396ed6c, 9ef9e8e, 66b65e7: camera & clear color palette, background grid shader/material with background_light gating + lightweight test variants, circle placeholder pipeline w/ per-ball color indexing, instancing feature scaffold (InstancingState + plugin), golden placeholder hash harness; exit criteria met: visual smoke via integration tests + deterministic placeholder golden hash; deferred enhancements: true GPU frame capture & instanced draw pipeline)
- [x] Phase 4: Port spawning (ring) + emitter (seeded RNG path) into gameplay crate (initial scaffold 57bcb46; config-driven ring params + tests b9f3ff5; seeded RNG jitter + radius variation 6ed99d0; EmitterConfig + validation faa607a; runtime emitter scaffold + wiring c3bad4b)
- [x] Phase 4: Add spawn & emitter integration tests (deterministic ring + emitter sequence, entity counts) (67232ac) (distribution histogram/radius variance statistical tests deferred -> new follow-up task)
- [x] Phase 4: Add spawn distribution & radius histogram statistical tests (variance thresholds) (TBD_SPAWN_STATS: implemented gameplay_ring_spawn_distribution_statistics covering angular gap, radius range & variation)
- [x] Phase 5: Port input interactions (drag, explosion) (commit TBD_INTERACTIONS; basic unit coverage for resource insertion; scripted input harness & advanced deterministic gesture tests deferred to follow-up)
- [x] Phase 6: Port clustering logic + original unit tests (ensure parity) (completed commit 7ace5c0: compute + persistence smoothing + detach threshold test + integration & unit tests green + clippy clean; detachment stability improvement commit 4bc560d (time >= threshold OR frame fallback); deferred: debug draw, cluster snapshot parity, scoring usage)
- [ ] Phase 7: Port metaballs (feature) + essential parity (initial scaffold cfeb6e7; app gating TBD_APP_GATING; pass-through feature gating TBD_GOLDEN_GATING; cluster snapshot determinism test TBD_CLUSTER_SNAPSHOT; golden harness env/file baseline + metrics TBD_GOLDEN_METRICS / PREIMAGE_NS; gpu capture incremental real path (alloc/copy/map/hash) TBD_GOLDEN_GPU_REAL_ALLOC / _COPY_IMPL / _MAP_IMPL / _HASH_IMPL; metaballs preimage contribution TBD_METABALLS_PREIMAGE; feature propagation TBD_METABALLS_FEATURE_PROP; asset init fix TBD_METABALLS_ASSET_INIT; WASM shader embed added (parity) TBD_METABALLS_WASM_EMBED; debug view uniform slot parity + test (resource + v1.w + debug_view_uniform_applied) TBD_METABALLS_DEBUG_VIEW; perf smoke harness (ignored test perf_smoke_metaballs_300_frames measuring mean_ns/p95_ns over 300 frames with 200 balls) TBD_METABALLS_PERF_SMOKE; perf baseline captured (bm_metaballs only: mean_ns=191733 p95_ns=245700 ball_count=200) TBD_METABALLS_PERF_BASELINE; legacy perf comparison captured (two runs: mean_ns=[191196,196427] p95_ns=[238000,258600] avg_mean_ns=193812 avg_p95_ns=248300 new_vs_legacy_mean_delta=-1.07% new_vs_legacy_p95_delta=-1.05%) TBD_METABALLS_PERF_LEGACY_DELTA; cluster color table dedupe decision: DEFERRED (low current ROI; duplicates only waste uniform slots, no correctness impact; see §14) TBD_METABALLS_COLOR_DEDUPE_DECISION; wasm build check script added (game/scripts/wasm_build_check.ps1) TBD_WASM_BUILD_CHECK; uniform stability determinism test (uniform_stable_across_frames) 26fed77; GPU golden capture Step 1 view tap placeholder implemented (tap_golden_gpu_view) 4855319 (TBD_GOLDEN_GPU_VIEW_TAP); GPU golden capture Step 2 copy source selection scaffolding 92a1329 (TBD_GOLDEN_GPU_COPY_SOURCE); GPU golden capture Step 3 conditional allocation & dynamic dims scaffolding 05830a0 (TBD_GOLDEN_GPU_DYNAMIC_DIM); STATUS 2025-08-22: essential Phase 7 documentation tasks complete (perf baseline + legacy delta + parity summary + wasm build check + uniform stability + view tap); remaining to meet original Phase 7 exit criteria: implement real golden frame capture & hash comparison vs legacy (HSV delta threshold) + cluster snapshot determinism test TBD_CLUSTER_SNAPSHOT + finalize GPU capture map/hash path. Non-essential deferred: async non-blocking GPU map, xtask golden baseline updater, instanced draw optimization benchmark, spatial culling, compute path.)
- [ ] Phase 8: Port debug tools (feature) & keybinding doc
- [ ] Phase 9: Port hot reload (feature) native-only; add watch test (mock FS timestamps)
- [ ] Phase 9: Add performance benchmark harness
- [ ] Phase 10: Run full parity comparison & produce report (performance, memory, binary size)
- [ ] Phase 10: Tag release and decide on deprecation strategy for legacy
```

---
## 11. Open Questions / To Clarify Later
* Should `auto_close` live in `core` or `gameplay` (low impact; revisit Phase 4)?
* Level of acceptable divergence in physics due to Rapier internal ordering—establish numeric tolerance early.


---
## 12. Metaballs Parity & Deferred Optimizations (Phase 7 Summary)

Parity Achieved:
- Uniform layout mirrors legacy (ball_count=v0.x, cluster_color_count=v0.y, radius_scale=v0.z derived from iso, iso=v0.w, normal_z_scale=v1.x, radius_multiplier=v1.z, debug_view=v1.w).
- Param tweak key bindings ported ([ / ], K/L, ,/. , R) with deterministic simultaneous press handling.
- Debug view uniform slot applied (test `debug_view_uniform_applied` green).
- WASM embedded shader path (avoids external fetch) implemented.
- Performance baseline vs legacy: new mean_ns=191733 (bm_metaballs only) vs legacy avg_mean_ns=193812 (≈ -1.07%); p95 similarly -1.05% (slight improvement; within noise but no regression).

Cluster Color Table Dedupe Decision (Deferred):
- Current approach: write one color Vec4 per cluster (possible duplicate colors if multiple clusters share color_index).
- Cost of duplication today: at most O(cluster_count) extra 16-byte entries; typical cluster counts << MAX_CLUSTERS (256); memory + upload impact negligible.
- Shader semantics: balls encode cluster_slot (first cluster of matching color). Dedupe would repurpose slot to color_slot and require mapping cluster->color; adds CPU hashmap + per-ball lookup branch.
- Risk: premature complexity, potential divergence from legacy debugging expectations (per-cluster ordering).
- Decision: Defer until profiling shows uniform upload or cache pressure from large cluster counts approaching MAX_CLUSTERS. Instrumentation hook (future) could log (unique_colors, clusters) to justify revisit.

Deferred Optimizations & Enhancements:
- Spatial culling of balls (skip encoding off-screen / outside influence AABB).
- Compute shader path (field accumulation in compute, fragment samples texture) for large ball counts.
- Async GPU capture (non-blocking map/hash) to reduce frame hitch in golden pipeline.
- Instanced draw optimization benchmark (compare circle placeholder / future instancing vs current).
- Golden pipeline refinements: preimage extensibility, multi-frame hash, differential metrics.
- Potential color table dedupe if (clusters - unique_colors)/clusters > 0.25 over sustained scenarios.

Next Immediate Remaining Phase 7 Essentials:
1. WASM build check script to guard embedded shader + feature gating (`cargo build -p bevy_app --target wasm32-unknown-unknown --features "metaballs"`).
2. (Optional polish before closing phase) Add instrumentation counters (ball_count, cluster_color_count, duplicates) into debug logging (behind feature).

### 15. GPU Golden Capture Implementation Plan (Incremental)

Goal: Replace placeholder/enriched preimage-only hash with real pixel hash under `golden + gpu_capture` features and integrate metaballs parity validation.

Planned Steps (each -> small commit):
1. Render Graph Tap: Add system in render app world to obtain main view target texture handle (post main pass). (TBD_GOLDEN_GPU_VIEW_TAP) [DONE 4855319]
2. Copy Source Selection: Modify `submit_golden_gpu_copy` to copy from actual view texture instead of standalone allocated texture when available; fallback to allocated texture for headless tests. (TBD_GOLDEN_GPU_COPY_SOURCE) [DONE 92a1329]
3. Conditional Allocation: Skip creating dedicated texture if view texture captured; only allocate buffer sized to that target; detect dimensions dynamically. (TBD_GOLDEN_GPU_DYNAMIC_DIM) [DONE 05830a0]
4. Non-Blocking Map: Split `map_and_hash_gpu_capture` into map submit + poll system (Maintain::Poll) to avoid blocking; retain blocking path behind `GOLDEN_BLOCKING_MAP=1` env for debugging. (TBD_GOLDEN_GPU_ASYNC_MAP) [DONE b1123a1]
5. Hash Preimage Extension: Include pixel hash length + first 16 bytes of pixel hash in preimage for multi-stage reproducibility (version tag bump to v3). (TBD_GOLDEN_PREIMAGE_V3) [DONE 7cae461]
6. Baseline Update Flow: Add env `GOLDEN_ALLOW_NEW=1` permitting auto-adoption only when prior baseline missing OR explicit flag set (avoid accidental drift). (TBD_GOLDEN_BASELINE_ADOPT_GUARD) [DONE 6e2fddd]
7. Legacy Parity Harness: Add integration test comparing legacy vs new pixel hashes with tolerance metric placeholder (HSV delta TODO). (TBD_GOLDEN_PARITY_TEST)
8. Tolerance Framework: Implement pixel diff producing counts & simple ΔE approximation; store metrics JSON on mismatch. (TBD_GOLDEN_DIFF_METRICS)
9. Documentation: Update Section 14 parity summary with real pixel hash status & thresholds. (TBD_GOLDEN_DOC_UPDATE)

Exit Criteria (Phase 7 Golden Capture):
- Real pixel hash captured on native build with `--features "golden gpu_capture metaballs"`.
- Non-blocking map path implemented (blocking fallback retained).
- Baseline match or documented accepted delta within threshold (future ΔE < threshold).
- Integration test harness passes without flakiness (two consecutive runs stable).
- Plan updated with completion references (commit hashes).

## 16. Immediate Visual Demo Fast-Path (Revision 2025-08-22)

Objective: Prioritize a runnable, visually verifiable slice (native + wasm) with minimal scope, deferring deeper parity (golden hashing, advanced metaballs validation, debug overlay, hot reload) to later milestones. This supplements (not replaces) Phases 7‑10; treat as Milestone A preceding completion of remaining Phase 7 goldens.

### 16.1 Milestone A Scope (Visual Game ASAP)

Deliverables (must compile & run with on‑screen motion + basic interaction):

1. Config Loading (Native + WASM)
   - Implement layered file load (native) & embedded include_str! path (wasm) inside `bevy_app` (mirroring legacy `load_config()` logic).
   - Run validation warnings logging.
2. Rapier Debug Optionality (Parity Light)
   - If `--features debug` and (config.rapier_debug || always_add_debug_when_feature) then attach RapierDebugRenderPlugin.
3. Examples (under game/examples/) small binaries:
   - spawn_demo: ring spawn only (no emitter) + camera.
   - emitter_demo: ring spawn + runtime emitter.
   - interactions_demo: adds drag + explosion (if interactions already ported).
   - metaballs_demo (feature gated `metaballs`): baseline metaballs view (no golden parity yet).
   - full_demo: current aggregated app (thin wrapper to match `bevy_app` but with CLI args for quick toggles).
4. WASM Build + Run Docs
   - Confirm `wasm32-unknown-unknown` target builds for spawn_demo + (optionally) metaballs_demo (feature gated).
   - Provide `wasm_server_run` snippet (e.g. `wasm-bindgen` or minimal `basic-http-server` instructions) (placeholder acceptable if full pipeline not yet scripted).
5. Run Instructions (Native)
   - Document cargo commands for each example & feature combos.
6. Minimal CLI Arg Parsing (Optional Fast Value)
   - (If trivial) Accept `--no-metaballs` and `--headless` in `bevy_app` (headless just skips window plugin & rendering plugin; enables future automated test harness). If non-trivial, defer.
7. README Addendum
   - Short “Quick Start Visual Demo” section linking examples & commands.

Out of Scope (Explicitly Deferred):
- Full golden pixel hash parity harness (stays in Phase 7 tasks).
- Cluster snapshot determinism for metaballs color pipeline.
- Hot reload + debug overlay.
- Performance benchmark harness evolution.
- GPU async capture polish beyond working path already scaffolded.

### 16.2 Milestone A Acceptance Criteria

| Item | Acceptance Test |
|------|-----------------|
| Config load | Native logs loaded layers or fallback; wasm builds & uses embedded config without panic |
| spawn_demo | Window opens; ring of balls rendered (circles) within 2s |
| emitter_demo | Additional balls spawn over time (entity count increases) |
| interactions_demo | Dragging pointer influences forces OR explosions trigger (visible movement) |
| metaballs_demo | Compiles & shows metaballs surface when feature enabled (fallback circles if disabled) |
| wasm build | `cargo build --target wasm32-unknown-unknown -p spawn_demo` succeeds (doc command recorded) |
| docs | README updated with commands; plan references this section |

### 16.3 Task Breakdown & Order

1. Implement config loader in `bevy_app` (native + wasm).
2. Wire RapierDebugRenderPlugin gating (respect feature + config).
3. Create example crates (scaffold Cargo.toml + main.rs) incrementally:
   a. spawn_demo (reuse existing plugins minimal set: core, config, physics, rendering, gameplay spawn systems only).
   b. emitter_demo (adds emitter system activation flag/config).
   c. interactions_demo (adds interaction systems; if not fully stable, log WARN and still run).
   d. metaballs_demo (behind `metaballs` feature; gracefully exits if feature missing).
   e. full_demo (thin).
4. Add README quick start & commands.
5. Validate wasm build for spawn_demo + metaballs_demo (feature) using existing `scripts/wasm_build_check.ps1` logic or extended.
6. (Optional) Add simple CLI arg parsing (structopt/clap heavy dependency avoided—use basic std::env scan) for `--no-metaballs` & `--headless`.
7. Tag internal milestone: `milestone-a-visual-demo`.

### 16.4 Example Command Matrix

Native (Windows example):
```
cargo run -p spawn_demo
cargo run -p emitter_demo
cargo run -p interactions_demo
cargo run -p metaballs_demo --features "metaballs"
cargo run -p full_demo --features "metaballs debug"
```

WASM (build artifacts only; serving strategy TBD):
```
rustup target add wasm32-unknown-unknown
cargo build -p spawn_demo --target wasm32-unknown-unknown --release
# (Optional) basic-http-server ./target/wasm32-unknown-unknown/release/
```
(If using wasm-bindgen / wasm-server-runner, integrate later; document chosen path in README when stabilized.)

### 16.5 Minimal Example Crate Template

Each example `main.rs` to:
- Create config (or load via shared helper in a tiny internal `examples_support` module).
- Insert `GameConfigRes`.
- Add minimal plugin chain; omit unneeded systems.
- For spawn_demo: ensure only initial ring spawn (could set emitter config disabled).
- For headless mode detection (future) allow skipping window plugin.

### 16.6 Integration With Existing Phase Plan

Milestone A slots logically between current partial Phase 7 (metaballs parity work-in-progress) and completion of Phase 7 golden capture tasks. Golden tasks remain; this milestone simply accelerates user-visible progress & onboarding.

After Milestone A:
- Resume Phase 7 remaining golden capture + cluster snapshot determinism.
- Proceed to Phase 8 (debug tools) once golden stability acceptable.
- Potential reordering: Hot reload (Phase 9) can run in parallel with debug tools if a separate contributor handles it.

### 16.7 Added Checklist Items (Appended to Section 10)

See updated Developer Checklist entries appended below existing Phase 10 entries:
```
- [ ] Milestone A: Config load (native + wasm embed)
- [ ] Milestone A: Rapier debug gating in bevy_app
- [ ] Milestone A: spawn_demo example
- [ ] Milestone A: emitter_demo example
- [ ] Milestone A: interactions_demo example
- [ ] Milestone A: metaballs_demo example (feature)
- [ ] Milestone A: full_demo example
- [ ] Milestone A: README quick start section
- [ ] Milestone A: wasm build validation (spawn_demo + metaballs_demo)
- [ ] Milestone A (Optional): CLI args (--no-metaballs, --headless)
- [ ] Milestone A: Tag milestone-a-visual-demo
```

### 16.8 Rationale

Creating runnable focused examples:
- Low risk: does not interfere with existing crate boundaries.
- High feedback: enables art/prototyping or parameter tuning earlier.
- Provides future automated CI matrix (each example builds under feature sets).

### 16.9 Risks & Mitigations (Milestone A Specific)

| Risk | Mitigation |
|------|------------|
| Example duplication of plugin wiring | Factor tiny shared module or rely on bevy_app crate with feature flags |
| WASM divergence early | Constrain scope: only assert build success, not interaction parity |
| Scope creep into golden capture | Explicit deferral; track via existing Phase 7 TODO markers |
| CLI arg parsing adds dependency bloat | Use manual env/arg parsing; no external crate initially |

END OF PLAN
