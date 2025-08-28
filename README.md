# Ball Matcher

Experimental clustering & popping prototype built with Bevy.

## Visual / Interaction Features

- Metaball rendering with multiple foreground / background shader modes.
- Cluster persistence & popping (growth / hold / shrink lifecycle).
- Ball state system (Enabled / Disabled) with secondary palette + smooth tween; disabled balls isolated (non-merging).

## Ball State & Dual Palette

Purpose: Instantly distinguish actionable (poppable) clusters from non-actionable ones while preserving merging behavior only for enabled clusters. Disabled (non-poppable) balls use a secondary fixed palette variant and are visually isolated (their metaball fields do not merge) without any WGSL interface changes.

### States
- Enabled: Cluster meets BOTH thresholds (`min_ball_count` AND `min_total_area`) -> shares a single color slot (classic merging).
- Disabled: Cluster fails one or both thresholds -> each ball receives its own unique slot (no field accumulation / no merging look).

### Components / Systems
- `BallState { enabled: bool, last_change: f32 }` lazily inserted/updated per ball after clustering (`compute_clusters`) in `BallStateUpdateSet`.
- `BallStatePlugin` orders `update_ball_states` after `compute_clusters` inside `PostPhysicsAdjustSet`.
- Rendering system assigns:
  1. Slots for enabled clusters (one per enabled cluster).
  2. Unique per-ball slots for disabled clusters.
  3. Overflow fallback (if unique slots exceed `MAX_CLUSTERS`): disabled balls grouped by base palette index (merging may reappear) with a one-time log.

### Palette / Colors
Primary: `BASE_COLORS`
Secondary: `SECONDARY_COLORS` (artist-authored alternatives; not algorithmic darkening)
Helper: `secondary_color_for_index(i)` keeps index alignment.

### Tweening
Linear interpolation in linear color space between enabled and disabled variants per represented state:
- On transition (enabled flag flip) `last_change` updated.
- Factor `t = clamp((now - last_change)/tween_duration, 0..1)`.
- If `enabled` now true: `lerp(disabled_color -> enabled_color, t)`.
- Else: `lerp(enabled_color -> disabled_color, t)`.

No extra buffers; color computed while packing uniform cluster color array.

### Config
`GameConfig.ball_state.tween_duration` (default 0.35s, clamped & warned if <= 0 -> treated as 0.01). Secondary palette is fixed (no config fields yet).

Example (optional explicit override in `game.ron`):
```ron
ball_state: (
    tween_duration: 0.35,
)
```

### Performance Considerations
- O(cluster_count + ball_count) per frame; uses pre-sized hash maps (capacity ball_count).
- No per-frame heap growth beyond temporary maps.
- Slot overflow handled gracefully; single INFO log via `OverflowLogged` resource.

### Logging
- INFO (target = "ball_state") on first Disabled insert and on state transitions (can be later throttled if overly chatty).
- INFO (target = "metaballs") on overflow fallback trigger (once).

### Edge Cases
- 0 clusters: system early returns (no inserts).
- Rapid threshold oscillation restarts tween each flip.
- Extremely small tween duration behaves like instant color swap.

### Testing
Unit/integration tests:
- `lerp_color_midpoint` accuracy.
- Secondary palette mapping difference.
- Slot allocation test: Disabled (unique slots) -> Enabled (shared slot) transition.

### Manual Visual Verification Checklist
- Disabled balls appear discrete (no merging halos) and show secondary hues.
- Enabled clusters continue to merge identically to legacy behavior.
- Smooth cross-fade when clusters become enabled/disabled.
- Overflow scenario (force many disabled balls) still renders (fallback grouping) with log.

## Building / Running

Standard Bevy workflow (ensure Rust toolchain installed):

```
cargo run
```

(For WASM builds ensure suitable target & embedding path remain up to date if config layout changes.)

## Rendering Policy
This project enforces a WebGPU-only rendering path on the web and restricts native builds to modern explicit backends (Vulkan / Metal / DX12). OpenGL / WebGL backends are intentionally not compiled or requested. Rationale:
- Consistent shader & feature parity (WGSL-first pipeline).
- Reduced maintenance surface (no dual GLSL/WGSL or downlevel limits).
- Explicit failure on unsupported browsers instead of silent GL fallback.
Configuration summary:
- `Cargo.toml` uses target-scoped `wgpu` dependencies:
  - wasm32: `features = ["webgpu","wgsl"]`
  - native: `features = ["wgsl","vulkan","metal","dx12"]`
- Renderer creation masks:
  - wasm32: `Backends::BROWSER_WEBGPU`
  - native: `Backends::{VULKAN|METAL|DX12}`
- Early WASM guard panics if `navigator.gpu` is absent.
- Startup assertion confirms chosen adapter backend matches policy.
Unsupported: Browsers or CI environments lacking WebGPU (wasm) or a modern native backend. Do not reintroduce WebGL/GL for fallback.

## License

GPL-3.0-or-later

## WASM (WebGPU) Development Loop

This project supports a fast WebGPU-only WASM workflow without extra tooling like trunk or wasm-pack. (Built with accessibility considerations; please still verify with your own tools.)

> Note: We rely exclusively on Bevy's internal wgpu; no direct `wgpu` dependency or WebGL fallback is included. Runtime guards assert the backend is BrowserWebGpu on wasm and Vulkan/Metal/DX12 on native.

### Prerequisites
- Rust toolchain (stable)
- Modern browser with WebGPU (Chrome 113+, Edge, Firefox Nightly (flag), Safari Technology Preview)
- No WebGL fallback is included—unsupported browsers will fail fast.

### One-Time Setup
```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-server-runner
# Optional for iterative rebuilds
cargo install cargo-watch
```

### Cargo Aliases
```powershell
# Debug
cargo wasm-build
cargo wasm-run          # serves via wasm-server-runner
# Release
cargo wasm-build-release
cargo wasm-run-release
```

### PowerShell Helper Script
```powershell
# First time (ensures target + tools)
pwsh scripts/wasm-dev.ps1 -Install
# Subsequent debug sessions (watch mode auto if cargo-watch installed)
pwsh scripts/wasm-dev.ps1
# Optimized build (single run)
pwsh scripts/wasm-dev.ps1 -Release
```

Script behavior:
- Validates `web/index.html`
- Uses `wasm-server-runner` as cargo runner (see `.cargo/config.toml`)
- Watches `src/`, `assets/`, and `web/` when `cargo-watch` is present
- Prints a clear WebGPU requirement notice

### Output Artifacts
Build artifacts appear under:
```
target/wasm32-unknown-unknown/debug/   (or release/)
```
Served JS / WASM modules are referenced by `web/index.html`.

### Troubleshooting
| Symptom | Cause | Action |
|---------|-------|--------|
| Browser console: `navigator.gpu undefined` | WebGPU unsupported | Use a supported browser / enable flag |
| Script says cargo-watch missing | Optional tool not installed | `cargo install cargo-watch` |
| Port conflict | Another runner instance active | Stop prior process / change port (`$env:WASM_SERVER_RUNNER_PORT`) |
| Stale build after edit (no watch) | Not using watch mode | Install cargo-watch or rerun script |

Accessibility note: Plain-language, high-contrast instructions; still review with an auditing tool (e.g., Accessibility Insights).
