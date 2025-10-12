# Sub‑Sprint: Migrate to Bevy Processed Asset Pipeline & Enhanced `game_assets`

## 1. Motivation / Problem Statement

We currently:

- Load raw fonts + shaders at runtime via `AssetServer` with root path indirection (`AssetRootMode`).
- Provide centralized handles (`GameAssets`) but no explicit load readiness signal, no processed asset pipeline usage, no embedding integration beyond a placeholder feature.
- Lack gating so downstream systems may race on not‑yet‑ready assets (especially shader graph setup).
- Want deterministic CI / WASM builds, optional preprocessing (future: shader preprocessing, config derivation), and a clean toggle between fast iter dev vs reproducible builds.

## 2. Goals (Acceptance Criteria)

1. Central configuration (`configure_*` helpers) can choose between:
   - Unprocessed (dev fast path)
   - Processed (hash + transform pipeline)
   - Embedded (compiled bytes) optionally layered on either mode (prefer Processed + Embedded for CI/WASM).
2. Feature flags:
   - `embedded_assets` (workspace aggregates `game_assets/embedded`)
   - `processed_assets` (workspace feature toggling `AssetMode::Processed`; native only)
3. Startup readiness marker (`AssetsReady` resource) guaranteed before gameplay/render pipeline systems that depend on shader handles.
4. Load group tracking uses `get_group_load_state` across all critical handles.
5. CI path:
   - Runs with `--features processed_assets,embedded_assets`
   - Fails if asset processing reports any failed transformations.
6. WASM path:
   - Always uses embedded assets (no file watcher).
7. Native Dev path:
   - Default unprocessed, optionally embedded disabled for hot iteration (fast reload by re-run; file watcher not enabled).
8. Documentation updated (crate README + this plan) describing modes & usage.
9. No layering violations: only `game_assets` manipulates asset mode decisions; other crates depend solely on handles or readiness marker.
10. New `Asset_Test` demo:
    - Native: uses `configure_demo` (unprocessed, non-embedded) and prints all asset handles + a readiness log line.
    - WASM: automatically uses embedded assets (no network fetch 404s).
    - Absolutely no build-time copying into a staging directory (e.g., no `embedded_assets/` folder).

Stretch (not required for acceptance but desirable):

- Basic custom processor scaffold present (no-op identity for shaders) so future transforms slide in with minimal changes.

## 3. Non-Goals

- Implement actual shader minification or config transformation now.
- Introduce third-party asset loader frameworks (`bevy_asset_loader`).
- Hot reload with file watcher on WASM (not feasible).
- Complex caching invalidation metrics reporting.
- Introducing any asset build step that copies or stages assets into a synthetic directory (e.g. `embedded_assets/`). Embedding = compile-time inclusion only.

## 4. Current State Summary

Component | State
----------|------
Root selection | `AssetRootMode` (Demo/Game/Workspace)
Embedding | Feature defined (`embedded`) but unused logic
Processing | Not used (`AssetPlugin` default `Unprocessed`)
Readiness gating | Absent
Custom assets | None
CI integration | Not asset-mode aware

## 5. Target Architecture Overview

```
            +-------------------+
            | configure_* API   |
            | decides Mode      |
            +---------+---------+
                      |
        +-------------v-----------------+
        | AssetPlugin(mode, meta_check) |
        | (Processed OR Unprocessed)    |
        +-------------+-----------------+
                      |
          (optional feature: embedded)
                      |
            +-------------------+
            | GameAssetsPlugin  |
            | - load handles    |
            | - track group     |
            | - insert AssetsReady
            +-------------------+
```

## 6. Phased Migration Plan

### Phase 0 – Prep (Day 1)

- Add workspace features (`embedded_assets`, `processed_assets`).
- Update `game_assets/Cargo.toml` with `bevy_embedded_assets` optional dependency.
- Commit scaffold (no behavior change yet).

### Phase 1 – Core Plugin Enhancements (Day 1–2)

Tasks:

- Implement revised `GameAssetsPlugin`:
  - Load assets
  - Store group of `UntypedHandle`s in `PendingAssetGroup`
  - Poll each frame; when loaded → insert `AssetsReady`
- Provide helper `assets_ready(world)`.

Exit Criteria:

- Demos compile & run; `info!("All startup assets loaded.")` log appears; no panics.

### Phase 1.5 – Asset_Test Demo Scaffold (Day 1)

Tasks:

- Create `demos/asset_test` crate.
- Use `configure_demo(&mut app)` to standardize asset root.
- Add a system that:
  - Logs each handle path and its `LoadState` each frame until all are `Loaded`.
  - Inserts a marker `AllAssetsLogged` (local or resource) after first full-ready transition to avoid spam.
- For wasm builds: rely on future `embedded_assets` feature (when enabled) with zero code changes in the demo.

Exit Criteria:

- `cargo run -p asset_test` prints a summary:
  "Asset group loaded: fonts=Loaded shaders=Loaded"
- When compiled for wasm with embedding, no network fetch failures appear.

### Phase 2 – Mode Configuration (Day 2)

Tasks:

- Extend `configure_standard_assets`:
  - Accept `processed: bool`
  - Accept `embedded: bool` (or derive from features)
- Choose `AssetMode` accordingly (`Processed` if `processed_assets` feature set and not wasm32).
- Register `EmbeddedAssetPlugin` only if `embedded_assets` feature set.

Edge Handling:

- If user requests processed on wasm32 and pipeline unsupported → fallback with a `warn!`.

### Phase 3 – Feature Gate Wiring (Day 2–3)

Tasks:

- Workspace `Cargo.toml`: `[workspace.features] embedded_assets = ["game_assets/embedded"]; processed_assets = []`
- Add compile-time cfg helpers in `game_assets`:
  - `cfg!(feature = "processed_assets")`
  - `cfg!(feature = "embedded_assets")`
- Provide public function `select_asset_modes()` returning `(processed, embedded)`.

### Phase 4 – Downstream Integration (Day 3)

Tasks:

- In each demo/game crate: replace root asset setup with new API (likely a single call: `configure_demo_with_defaults(app)` – optional convenience wrapper).
- Gate initialization systems that depend on assets using:
  - `run_if(resource_exists::<game_assets::AssetsReady>())`
  - or state transition added inside `game_assets` (document approach).
- Update `Asset_Test` to switch from ad-hoc polling to the new `AssetsReady` resource once implemented, replacing manual load-state logging with a single readiness confirmation log line.

### Phase 5 – CI Pipeline (Day 3–4)

Tasks:

- Update CI script (not in repo yet? Add docs) to build:
  - `cargo build --all --features "processed_assets embedded_assets"`
  - `cargo test --all --features "processed_assets embedded_assets"`
- Add an optional verification system that logs any `Failed` load states (already logged) – ensure build fails if any occurred (possible by inserting a post-startup system that panics if a `LoadState::Failed` was seen; behind `processed_assets` only).

### Phase 6 – Documentation & READMEs (Day 4)

Tasks:

- Update `crates/game_assets/README.md` with:
  - Table of modes (Dev, Processed, Processed+Embedded)
  - Feature flags
  - Example code snippet
- Update root `crates/README.md` (already an overview) with a bullet referencing asset modes.
- Add summary to `design-doc.md` (optional).

### Phase 7 – Optional Custom Processor Scaffold (Stretch, Day 5)

Tasks:

- Add stub `shader_processor.rs`:
  - Implements an `Identity` transformer capturing dependency list (future: chunk inclusion, macro expansion)
  - Register if `processed_assets`.
- Document extension point.

## 7. Detailed Task Breakdown

Task | File(s) | Est
-----|---------|----
Add workspace features | `Cargo.toml` | 0.5h
Add embedded dep | `crates/game_assets/Cargo.toml` | 0.25h
Refactor plugin for readiness | `crates/game_assets/src/lib.rs` | 1h
Add mode selection logic | same | 0.5h
Update demos asset config | each `demos/*/src/main.rs` | 1h
Add CI build feature flags doc | `scripts/` + docs | 0.5h
README updates | 2 READMEs + this plan | 0.75h
Stretch: processor scaffold | `shader_processor.rs` | 1h

Total core ~4–5h (excluding stretch).

## 8. Risks & Mitigations

Risk | Impact | Mitigation
-----|--------|-----------
Incompatible `bevy_embedded_assets` version w/ 0.16 | Build fail | Pin exact version; verify release notes.
Processed mode overhead slows dev iteration | Dev friction | Default to unprocessed; require explicit feature.
Unexpected asset load order issues | Runtime panic | Use group load gating with single readiness marker.
WASM fallback mismatch | Load failure | Conditionally compile; warn and fallback to unprocessed+embedded.
Future shader transform invalidates assumptions | Silent break | Unit test placeholder: ensures processed & unprocessed produce same WGSL text initially.
Accidental asset copy workflow introduced | Violates no-copy invariant | Add CI check to assert absence of `embedded_assets/` output directory

## 9. Rollback Strategy

- Features are additive; disable by removing `--features processed_assets embedded_assets`.
- Revert plugin changes via single commit (keep old version tag `pre-asset-pipeline`).
- Maintain old code path comment for one sprint.

## 10. Testing Strategy

Type | Focus
-----|------
Unit | `assets_ready` returns false → true after asset server finishes (can simulate by polling until loaded).
Integration (demos) | Ensure no systems needing shaders run before readiness (add assert or warning).
WASM smoke | `wasm-dev.ps1 -Embed` loads without 404 network fetches.
CI Path | Build with both features; ensure log line "All startup assets loaded." appears exactly once.

Optional Test Snippet (pseudo):

```rust
#[test]
fn assets_ready_eventually() {
    // Build minimal App with headless schedule + game_assets configured.
    // Step frames until readiness; assert within N frames.
}
```

## 11. Implementation Notes

- Keep `meta_check: AssetMetaCheck::Full` initially for safety; consider `Quick` in tight-loop dev.
- Consider adding `warn!` if both features disabled in CI environment variable (e.g., `CI=true`).
- Avoid adding file watcher feature back in; maintain WASM compatibility.
- Invariants: (1) No build-time asset copying step. (2) Only `game_assets` decides asset mode. (3) Demos never hardcode relative paths outside of helper calls.

## 12. Commands Cheat Sheet

Dev (fast):

```pwsh
cargo run -p compositor_test
```

Processed (native):

```pwsh
cargo run -p compositor_test --features processed_assets
```

Embedded + Processed (CI / WASM):

```pwsh
cargo build --all --features "processed_assets embedded_assets"
```

WASM (script):

```pwsh
pwsh scripts/wasm-dev.ps1 -Embed
```

## 13. Future Enhancements (Backlog Seeds)

Item | Description | Priority
-----|-------------|---------
Config Asset Type | Typed gameplay config (RON/JSON) pre-validated | High
Shader Include Preprocessor | Support `#include` merging & caching full_hash | Medium
Texture Atlas Packing | Offline packer producing a single atlas meta | Medium
Audio Transcoding | Convert source audio (e.g., wav) → compressed target | Low
Asset Version Pinning Tool | Compare committed hash manifest vs current | Low

## 14. Definition of Done Checklist

- [ ] Feature flags defined & documented
- [ ] `GameAssetsPlugin` exposes `AssetsReady`
- [ ] All demos updated to use readiness gating
- [ ] CI instructions updated
- [ ] README(s) updated
- [ ] Local dev run unaffected (no slowdown)
- [ ] WASM build success with embedded
- [ ] At least one unit test or harness verifying readiness (optional but preferred)
- [ ] Plan file (this) committed
- [ ] `Asset_Test` demo logs readiness (native) and runs under wasm with embedded assets (no 404s)
- [ ] No `embedded_assets/` or similar staging directory produced during builds

---

Author: (add name)
Date: (fill on commit)
Status: Draft → In Progress → Complete
