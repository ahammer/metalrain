# Ball Matcher

Experimental clustering & popping prototype built with Bevy.

## Visual / Interaction Features

- Metaball rendering with multiple foreground / background shader modes.
- Cluster persistence & popping (growth / hold / shrink lifecycle).
- NEW: Cluster Highlighting (Disabled / Enabled / Active) with smooth color tweening.

## Cluster Highlighting

Purpose: Instantly communicates which clusters are:
- Disabled: Below pop size threshold (darker).
- Enabled: Reachable / can be popped (base palette color).
- Active: Currently popping (lighter) during lifecycle animation.

### Configuration (game.ron)

```ron
cluster_highlight: (
    enabled: true,
    color_tween_seconds: 0.20,
    disabled_mix: 0.5,
    active_mix: 0.5,
)
```

Field semantics:
- enabled: Master toggle. When false, rendering uses legacy base colors (variants collapse to base).
- color_tween_seconds: Linear tween duration (0.0..2.0 clamped) for transitions between state colors.
- disabled_mix: Lerp factor toward black for Disabled variant (0..1 clamped; auto-bumped +0.1 if contrast too low).
- active_mix: Lerp factor toward white for Active variant (0..1 clamped; auto-bumped +0.1 if contrast too low).

Cluster pop threshold is NOT duplicated; the size gate is always: `interactions.cluster_pop.min_ball_count`.

### Runtime Model

Per logical cluster:
- `ClusterHighlight` component tracks state machine & tween.
- State derivation:
  - Active if any ball in cluster has `PaddleLifecycle`.
  - Else Enabled if `ball_count >= min_ball_count`.
  - Else Disabled.
- Palette variants (enabled/disabled/active) precomputed once at startup (or hot-reload path if extended later) and reused.

### Performance Notes

- No extra per-ball allocations; per-frame highlight color lookup is O(cluster_count).
- Tween math only runs while `tween_t < 1.0`.
- CPU-side blended color written into existing uniform cluster color array (no shader change required).

### Testing

Pure tests include:
- Color lerp midpoint.
- Palette variant generation clamp & luminance direction.
- State decision logic (`decide_state`).
(See `src/rendering/palette/palette.rs` & `src/interaction/cluster_highlight/mod.rs` tests.)

### Logging

- `info[target=cluster_highlight]` on palette variant initialization (or when disabled).
- (debug feature only) `debug[target=cluster_highlight]` for state transitions (guarded & minimal).
- Validation warnings for out-of-range config values emitted via existing `GameConfig::validate()` (new warnings for highlight fields).

### Future Enhancements (Documented Only)

- GPU-side branching / variant arrays & per-cluster factor to skip CPU blending.
- Non-linear easing curves (ease-in-out).
- Per-color adaptive mixes to normalize perceived luminance.
- Outline / halo effect or subtle pulse instead of pure fill color shift.
- Adjustable minimum luminance delta fields (e.g., `min_luminance_delta_disabled`, `min_luminance_delta_active`).

### Manual Playtest Checklist (Developer)

- Toggle threshold size around `cluster_pop.min_ball_count` and confirm a ~200ms smooth transition (no abrupt jump).
- Click an Enabled cluster: transitions to Active variant immediately and tweens if coming from another state.
- Disabled clusters appear visibly darker; Active clusters noticeably lighter (target ~50% mix).
- Disable feature in config and confirm original visuals restored and logs reflect disabled state.
- Profiling: Ensure color update system adds < 0.2ms at representative cluster counts.

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
