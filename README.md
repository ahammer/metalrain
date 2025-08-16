<div align="center>

# Ball Matcher

Interactive 2D Bevy (0.14) sandbox: hundreds of colorful balls spawn, bounce, separate, and optionally melt into metaball blobs – configurable via a single RON file.

[Play in your browser](https://ahammer.github.io/metalrain/) · [Developer Docs](./DEVELOPER-README.md)

</div>

## Quick Start
Prereqs: Latest stable Rust toolchain. (Windows users: install the MSVC Build Tools / C++ workload.)

Run native (desktop):
```powershell
cargo run
```

Run optimized:
```powershell
cargo run --release
```

WebAssembly build (already deployed via GitHub Pages):
```powershell
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```
Artifacts appear in `target/wasm32-unknown-unknown/release/` and are post-processed in CI with `wasm-bindgen`.

## What You See
- Elastic bouncing balls (random position, velocity, radius, color)
- Optional overlap separation & velocity damping
- Metaball shader view (toggle circle meshes off to see only blobs)
- Rapier 2D physics, with optional debug overlay

## Change the Simulation
Edit `assets/config/game.ron` – window size, gravity, bounce, spawn counts/ranges, separation behavior, interaction toggles.
Restart the binary after edits (hot-reload is a potential future enhancement).

Example snippet:
```ron
gravity: (y: -600.0),
balls: (count: 150, radius_range: (min: 5.0, max: 25.0)),
separation: (enabled: true, push_strength: 0.5),
```

## Controls / Interactions
Currently autonomous (no keyboard needed) – future interactions (explosion taps / drag forces) are configurable under `interactions` in the config.

## Key Features
- Deterministic data-driven setup (RON config)
- Modular plugin architecture (`GamePlugin` aggregates)
- Efficient rendering: shared mesh; metaballs in a single full-screen pass
- Clean separation system for visual clarity
- Browser play via GitHub Pages (WASM build pipeline included)

## Troubleshooting
| Issue | Fix |
|-------|-----|
| Editor/linker error LNK1189 on Windows | Ensure `bevy` does NOT enable `dynamic_linking` feature. |
| Nothing renders in browser | Open dev tools console; ensure `ball_matcher.js` loaded (check network tab). |
| Metaballs slow on large windows | Lower ball count or disable `metaballs_enabled`. |

## Contributing
See [Developer Docs](./DEVELOPER-README.md) for architecture, system ordering, adding plugins, and the deployment workflow.

Small PRs welcome: docs fixes, performance improvements, new optional plugins.

## License
GPL-3.0-or-later

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the [GNU GPL v3](./LICENSE) for more details.

---
Enjoy watching emergent blobs? Share a screenshot! 🟡🟣🔵
