# Ball Matcher Codebase Audit (2025-08-16)

Focus: Only opportunities to simplify, harden, or better organize for future growth & maintainability. Strengths intentionally omitted.

## Snapshot
- Bevy 0.14 + Rapier 0.27 2D game / simulation (balls + clustering + metaballs visualization & interactions)
- Architecture split into many focused plugins (good baseline) but some cross‑cutting concerns & duplicated color logic.
- Test coverage exists for several subsystems but is uneven; rendering & interaction logic largely untested; some tests are placeholders.
- Config driven via RON, no hot‑reload yet (empty `config_hot_reload.rs`).

## High-Level Themes
1. Clarity & Consistency: Naming, duplication of palette logic, empty + unused modules, comments vs code divergence.
2. Configuration & Tuning: Centralized config heavy but lacks schema validation, defaults, live reload, feature gating pattern.
3. Data-Oriented ECS Practices: Queries sometimes re-count iterators per frame; potential micro‑optimizations & caching; broad-phase clustering could be simplified.
4. Rendering & Materials: Metaballs shader path packs large uniform; could move to storage buffer once on Bevy >=0.15 (expected wgpu improvements) to lift MAX limits.
5. Testing & Tooling: No CI, no lint gating (`clippy` / `fmt`), sparse property tests, limited benchmarks.
6. Error Handling & Logging: Panics (`expect`) for recoverable config issues; logging could be structured & rate‑limited for perf.
7. Plugin Boundaries & System Ordering: Manual sets exist but missing explicit dependencies for some ordering assumptions (e.g., cluster debug draw vs metaball update interplay, interaction vs separation ordering clarity).
8. Performance & Scalability: O(N^2) worst cases inside clustering union-find despite hashing; separation system may over-allocate hash maps each frame.
9. Unused / Dead Code: `physics.rs` appears unused (custom simple physics vs Rapier) – should be removed or gated. Empty `config_hot_reload.rs`.
10. Future Extensibility: Lacks feature flags (Cargo) to compile out experimental systems (metaballs, clustering, radial gravity) for web/mobile size.

## Detailed Findings & Recommendations
### 1. Naming & Module Hygiene
- `physics.rs` vs `rapier_physics.rs`: Remove or rename `physics.rs` to `legacy_physics.rs` and gate behind feature if still educational.
- Empty file `config_hot_reload.rs` -> implement or delete. A stub creates confusion.
- System set names: `PostPhysicsAdjustSet` could be shortened to `PostPhysicsSet` or split into `PostPhysicsCorrectionSet` & later visual sets.
- Consistent prefix for ball-related components (e.g., `BallRadius` ok, but `Ball` marker generic). Option: `BallTag` to clarify.

### 2. Palette & Color Logic Duplication
- Color palette repeated in `materials.rs`, `cluster.rs`, `metaballs.rs`. Centralize into a `ball_palette` module exposing:
  - `const BASE_COLORS: [Color; N]`
  - helper `fn color_for_index(i: usize) -> Color`.
- Provide palette handle->linear color mapping so metaballs & debug drawing pull from single source (prevent drift when palette changes).

### 3. Configuration Improvements
- Add `Default` impl + `GameConfig::load_or_default(path)` to allow missing file fallback.
- Add validation method returning `Vec<String>` of warnings (e.g., negative radii, inconsistent ranges, drag max_speed < pull_strength heuristic).
- Introduce layered config: `base.ron`, `local.ron` (ignored via `.gitignore`) merged at startup.
- Implement hot reload using file watcher (e.g., `notify` crate) updating resource & emitting event; systems subscribe to adjust parameters.
- Provide dynamic control toggles via keyboard for enabling/disabling heavy systems (metaballs, cluster AABB draw) reading a shared `DebugToggles` resource.

### 4. ECS Query & Data Patterns
- In `cluster.rs` cluster building: Avoid multiple linear scans (`entities.iter().position`) by constructing an `Entity -> idx` map once.
- Preallocate vector capacities using `count` to prevent reallocation in loops where possible (some done; refine others e.g., `existing_ids`).
- Store per-ball persistent data as component (`ClusterPersist` component) instead of HashMap resource to leverage archetype locality & avoid hashing each frame (tradeoff: component churn on despawn). Consider after profiling.
- `SeparationPlugin`: Accumulates `HashMap` each frame. Consider reusing a cleared `Local` (e.g., `Local<FrameScratch { pos_shifts: Vec<(Entity, Vec2)> ... }>` plus stable sorting) or use `SmallVec` for typical small collision counts.

### 5. Clustering Algorithm
- Current union-find uses spatial hashing but visits all neighbor cells; could early prune by bounding radii (already partial). For dense scenes >1k entities potential cost; consider hierarchical grid or `par_iter` once Bevy parallel scheduler stable post 0.14.
- Consider storing color partition first: group entities by color_index, then cluster each color subset—reduces comparisons across colors early.
- Provide optional feature flag `cluster-persistence` to compile out stabilization logic for simpler builds.

### 6. Metaballs Rendering
- Uniform block size risk: 1024 * 16B (Vec4) * 2 arrays ~ 32KB + overhead okay now, but future scaling limited. Plan migration to storage buffer when Bevy exposes stable API (track release notes 0.15 / 0.16).
- Factor parameter tweak keys into a debug UI (e.g., `egui` plugin optional) to avoid hard-coded key bindings.
- Extract kernel math & radius_scale computation into a pure helper tested separately.

### 7. Interaction Systems
- Explosion & drag share pointer acquisition. Extract `fn pointer_world(...) -> Option<Vec2>` into dedicated module reused by both; reduces duplication.
- Drag suppression flag resets implicit; add explicit event `DragEnded { moved: bool }` consumed by explosion system to decide.
- Add predictive smoothing: While dragging apply critical damping to approach pointer (like PD controller) for more natural feel.

### 8. Error Handling & Logging
- Replace `expect` in config loading with graceful fallback: log error, use defaults, surface message in window title (`[CONFIG ERROR]`).
- Adopt `tracing` crate with env filter; add per-frame rate limiting for debug counters (or rely on existing timer but centralize).
- Provide instrumentation spans around heavy systems (`compute_clusters`, `update_metaballs_material`) for profiling.

### 9. Testing Strategy
- Add snapshot tests for palette consistency (single source-of-truth).
- Property tests: cluster connectivity invariants (using `proptest`) for random layouts ensuring disjoint color groups never merge.
- Add regression test for separation overlap resolution magnitude within bounds.
- Benchmark harness via `criterion` for clustering vs entity counts (baseline for future refactors).
- Integration test spawning N balls ensures no panic under typical config after X frames.

### 10. Build, CI & Tooling
- Add GitHub Actions: build + `cargo fmt --check` + `clippy -D warnings` + tests + `wasm32-unknown-unknown` build.
- Add `cargo deny` to monitor license / security advisories.
- Add `justfile` or `makefile.toml` with common tasks (run, test, bench, wasm-build, fmt, clippy, release-profile-run).
- Add `cargo features`:
  - `metaballs`
  - `clusters`
  - `radial-gravity`
  - `emitter`
  - default = ["clusters", "metaballs"] etc.; gate modules with `#[cfg(feature = "...")]`.

### 11. Performance & Profiling Roadmap
- Introduce instrumentation & frame timing overlay (simple text system) gating by debug toggle.
- Use `bevy::tasks::AsyncComputeTaskPool` for expensive cluster building if necessary; ensure deterministic writes by staging result into resource.
- Evaluate using Rapier contact graph to derive clusters (same-color contact pairs) instead of custom broad-phase to eliminate duplicate logic.

### 12. Code Style & Lints
- Enable `#![deny(clippy::unwrap_used, clippy::expect_used)]` at crate root after refactors.
- Add module-level docs for each plugin explaining responsibilities & data flows.
- Consolidate repeated constants (e.g., debug key bindings) into a `controls.rs` mapping.

### 13. Dead / Legacy Paths
- Decide on fate of unused custom `physics.rs` (currently conflicts conceptually with Rapier). If educational, move to `examples/` with explanation.

### 14. Documentation
- Augment `README.md` with architecture diagram: startup → materials → spawn → physics → clustering → metaballs.
- Add `docs/architecture.md` elaborating resources & system sets; include change log of design decisions.

### 15. Future Refactors (Higher Effort)
- ECS Data Layout Optimization: Convert cluster persistence HashMap to component or slot map structure.
- GPU Driven Metaballs: Indirect draw of per-ball quads with compute pass generating SDF field (post wgpu 0.20 adoption).
- Editor / Debug Panel: In-game inspector for toggles & live config editing (persist diff back to RON on exit).

## Impact vs Effort Checklist
Legend: [ ] not started. Order = highest leverage first (impact/effort).

1. [ ] Centralize color palette logic (single module) & replace duplicates (Low effort / High impact).
2. [ ] Remove or feature-gate unused `physics.rs`; implement `--features legacy-physics` (Low / High clarity).
3. [ ] Delete or implement `config_hot_reload.rs`; if implementing, minimal file watcher updating `GameConfig` (Low / High).
4. [ ] Add `GameConfig::validate()` returning warnings; log them at startup (Low / Medium).
5. [ ] Introduce basic CI (build + test + clippy + fmt) GitHub Action (Low / High ongoing benefit).
6. [ ] Replace `expect` in config load with graceful fallback + error log (Low / Medium robustness).
7. [ ] Create feature flags for optional systems (clusters, metaballs, radial-gravity, emitter) (Low-Med / Medium).
8. [ ] Add module docs & top-level architecture doc stub (`docs/architecture.md`) (Low / Medium onboarding).
9. [ ] Extract pointer acquisition helper to reduce duplication across interaction systems (Low / Medium clarity).
10. [ ] Add palette consistency test + basic integration test spawn & run frames (Low / Medium).
11. [ ] Refactor clustering to pre-build `entity_index` map to remove O(N^2) position searches (Med / Medium perf & clarity).
12. [ ] Refactor separation system to reuse frame-local scratch buffers (Med / Medium perf).
13. [ ] Add instrumentation (`tracing`) & spans to heavy systems (Med / Medium profiling leverage).
14. [ ] Implement config hot reload (full) with `notify` crate & change events (Med / Medium productivity).
15. [ ] Add property tests for clustering invariants (`proptest`) (Med / Medium reliability).
16. [ ] Add benchmark suite (`criterion`) for clustering & separation (Med / Medium perf regression guard).
17. [ ] Replace hash-map based persistence with component-based approach (High effort / Medium-High perf future scaling).
18. [ ] Evaluate using Rapier contact events directly for cluster building (High / Potential High simplification).
19. [ ] Migrate metaballs data to storage buffer when Bevy exposes stable API (High / High scalability). 
20. [ ] Build in-game debug UI (egui) for live tweaking & toggles (High / Medium developer productivity).
21. [ ] Implement PD-controlled drag smoothing & event-based drag end (Med / UX improvement).
22. [ ] Add layered config merging (base + local override) (Low-Med / Medium flexibility).
23. [ ] Add `.cargo/config.toml` with target-specific opt settings & alias tasks (Low / Medium convenience).
24. [ ] Introduce `justfile` with standard tasks (Low / Medium productivity).
25. [ ] Add documentation diagram & update README (Low / Medium).
26. [ ] Add frame timing overlay & instrumentation output toggle (Med / Medium perf visibility).
27. [ ] Add `cargo deny` config for deps audit (Low / Medium security).
28. [ ] Add lint denies for unwrap/expect after refactor (Low / Medium robustness).
29. [ ] Consolidate key bindings into config or table (Low / Low-Med clarity).
30. [ ] Extract metaballs parameter logic into independent tested math module (Low / Low-Med reliability).
31. [ ] Provide integration example demonstrating feature-flag minimized build (Low / Medium adoption clarity).
32. [ ] Add architecture decision records (ADRs) for major refactors (Low / Medium knowledge retention).
33. [ ] Plan GPU compute-based metaballs (research spike) (High / Future scalability).

## Suggested Execution Phases
- Phase 1 (Week 1): Items 1-10 (quick wins + CI + hygiene).
- Phase 2 (Week 2): Items 11-16 (performance baseline & testing depth).
- Phase 3 (Week 3): Items 17-22 (structural improvements & config flexibility).
- Phase 4 (Week 4+): Remaining backlog & exploratory enhancements.

## Metrics / Success Indicators
- CI green across main & PRs; zero clippy warnings.
- Benchmarks show <=5% regression per PR for clustering baseline size sets.
- Config change (radius range) reflected live without restart within 500ms.
- Palette update requires single-line change.
- Coverage (approx) includes cluster logic, separation, config validation; property tests catch random degenerate cluster merges.

## Appendix: Potential Module Reorg
```
src/
  main.rs
  app.rs (build App & apply features)
  palette.rs
  config/{mod.rs, hot_reload.rs, validate.rs}
  gameplay/{spawn.rs, separation.rs, clustering/{mod.rs, persistence.rs}, interactions/{mod.rs, pointer.rs}}
  rendering/{materials.rs, metaballs/{mod.rs, params.rs}}
  physics/{rapier_setup.rs, radial.rs}
  debug/{timing.rs, ui.rs}
```

## Appendix: Tracing Example
```rust
fn compute_clusters(...) {
    let _span = info_span!("compute_clusters", entities = count).entered();
    // ... existing logic
}
```

## Appendix: Config Validation Sketch
```rust
impl GameConfig {
  pub fn validate(&self) -> Vec<String> {
    let mut w = Vec::new();
    if self.balls.radius_range.min <= 0.0 { w.push("ball radius min must be >0".into()); }
    if self.balls.radius_range.min > self.balls.radius_range.max { w.push("radius min>max".into()); }
    if self.separation.push_strength < 0.0 { w.push("push_strength negative".into()); }
    // ...
    w
  }
}
```

---
End of audit.
