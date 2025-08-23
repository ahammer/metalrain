# Ball Matcher Rebuild (Modular Workspace)

Port of the legacy Ball Matcher game into a modular, testable workspace.

## Goals
1. Functionality broken into independent crates
2. Proper acyclic dependency hierarchy (single responsibility per crate)
3. Isolated samples (visual + headless) for rapid iteration
4. Optional integrations / debug / metaballs behind feature flags

## Crate Layout (Summary)
- `crates/config` (pure data + validation)
- `crates/core` (core components, system sets, RNG seed wrapper)
- `crates/physics` (rapier setup, radial gravity, separation adjustment)
- `crates/rendering` (camera, background, circle/metaballs pipelines)
- `crates/gameplay` (ring spawn, emitter, interactions, clustering)
- `crates/metaballs` (feature `metaballs`)
- `crates/debug_tools` (feature `debug`) (pending)
- `crates/hot_reload` (feature `hot-reload`) (pending)
- `bevy_app` (primary binary wiring plugins)
- `examples/*` (focused runnable slices)

## Milestone A Visual Demos (Fast Path)

Quick commands (native):

```
# Ring spawn only (no emitter/interactions)
cargo run -p spawn_demo

# Continuous spawning (emitter enabled)
cargo run -p emitter_demo

# Interactions (drag + tap explosion; emitter per config)
cargo run -p interactions_demo

# Metaballs (once crate feature ready; falls back if feature absent)
cargo run -p bevy_app --features metaballs
```

Add feature combos:

```
# Metaballs + debug (rapier debug render always added under debug)
cargo run -p bevy_app --features "metaballs debug"
```

(Additional example crates `metaballs_demo` / `full_demo` will be added; see plan Section 16.)

## WASM Build (Basic)

```
rustup target add wasm32-unknown-unknown
cargo build -p bevy_app --target wasm32-unknown-unknown --release --features metaballs
# Serve artifacts (choose one):
python -m http.server
# or
basic-http-server .
```

For demo examples replace `-p bevy_app` with `-p spawn_demo` etc. A simple `index.html` loader (not yet included) should instantiate the `.wasm` from `target/wasm32-unknown-unknown/release/`.

## Feature Flags

| Feature | Effect |
|---------|--------|
| `metaballs` | Enables metaballs crate + shader pipeline (sets/overrides `metaballs_enabled` if needed) |
| `debug` | Enables future debug tools + unconditional rapier debug render addition |
| `hot-reload` | (Planned) Native file watcher for config |
| (none) | Minimal fast compile (circles + physics + gameplay) |

## Example Crate Behaviors

| Crate | Focus | Forced Tweaks |
|-------|-------|---------------|
| `spawn_demo` | Initial ring only | `emitter.enabled = false` |
| `emitter_demo` | Continuous spawning | `emitter.enabled = true` (bumps max_live if too low) |
| `interactions_demo` | Drag + tap explosion | Respects config; `EMITTER=0` env disables emitter |
| `bevy_app` | Aggregated current game slice | Loads full config; conditional plugin adds |

## Config Loading

Native: layered load of `assets/config/game.ron` then optional `assets/config/game.local.ron` (missing files tolerated).  
WASM: embedded `game.ron` via `include_str!`.

Validation warnings are logged (non-fatal).

## Determinism & Seeds

`RngSeed(u64)` resource seeds deterministic logic (spawning, emitter). Example crates vary the seed to avoid visually identical sequences.

## Next Steps (See plan.md Section 16)

- Add `metaballs_demo` + `full_demo` example crates
- Golden frame capture completion (Phase 7 remaining)
- Debug tools & hot reload features
- README quick start table expansion once new examples committed

## License

GPL-3.0-or-later (see root LICENSE)

</content>
