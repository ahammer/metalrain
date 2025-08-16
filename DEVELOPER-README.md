# Developer Documentation (Ball Matcher)

This document provides deep technical details for contributors: architecture, system ordering, configuration design, WebAssembly deployment, and extension guidelines.

---
## Table of Contents
1. Goals & Philosophy
2. High-Level Architecture
3. Plugin Inventory & Responsibilities
4. Configuration (`GameConfig`) Schema
5. Runtime Flow & System Ordering
6. Components & Data Model
7. Metaball Rendering Pipeline
8. Separation Logic (Overlap Mitigation)
9. WebAssembly Build & Deployment (GitHub Pages)
10. Adding New Plugins / Systems
11. Performance Considerations & Profiling Tips
12. Testing Strategy
13. Extensibility Roadmap / Ideas
14. FAQ / Troubleshooting
15. License & Contribution Model

---
## 1. Goals & Philosophy
- **Modular**: Each concern is isolated in a focused plugin module.
- **Data-Driven**: Simulation parameters live in a RON config, not hard-coded constants.
- **Deterministic-ish**: Random spawn seeds kept local; acceptable nondeterminism for visuals.
- **Learnable**: Minimal ceremony to add or reorder systems.
- **Inspectable**: Optional debug visualization (Rapier & cluster AABBs) for reasoning about behavior.

## 2. High-Level Architecture
```
main.rs -> loads config (native: fs; wasm: embedded) -> inserts GameConfig resource
          -> configures primary window -> adds GamePlugin (+ optional debug plugins)

GamePlugin -> aggregates: Camera, Spawn, Emitter, Materials, Physics (Rapier),
              Separation, Clustering, Metaballs, Interactions, Radial Gravity, etc.
```
The `GamePlugin` is the single composition point: add/remove new features here.

## 3. Plugin Inventory & Responsibilities
| Plugin | Purpose | Notes |
|--------|---------|-------|
| `camera` | Spawns 2D camera | Single startup system |
| `spawn` | Initial batch of balls | Reuses one mesh handle; random attributes |
| `emitter` | Continuous spawning toward coverage target | Rate/threshold can move to config |
| `materials` | Palette + material setup | Centralized colors & restitution variants |
| `rapier_physics` | World gravity & static walls | Rebuilds bounds on resize |
| `separation` | Pairwise gentle push + velocity damp | Runs after physics contacts |
| `cluster` | Detects touching same-color components | Spatial hash + union-find |
| `metaballs` | Fullscreen WGSL pass (blending) | Packs per-ball into uniform buffer |
| `radial_gravity` | Custom radial gravity field | Pre-physics force injection |
| `input_interaction` | Explosions & dragging (configurable) | Pointer-based, gated by config |
| `emitter` | Periodic additional entities | Avoids front-loading too many entities |

## 4. Configuration Schema (`GameConfig`)
Located in `config.rs`, loaded at startup. On wasm, `include_str!` embeds RON.
Key sections:
- `window`: size + title
- `gravity`: vertical acceleration (Y axis)
- `bounce`: restitution scalar for wall collisions
- `balls`: spawn ranges (count, radius, position, velocity)
- `separation`: overlap avoidance tunables
- `rapier_debug`, `draw_circles`, `metaballs_enabled`, `draw_cluster_bounds`
- `interactions`: `explosion` & `drag` parameters

Add a field -> update struct + RON file + system usage (avoid panics with defaults). Tests validate parsing.

## 5. Runtime Flow & System Ordering
Simplified per-frame order:
```
PrePhysicsSet: (custom forces) -> Rapier step -> PostPhysicsAdjustSet: separation -> render pipeline
```
Ordering managed with labeled `SystemSet`s. When adding a new velocity-modifying system, place it in `PrePhysicsSet` before movement.

## 6. Components & Data Model
| Component | Description |
|-----------|-------------|
| `Ball` | Marker tag |
| `Velocity(Vec2)` | Linear velocity (manual integration or forces applied) |
| Additional cluster / metaball metadata stored in resources & uniform packing structures |

Ball radius is implied by transform scale (diameter) or explicit component depending on rendering path.

## 7. Metaball Rendering Pipeline
- CPU gathers per-ball `(x, y, radius, cluster_index)` into a fixed-size array (max 1024).
- Uniform struct + color table uploaded once per frame.
- WGSL shader evaluates bounded Wyvill kernel: `f = (1 - (d/R)^2)^3` within radius.
- Iso threshold (`iso`) + pseudo-normal derivation for simple lighting.
- Optionally hide circle meshes when relying purely on metaballs (config toggles).
- Potential future: compute shader tile culling or SSBO storage for >1024 balls.

## 8. Separation Logic
When a contact event occurs:
1. Determine desired spacing = `(r1 + r2) * overlap_slop`.
2. If distance < desired, compute overlap.
3. Compute push vector along normalized center delta.
4. Apply half push per entity (clamped by `max_push * push_strength`).
5. Dampen velocity component along normal (scales by `velocity_dampen`).
Edge cases: identical centers -> pick fallback axis; very small overlaps -> skip to reduce jitter.

## 9. WebAssembly Build & Deployment
### Local Build
```
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/ball_matcher.wasm --out-dir web --target web
```
Optional optimization:
```
wasm-opt -Oz web/ball_matcher_bg.wasm -o web/ball_matcher_bg.wasm
```
### CI Workflow
`.github/workflows/deploy.yml` steps:
1. Install toolchain & wasm target
2. Cache cargo dirs
3. Build release wasm
4. Install `wasm-bindgen-cli`
5. Generate JS glue into `web/`
6. Run (best-effort) `wasm-opt` optimization
7. Copy `assets/` -> `web/assets/`
8. Touch `.nojekyll`, add minimal `404.html`
9. Deploy `web/` to `gh-pages` via `peaceiris/actions-gh-pages`

Embedded config avoids runtime file IO on wasm. Panic hook prints to browser console.

### Canvas / Loader
`web/index.html` provides a progress bar and swaps out once engine initializes. Fetch hook provides approximate download progress for browsers supporting streaming.

## 10. Adding New Plugins / Systems
1. Create new `src/<name>.rs` with a `pub struct <Name>Plugin;` implementing `Plugin`.
2. Register in `game.rs` inside `GamePlugin` ordering tuple.
3. Optional: add config fields (update `GameConfig` + RON).
4. For systems needing deterministic order, create/extend a `SystemSet`.
5. Add tests for core logic (see `cluster` & `config` tests).

## 11. Performance Considerations
| Area | Tip |
|------|-----|
| Spawning | Reuse meshes & materials (clone handles). |
| Overlap Separation | Keep math branch-light; bail early if distance large. |
| Metaballs | Limit max balls; consider dynamic cutoff based on radius & contribution. |
| Allocations | Reuse `Vec` buffers in clustering (clear each frame). |
| Window Resize | Only rebuild walls when size actually changes. |

### Profiling
- Use `bevy_diagnostic` / log instrumentation.
- For wasm, rely on browser performance tools; consider `tracing-wasm` (already included indirectly).

## 12. Testing Strategy
Examples in `config.rs`, `cluster.rs`, and separation logic.
Pattern:
```
#[test]
fn some_behavior() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // insert resources / spawn entities
    app.update();
    assert!(/* condition */);
}
```
Prefer narrow, deterministic assertions (e.g. cluster counts, velocity adjustments). Avoid frame-count sensitive timing tests.

## 13. Extensibility Roadmap
- Hot-reload config (AssetServer + watcher)
- GPU-based clustering & field evaluation
- Ball-ball elastic collision resolution (beyond separation)
- Interaction UI (toggle features in real time)
- Persistence (serialize seeds / states)
- WASM size trimming (feature gating audio, rapier debug, etc.)

## 14. FAQ / Troubleshooting
| Problem | Resolution |
|---------|------------|
| WASM build fails with getrandom backend msg | Ensure `.cargo/config.toml` has `--cfg=getrandom_backend="wasm_js"`. |
| UUID js feature missing | Add `uuid = { version = "1", features = ["js"] }` under wasm target. |
| No metaball effect | Confirm `metaballs_enabled: true` and circle meshes hidden only if desired. |
| Jitter after separation | Lower `push_strength` or increase `overlap_slop`. |
| Rapier debug not visible | Set `rapier_debug: true` in config before startup. |

## 15. License & Contribution Model
The project is licensed under **GPL-3.0-or-later** (see `LICENSE`).

By submitting a contribution you agree to license your work under the same GPL-3.0-or-later terms. Please:
- Keep changes focused & documented
- Add/adjust tests when behavior changes
- Update `assets/config/game.ron` if you introduce new config fields

Standard GPL notice for source headers (optional but recommended):
```
// This file is part of Ball Matcher.
// Copyright (C) 2025 Adam and contributors
// SPDX-License-Identifier: GPL-3.0-or-later
```

For significant new modules, include a brief module-level comment describing purpose & system ordering constraints.

Happy hacking! Add yourself to CONTRIBUTORS (create if missing) when submitting significant changes.
