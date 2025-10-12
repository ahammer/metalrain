# WASM Deployment Guide

This document explains how to build and deploy the physics_playground demo as a WebAssembly application.

## Quick Start

### Local Development

To run the physics_playground in your browser locally:

```powershell
# First time setup (installs wasm32 target and wasm-server-runner)
pwsh scripts/physics_playground_wasm.ps1 -Install

# Development mode with hot reload (requires cargo-watch)
pwsh scripts/physics_playground_wasm.ps1

# Release mode (optimized build)
pwsh scripts/physics_playground_wasm.ps1 -Release

# Embedded shaders mode (no network fetches for shaders)
pwsh scripts/physics_playground_wasm.ps1 -Embed
```

The script will:

1. Build the physics_playground for `wasm32-unknown-unknown` target
2. Start a local web server (typically at <http://localhost:1334>)
3. Automatically open your browser to the demo
4. Watch for file changes and rebuild (if cargo-watch is installed)

### Requirements

- **Rust toolchain** with `wasm32-unknown-unknown` target
- **wasm-server-runner** (auto-installed with `-Install` flag)
- **cargo-watch** (optional, for hot reload)
- **Modern browser** with WebGPU support:
  - Chrome ≥ 113
  - Edge (Chromium-based)
  - Firefox Nightly (with `dom.webgpu.enabled` flag)
  - Safari Technology Preview

## GitHub Pages Deployment

The project uses GitHub Actions to automatically deploy to GitHub Pages on pushes to `main`.

### Workflow Overview

The `.github/workflows/deploy.yml` workflow:

1. **Builds** `physics_playground` package for wasm32-unknown-unknown target
2. **Generates JS glue** using wasm-bindgen
3. **Optimizes** the WASM binary with wasm-opt (binaryen)
4. **Copies assets** from `assets/` to `web/assets/`
5. **Deploys** to the `gh-pages` branch

### Manual Deployment

To manually trigger a deployment:

1. Go to the repository on GitHub
2. Navigate to **Actions** → **Deploy WASM (GitHub Pages)**
3. Click **Run workflow** → **Run workflow**

### Local Production Build

To test a production build locally before deploying:

```powershell
# Build the wasm binary
cargo build --release --target wasm32-unknown-unknown --package physics_playground

# Install wasm-bindgen-cli if needed
cargo install wasm-bindgen-cli

# Generate JS glue
wasm-bindgen target/wasm32-unknown-unknown/release/physics_playground.wasm `
  --out-dir web --target web --no-typescript

# Copy assets
Copy-Item -Recurse -Force assets/* web/assets/

# Serve the web directory
# Use any static file server, e.g.:
# python -m http.server --directory web 8080
# or use wasm-server-runner as configured in the dev script
```

## Architecture

### Package Structure

- **`physics_playground`**: Demo binary showcasing physics simulation with metaball rendering
- **`game_physics`**: Physics simulation using bevy_rapier2d
- **`metaball_renderer`**: GPU-based metaball renderer
- **`game_rendering`**: Compositor pipeline
- **`game_assets`**: Centralized asset management

### WASM-Specific Considerations

1. **No File Watcher**: The workspace doesn't use Bevy's `file_watcher` feature (incompatible with WASM)
2. **Asset Loading**: Assets are served via HTTP from the `web/assets/` directory
3. **Shader Embedding**: Optional `embed_shaders` feature for deterministic shader loading without network requests
4. **WebGPU Only**: No WebGL fallback; requires WebGPU-capable browser

### Asset Configuration

The project uses `game_assets` crate for consistent asset root configuration:

- **Native**: Resolves assets relative to workspace or demo crate
- **WASM**: Assets must be in `web/assets/` directory (handled by deploy script)

## Troubleshooting

### Browser Shows "WebGPU not supported"

Ensure your browser supports WebGPU:

- Update to the latest version
- For Firefox: enable `dom.webgpu.enabled` in `about:config`
- Try Chrome or Edge as a reference

### 404 Errors for Shaders

If you see 404 errors for `.wgsl` files:

- Ensure assets were copied: `Copy-Item -Recurse assets/* web/assets/`
- Or use embedded mode: `pwsh scripts/physics_playground_wasm.ps1 -Embed`

### Build Errors

Common issues:

- **Missing target**: Run `rustup target add wasm32-unknown-unknown`
- **wasm-bindgen version mismatch**: Ensure wasm-bindgen-cli matches the version in Cargo.lock
- **Out of memory**: Try building without parallel jobs: `cargo build --target wasm32-unknown-unknown -j 1`

### Performance Issues

The WASM build is debug mode by default. For better performance:

- Use release mode: `pwsh scripts/physics_playground_wasm.ps1 -Release`
- Or manually: `cargo build --release --target wasm32-unknown-unknown --package physics_playground`

## References

- [Bevy WASM Guide](https://bevy-cheatbook.github.io/platforms/wasm.html)
- [wasm-bindgen Documentation](https://rustwasm.github.io/wasm-bindgen/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)

## Switching to Other Demos

To deploy a different demo (e.g., `metaballs_test`):

1. Update `.github/workflows/deploy.yml`:
   - Change `--package physics_playground` to `--package <demo_name>`
   - Update wasm-bindgen target from `physics_playground.wasm` to `<demo_name>.wasm`
   - Update output files in subsequent steps

2. Update `web/index.html`:
   - Change the import from `physics_playground.js` to `<demo_name>.js`
   - Update page title and loading message

3. Create a new script (optional):
   - Copy `scripts/physics_playground_wasm.ps1` to `scripts/<demo_name>_wasm.ps1`
   - Update the package name constants in the script
