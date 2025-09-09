# Ball Matcher

Current version: https://ahammer.github.io/metalrain/

Ball Matcher is a data-driven, Bevy-based 2D physics demo/game that simulates many interacting balls with metaball rendering and configurable interactions. This repository provides native (desktop) and WebAssembly (WASM) builds. The project enforces a WebGPU-only policy for the web build (no WebGL fallback).

---

## Quick Introduction

- Engine: Rust + Bevy (curated features)
- Physics: Rapier2D (via `bevy_rapier2d`)
- Rendering: custom metaballs shader (WGSL) with modes and optimizations
- Targets: native desktop (Vulkan/Metal/DX12) and wasm32 (Browser WebGPU)
- Config: RON/TOML in `assets/config/` and `assets/levels/`

---

## Getting Started

Prerequisites:
- Rust toolchain (rustup)
- For native: a system with a modern GPU and Vulkan/Metal/DX12 drivers
- For WASM: a modern browser supporting WebGPU (Chrome >=113, Edge, Safari TP, or Firefox Nightly with flags)
- Optional dev helpers: `cargo-watch`, `wasm-server-runner` (see `scripts/wasm-dev.ps1`)

Native (desktop) debug run:
1. From the repo root:
   cargo run

Native release build:
   cargo run --release

WASM (development helper, PowerShell on Windows):
1. Ensure prerequisites (optional install step):
   pwsh scripts/wasm-dev.ps1 -Install
2. Run the dev server (watch mode if cargo-watch present):
   pwsh scripts/wasm-dev.ps1

Notes:
- The WASM helper expects `web/index.html` to be present and the repo root to be the working directory.
- The project uses a strict WebGPU-only policy; unsupported browsers will fail early (see `web/index.html` and `src/webgpu_guard.rs`).

---

## User Manual

Controls and Input
- Primary click / tap: `Mouse Left` / touch tap — action `PrimaryTap` (see `assets/config/input.toml`).
- Toggle overlay / debug keys: `F1`, `Digit1`..`Digit4` (debug modes)
- Metaball iso tuning: `[` and `]` (BracketLeft / BracketRight)
- Virtual axes: `A`/`D` (MoveX) — used by certain interactions

Configuration
- Default runtime config lives at `assets/config/game.ron` (embedded on wasm). Edit or override by adding `assets/config/game.local.ron` for local (native) runs.
- Common tunables: window size/title, gravity, ball counts/radius ranges, metaballs shader params, interaction toggles.

Levels
- Levels are data-driven and live in `assets/levels/`.
- A level is composed of: `basic_walls.ron` (master bounds) + `layout.ron` (unique level walls) + spawn files (`spawn##.ron`). See `assets/levels/readme.md` for intent.

Troubleshooting (user-facing)
- If the WASM build fails in the browser, verify your browser supports WebGPU and that `navigator.gpu` is available.
- If the native build errors with adapter backend mismatch, ensure your system uses Vulkan/Metal/DX12 and that other backends (GL) are not selected.

---

## Developer Manual

Repository layout (high level)
- `src/` — Rust source
  - `main.rs` — native / wasm entry; backend assertions and plugin assembly
  - `lib.rs` — crate exports and module structure
  - `app/` — high-level game plugin (`GamePlugin`)
  - `core/` — components, config parsing, level loading systems
  - `interaction/` — input mapping, gestures, interaction logic
  - `physics/` — clustering, gravity, rapier integration
  - `rendering/` — metaballs, materials, shaders
  - `debug/` — debug overlays and helpers
- `assets/` — configuration, levels, shaders (WGSL), and other game data
- `web/` — minimal web host (`index.html`) to run the WASM build
- `scripts/` — helper scripts (e.g., `wasm-dev.ps1`)

Building & Tooling
- Build and run (native): `cargo run` or `cargo run --release`.
- WASM dev helper (Windows PowerShell): `pwsh scripts/wasm-dev.ps1` (optionally `-Install`).
- Add the wasm target: `rustup target add wasm32-unknown-unknown`.
- Recommended: install `cargo-watch` and `wasm-server-runner` for rapid WASM iteration (script can install `wasm-server-runner`).

Feature flags
- Default feature set includes `debug` in `Cargo.toml`. Toggle with `--features debug`.
- Metaballs and level-loading behaviors are gated by features (`metaballs_early_exit`, `embedded_levels`, `live_levels`) — inspect `Cargo.toml` for details.

Editing Shaders
- WGSL shaders are in `assets/shaders/` (and embedded on wasm builds via `include_str!`).
- When changing shaders, rebuild the project. The wasm build embeds shader sources on wasm target to avoid relying on separate files.

Inputs & Binding
- Input mappings are defined in `assets/config/input.toml` and parsed by `src/interaction/inputmap/parse.rs`.
- To add actions or bindings, follow the TOML schema used in that file (actions, bindings, debug.bindings, virtual_axes, gesture).

Testing & Debugging
- Unit/integration tests: `cargo test` (see `tests/` for examples).
- Debug rendering: enable the `debug` feature or set `rapier_debug` in config to true; this activates the Rapier debug renderer.
- Logging: the app uses Bevy's logging; inspect console output for `CONFIG WARNING` and `CONFIG LOAD ISSUE` messages from `main.rs`.

Contributing
- Follow existing code style and module boundaries.
- Run `cargo fmt` and `cargo clippy` (project contains a `clippy.toml`) before submitting PRs.
- Describe changes to `assets/levels/` or `assets/config/` in PR description if behavior or defaults change.

### Glyph SDF Mode (Experimental)
When `sdf_shapes.glyph_mode` is enabled and the SDF atlas provides tiles named `glyph_<char>`, each spawned ball receives a glyph silhouette. Mapping is deterministic by spawn order and governed by:
- `glyph_text`: source string (characters beyond atlas fall back to analytic circle)
- `glyph_wrap`: `Repeat` | `Clamp` | `None`
- `glyph_skip_whitespace`: skips whitespace when true
Small radii below `use_circle_fallback_when_radius_lt` still render analytic circles. Missing glyphs are logged once each (target `sdf`).

// TODO(glyph): Integrate richer documentation & examples into autogenerated README script output.

---

## Troubleshooting & Notes for Developers

- WebGPU only: `src/webgpu_guard.rs` asserts presence of `navigator.gpu` on wasm; browsers lacking WebGPU will early-fail.
- Renderer backend assertion: `main.rs` asserts allowed backends on startup. If you see adapter backend mismatch errors, confirm host GPU driver/backends.
- WASM embedding: on wasm the code embeds `assets/config/game.ron` and shaders; changes to those files may be ignored at runtime unless rebuilt.

---

## License & Contact

This project is licensed under GPL-3.0-or-later (see `LICENSE`).

For reference and the hosted demo, see: https://ahammer.github.io/metalrain/

