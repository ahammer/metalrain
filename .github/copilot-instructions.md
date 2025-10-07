## AI Coding Agent – Project Instructions (Metalrain)

Purpose: Enable rapid, correct contributions without rediscovering architecture decisions. Keep answers specific to THIS repo; avoid generic Rust/Bevy boilerplate unless tying back to these patterns.

### 1. Architecture Snapshot
- Modular crates (see `Cargo.toml` members). Core domain in `game_core`; no other game/* crate may depend on higher‑level crates (enforce acyclic graph).
- Rendering stack: `background_renderer`, `widget_renderer`, `metaball_renderer` feed coordinated composition via `game_rendering` (layers Background→GameWorld→Metaballs→Effects→UI).
- `game_assets` centralizes fonts + shaders & abstracts asset root modes (demo, game crate, workspace). Always use its configure helpers—never hardcode `assets/` paths.
- Demos (`demos/*`) act as integration tests and examples; prefer extending an existing focused demo before creating a new one.

### 2. Key Workflows
- Build all native: `cargo build --all` (do not add non‑workspace stray crates; update workspace root instead).
- Run representative subsystems: `cargo run -p architecture_test`, physics: `physics_playground`, rendering: `compositor_test`, metaballs: `metaballs_test`.
- WASM (WebGPU only): use `pwsh scripts/wasm-dev.ps1 [-Install] [-Release] [-Embed]`. The script sets runner + optional shader embedding. Prefer `-Embed` for CI / deterministic shader loads, omit for hot reload.
- Tests: `cargo test --all`; add unit tests inside each crate; integration flows belong in demos.
- Lint: `cargo clippy --all -- -D warnings`; Formatting: `cargo fmt --all` (keep CI clean—avoid introducing warnings).

### 3. System & Module Conventions
- Each crate root re‑exports its public modules; keep public API minimal (only components/resources/events/bundles that must cross crate boundaries).
- System naming: verb_noun (`apply_clustering_forces`, `check_win_condition`). Group by lifecycle stage when adding (`Startup`, `Update`, `OnEnter/Exit`).
- Ordering: Physics before Gameplay before Rendering. If adding new sets, chain them accordingly inside `game` or the orchestrating crate.
- Events drive cross‑crate interaction (e.g. `BallSpawned`, `TargetDestroyed`). Prefer emitting an event over directly mutating distant crate state.

### 4. Rendering & Assets
- Never query raw shader file paths; acquire via `Res<GameAssets>.shaders.*`.
- Adding a new render layer: extend enum / mapping in `game_rendering`, allocate offscreen target, then composite back‑to‑front (do NOT break existing order unless justified; document rationale in PR).
- Metaballs: heavy GPU pass—avoid per‑frame dynamic allocations; batch updates in a single system.

### 5. Extending Domain Model
- New shared components/events → `game_core` (keep logic out). Behavior lives in specialized crate (physics, rendering, gameplay). If unsure, default to the lowest layer that doesn’t introduce upward dependency.
- Avoid leaking renderer‑specific types (e.g. WGSL details) into `game_core`—translate via simple enums/structs.

### 6. Asset Root & Demos
- For demos use `configure_demo(&mut app)`; for game crate `configure_game_crate`; workspace root `configure_workspace_root`. Mixing modes causes 404s in browser—fix by picking correct helper.
- When adding new assets place them under existing categorized folders; add typed handle fields to the relevant asset struct rather than ad‑hoc loads in systems.

### 7. Error & Logging Strategy
- Use `warn!` to clamp out‑of‑range config values; panic only for programmer errors (e.g. invariant violations in setup). Log adapter limits once at startup (follow existing pattern in crates README notes).

### 8. Testing & Bench
- Unit tests live next to code (`mod tests { ... }`) or in crate `tests/` directory. Demos function as integration tests; enhance them instead of writing large scenario harnesses elsewhere.
- Performance-sensitive additions (metaball sync, collision loops) should include a benchmark skeleton under the owning crate (see pattern in `metaball_renderer/benches/`).

### 9. Do / Avoid
- DO: Reuse bundles (`BallBundle`, etc.) for spawning; expand bundles if many systems add identical component sets.
- DO: Emit events for cross-crate reactions instead of direct queries into foreign state.
- AVOID: Adding Bevy features that break WASM (`file_watcher`)—the workspace intentionally removed it; gate any native‑only additions behind a feature.
- AVOID: Hardcoding numeric gameplay constants across crates—centralize into a resource or config file.

### 10. PR Guidance
- State which crate boundaries are touched and why. If adding a dependency edge, justify that it doesn’t invert layering.
- Provide a minimal demo scenario (new or updated) demonstrating new behavior; don’t leave dead code paths.

Feedback Wanted: Are any workflows unclear (e.g., adding new shader, introducing gameplay state machine changes, expanding asset embedding)? Point out gaps so we can refine this guide.
