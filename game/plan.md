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
- [ ] Phase 0: Scaffold workspace & stub crates
- [ ] Phase 1: Port config crate + tests (no bevy)
- [ ] Phase 1: Port core crate (components, system sets)
- [ ] Phase 2: Implement physics crate (rapier setup, radial gravity, separation) + deterministic headless test
- [ ] Phase 3: Implement rendering crate (camera, background, materials, palette)
- [ ] Phase 4: Port spawning (ring) + emitter (seeded RNG path) into gameplay crate
- [ ] Phase 4: Add spawn & emitter integration tests (entity count distribution)
- [ ] Phase 5: Port input interactions (drag, explosion) with scripted input test
- [ ] Phase 6: Port clustering logic + original unit tests (ensure parity)
- [ ] Phase 7: Port metaballs (feature) + golden frame baseline capture
- [ ] Phase 8: Port debug tools (feature) & keybinding doc
- [ ] Phase 9: Port hot reload (feature) native-only; add watch test (mock FS timestamps)
- [ ] Phase 9: Add performance benchmark harness
- [ ] Phase 10: Run full parity comparison & produce report (performance, memory, binary size)
- [ ] Phase 10: Tag release and decide on deprecation strategy for legacy
```

---
## 11. Migration Log

| Date | Phase | Commit | Notes |
|------|-------|--------|-------|
| (tbd) | 0 | | Workspace scaffold created |

---
## 12. Open Questions / To Clarify Later
* Do we foresee networked/multiplayer requirements? (Would influence determinism & architecture.)
* Should `auto_close` live in `core` or `gameplay` (low impact; revisit Phase 4)?
* Level of acceptable divergence in physics due to Rapier internal ordering—establish numeric tolerance early.

---
## 13. Immediate Next Steps
1. Execute Phase 0 checklist item: create workspace + stub crates with minimal `lib.rs` + feature placeholders.
2. Port config + tests (Phase 1 start).
3. Seed RNG abstraction early to avoid test rewrites later.

---
END OF PLAN

