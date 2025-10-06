# Sprint: Cross-Platform Asset Management & Shader Optimization

**Status**: Not Started  
**Priority**: High  
**Target**: Desktop & WASM Compatibility  
**Dependencies**: Current shader system, asset loading patterns

---

## Executive Summary

This sprint refactors the asset management system to follow Bevy best practices for cross-platform deployment (Desktop & WASM). The primary focus is on shaders, with particular attention to loading strategies, embedding options, and ensuring consistent behavior across platforms. The current codebase uses mixed patterns (AssetServer paths, potential embedded shaders) that need standardization.

---

## Current State Analysis

### Assets Structure

```
assets/
├── fonts/           # Currently empty or minimal
└── shaders/         # Core render pipeline shaders
    ├── background.wgsl           (Background gradient/patterns)
    ├── compositor.wgsl           (Multi-layer composition)
    ├── compute_3d_normals.wgsl   (Metaball normal computation)
    ├── compute_metaballs.wgsl    (Metaball field generation)
    └── present_fullscreen.wgsl   (Final presentation + lighting)
```

### Current Loading Patterns

**Shader Loading Examples:**

- `metaball_renderer`: Uses `AssetServer.load("shaders/compute_metaballs.wgsl")`
- `background_renderer`: `ShaderRef::Path("shaders/background.wgsl".into())`
- `game_rendering` compositor: `ShaderRef::Path("shaders/compositor.wgsl".into())`

**Asset Path Configuration:**

- `metaballs_test` demo: Manually overrides with `file_path: "../../assets"`
- Inconsistent across demos (needs workspace-relative resolution)

### Issues Identified

1. **Path Inconsistency**: Demos require manual `AssetPlugin` path overrides
2. **WASM Unproven**: No confirmation shaders load correctly via HTTP on WASM
3. **No Embedding Strategy**: Critical shaders not embedded; deployment complexity
4. **Missing Asset Meta**: No `.meta` files; potential WASM load failures
5. **No Loading State**: Shaders loaded on-demand; potential first-frame stutters
6. **Zero Hot-Reload**: `file_watcher` feature disabled for WASM compatibility

---

## Goals & Success Criteria

### Primary Goals

1. **Cross-Platform Shader Loading**: All 5 shaders load correctly on Desktop & WASM
2. **Standardized Asset Paths**: Eliminate manual `AssetPlugin` overrides in demos
3. **Embedded Shader Option**: Critical shaders optionally embedded for single-file WASM
4. **Loading Screen Support**: Infrastructure for preloading shaders before first render
5. **WASM Deployment Verified**: Shaders served correctly via HTTP; no 404s

### Success Criteria

- ✅ `trunk serve` runs without shader load errors
- ✅ Desktop builds find shaders without path hacks
- ✅ Single WASM file option (embedded shaders) available
- ✅ Shader hot-reload works on Desktop (optional feature flag)
- ✅ Zero shader-related console errors in browser DevTools

---

## Technical Approach

### Strategy 1: Standardized External Assets (Default)

**For**: Desktop development, smaller WASM initial load, hot-reload support  
**Approach**: Keep shaders as external files, fix path resolution

#### Tasks

1. **Workspace Asset Path Resolution**
   - Use `CARGO_MANIFEST_DIR` or workspace root detection
   - Remove manual `AssetPlugin` overrides from all demos
   - Test: `cargo run -p metaballs_test` finds shaders without modification

2. **WASM Asset Serving**
   - Configure `Trunk.toml` to copy `assets/` to `dist/`
   - Verify HTTP paths: `assets/shaders/background.wgsl` accessible
   - Add `.nojekyll` for GitHub Pages if deploying there

3. **Asset Meta Files** (Optional)
   - Generate `.meta` files for shaders OR insert `AssetMetaCheck::Never`
   - Document meta file generation for new shaders
   - Test WASM load behavior with/without meta files

4. **Loading State System**
   - Create `AssetCollection` for 5 core shaders using `bevy_asset_loader`
   - Implement `LoadingState` → `Ready` transition
   - Show loading screen during shader compilation (first load)

### Strategy 2: Embedded Shaders (Optional Build)

**For**: Single-file WASM, guaranteed deployment, no HTTP fetches  
**Approach**: Use `bevy_embedded_assets` or `embedded_asset!` macro

#### Tasks

1. **Workspace Feature Flag**

   ```toml
   # Cargo.toml
   [features]
   embedded_shaders = ["bevy/embedded_watcher"]
   ```

2. **Embed Critical Shaders**
   - Use `embedded_asset!(app, "shaders/compute_metaballs.wgsl")` in plugin init
   - Update `Material2d::fragment_shader()` to return `embedded://` path
   - Keep compute shaders embedded; render shaders optional

3. **Hybrid Approach**
   - Embed: `compute_metaballs.wgsl`, `compute_3d_normals.wgsl` (core logic)
   - External: `background.wgsl`, `compositor.wgsl`, `present_fullscreen.wgsl` (tweakable)
   - Rationale: Balance binary size vs. iteration speed

4. **Build Configurations**
   - Desktop Dev: External shaders + hot-reload
   - Desktop Release: External shaders (smaller binary)
   - WASM: Embedded shaders (deployment simplicity)

### Strategy 3: Hot-Reload for Desktop

**For**: Shader development, rapid iteration  
**Approach**: Re-enable `file_watcher` behind platform flag

#### Tasks

1. **Conditional Feature**

   ```toml
   [features]
   dev = ["bevy/file_watcher"]
   
   [target.'cfg(not(target_arch = "wasm32"))'.dependencies]
   bevy = { workspace = true, features = ["file_watcher"] }
   ```

2. **Document Workflow**
   - `cargo run -p metaballs_test --features dev` for hot-reload
   - Edit `present_fullscreen.wgsl` → auto-recompile on save
   - No hot-reload on WASM (browser limitation)

---

## Implementation Plan

### Phase 1: Path Standardization (Week 1)

**Sub-Sprint 1.1: Workspace Asset Resolution**

- [ ] Remove `AssetPlugin { file_path: "../../assets" }` from all demos
- [ ] Test default Bevy asset discovery (should find workspace `assets/`)
- [ ] If broken, implement custom asset path resolver using workspace detection
- [ ] Document: "Run demos from workspace root for asset discovery"

**Sub-Sprint 1.2: WASM Asset Serving**

- [ ] Create `Trunk.toml` with `[[copy]]` directive for `assets/`
- [ ] Run `trunk serve --open` and verify shaders load via DevTools Network tab
- [ ] Fix any CORS/path issues (unlikely with relative paths)
- [ ] Add deployment guide: "Upload `dist/` folder; keep `assets/` structure"

**Validation**

```bash
# Desktop
cd ball_matcher
cargo run -p metaballs_test  # Should find shaders

# WASM
trunk serve --port 8080
# Open http://localhost:8080, check console for shader load errors
```

### Phase 2: Shader Embedding (Week 2)

**Sub-Sprint 2.1: Compute Shader Embedding**

- [ ] Add `bevy_embedded_assets = "0.11"` to workspace deps (or use `embedded_asset!`)
- [ ] In `metaball_renderer/src/lib.rs`:

  ```rust
  use bevy::asset::embedded_asset;
  
  impl Plugin for MetaballRendererPlugin {
      fn build(&self, app: &mut App) {
          embedded_asset!(app, "../../assets/shaders/compute_metaballs.wgsl");
          embedded_asset!(app, "../../assets/shaders/compute_3d_normals.wgsl");
          // ... rest of plugin
      }
  }
  ```

- [ ] Update compute pipeline to use `embedded://ball_matcher/shaders/compute_metaballs.wgsl`
- [ ] Test: Build WASM, verify no HTTP requests for compute shaders

**Sub-Sprint 2.2: Material Shader Embedding**

- [ ] Embed `background.wgsl` in `background_renderer`:

  ```rust
  impl Material2d for BackgroundMaterial {
      fn fragment_shader() -> ShaderRef {
          ShaderRef::Path("embedded://ball_matcher/shaders/background.wgsl".into())
      }
  }
  ```

- [ ] Same for `compositor.wgsl` in `game_rendering`
- [ ] Test: `wasm-pack build --target web`, inspect binary size increase

**Sub-Sprint 2.3: Presentation Shader Handling**

- [ ] Decision: Keep `present_fullscreen.wgsl` external (large, tweakable)
- [ ] OR: Embed for production, external for dev (feature-gated)
- [ ] Implement hybrid loading (fallback to embedded if external fails)

**Validation**

```bash
# Build and inspect
wasm-pack build --release
ls -lh target/wasm32-unknown-unknown/release/*.wasm
# Should see size increase (embedded shaders ~10-20KB total)

# Functional test
python3 -m http.server 8080 --directory target/wasm-bindgen
# No shader 404s in browser console
```

### Phase 3: Loading Infrastructure (Week 3)

**Sub-Sprint 3.1: Shader Asset Collection**

- [ ] Add `bevy_asset_loader = "0.19"` to workspace deps
- [ ] Create `ShaderAssets` resource:

  ```rust
  #[derive(AssetCollection, Resource)]
  struct ShaderAssets {
      #[asset(path = "shaders/background.wgsl")]
      background: Handle<Shader>,
      #[asset(path = "shaders/compositor.wgsl")]
      compositor: Handle<Shader>,
      // ... all 5 shaders
  }
  ```

- [ ] Register loading state: `LoadingState → Ready`

**Sub-Sprint 3.2: Loading Screen**

- [ ] Create minimal loading screen (logo + progress bar)
- [ ] Show during shader compilation (if non-embedded)
- [ ] Transition to game when all shaders ready
- [ ] WASM: Show "Downloading assets..." if HTTP fetch in progress

**Sub-Sprint 3.3: Error Handling**

- [ ] Detect shader load failures (404, compile errors)
- [ ] Display user-friendly error: "Failed to load rendering shaders"
- [ ] Fallback: Solid color rendering if shaders missing (graceful degradation)

**Validation**

```bash
# Simulate slow network
chrome --user-data-dir=/tmp/chrome --throttling.downloadThroughputKbps=50
# Loading screen should appear, then fade to game
```

### Phase 4: Hot-Reload & Dev Experience (Week 4)

**Sub-Sprint 4.1: Desktop Hot-Reload**

- [ ] Add `dev` feature flag (enables `file_watcher`)
- [ ] Test: Edit `present_fullscreen.wgsl`, save, see live update
- [ ] Document in `README.md`: "Run with `--features dev` for hot-reload"

**Sub-Sprint 4.2: Shader Validation**

- [ ] Pre-commit hook: Validate WGSL syntax with `naga` CLI
- [ ] CI: Compile all shaders, catch errors before merge
- [ ] Linting: Check for common WGSL mistakes (uninitialized vars, etc.)

**Sub-Sprint 4.3: Documentation**

- [ ] Write `docs/SHADERS.md`:
  - Shader descriptions (what each does)
  - Uniform layouts (what data each expects)
  - Texture bindings
  - Editing guidelines (precision, WASM constraints)
- [ ] Update `README.md` with asset deployment instructions

**Validation**

```bash
# Hot-reload test
cargo run -p metaballs_test --features dev
# Edit shader → Ctrl+S → See instant update

# Shader validation
naga assets/shaders/background.wgsl
# Should output IR or validation errors
```

---

## Shader-Specific Considerations

### 1. `background.wgsl`

**Current**: Linear/radial gradients, animated patterns  
**Loading**: Material2d fragment shader (external or embedded)  
**Embedding Priority**: Low (aesthetic, frequently tweaked)  
**WASM Concerns**: None (small, simple)

**Actions**:

- Keep external by default
- Embed in release WASM builds
- Test: Verify gradients render identically on desktop/web

### 2. `compositor.wgsl`

**Current**: 5-layer blending (normal/additive/multiply)  
**Loading**: Material2d fragment shader, samples 5 textures  
**Embedding Priority**: Medium (core to render pipeline)  
**WASM Concerns**: Texture sampling (ensure samplers bound correctly)

**Actions**:

- Embed in all builds (critical for rendering)
- Verify texture bindings match across platforms
- Test: All blend modes (normal/additive/multiply) work on WASM

### 3. `compute_metaballs.wgsl`

**Current**: Field computation, gradient calculation, clustering  
**Loading**: Compute shader (manually loaded in pipeline)  
**Embedding Priority**: **HIGH** (core simulation logic)  
**WASM Concerns**: Compute shader support (WebGPU required)

**Actions**:

- **EMBED ALWAYS** (non-negotiable for reliability)
- Use `embedded_asset!` in `metaball_renderer` plugin
- Test: Verify compute dispatch works on WASM WebGPU
- Fallback: If WebGPU unavailable, show error (no CPU fallback planned)

### 4. `compute_3d_normals.wgsl`

**Current**: Normal generation from metaball field  
**Loading**: Compute shader  
**Embedding Priority**: **HIGH** (paired with compute_metaballs)  
**WASM Concerns**: Same as compute_metaballs

**Actions**:

- **EMBED ALWAYS**
- Same embedding strategy as `compute_metaballs.wgsl`
- Test: Normal texture output matches desktop

### 5. `present_fullscreen.wgsl`

**Current**: Lighting, shadows, edge AA, final presentation  
**Loading**: Fragment shader for metaball presentation  
**Embedding Priority**: Medium-High (complex, but stable)  
**WASM Concerns**: Float precision (use `highp` or ensure f32 consistency)

**Actions**:

- Embed in WASM release builds
- Keep external for desktop dev (frequent lighting tweaks)
- Test: Shadows and lighting match desktop/web
- Profile: Check fragment shader performance on lower-end WASM (M1 Air, etc.)

---

## Testing Strategy

### Unit Tests

- Asset path resolution (mock `AssetServer`)
- Shader handle validity (ensure non-default handles)
- Embedding verification (check `embedded://` path resolution)

### Integration Tests

- Desktop: All demos run without shader errors
- WASM: `trunk serve` + manual browser test
- Cross-browser: Chrome, Firefox, Safari (WebGPU support varies)

### Visual Regression

- Capture frames from desktop/WASM builds
- Compare pixel-perfect (shaders should produce identical output)
- Tools: `bevy_framepace` + screenshot capture

### Performance

- Shader compile time (first load)
- WASM binary size (embedded vs external)
- Runtime shader overhead (should be zero after compile)

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| WASM WebGPU unavailable | Critical (no compute shaders) | Feature detection + error message |
| Embedded shaders bloat WASM | Medium (larger download) | Hybrid approach: embed only critical |
| Hot-reload conflicts with WASM | Low (dev-only feature) | Feature-gate `file_watcher` for native |
| Shader compile errors on WASM | High (black screen) | Pre-validation with `naga` in CI |
| Path resolution breaks demos | Medium (dev friction) | Extensive testing + fallback to `../../assets` |

---

## Dependencies

### Crates to Add

- `bevy_embedded_assets = "0.11"` (optional, or use Bevy's `embedded_asset!`)
- `bevy_asset_loader = "0.19"` (loading state management)
- `naga` (shader validation, dev dependency)

### External Tools

- `trunk` (WASM dev server, already in use)
- `wasm-bindgen` (WASM packaging)
- `http-server` or Python's `http.server` (testing served assets)

---

## Rollout Plan

### Week 1: Stabilization

- Fix asset paths for all demos (desktop)
- Verify `trunk serve` works with external shaders
- Merge to `clean_start` branch

### Week 2: Embedding

- Implement embedded shader loading
- Create WASM release build with embedded shaders
- Test deployment to GitHub Pages or itch.io

### Week 3: Polish

- Add loading screen with shader preloading
- Implement error handling (missing shaders)
- Document asset deployment process

### Week 4: Dev Experience

- Enable hot-reload for desktop
- Add shader validation to CI
- Write `SHADERS.md` documentation

---

## Definition of Done

- [ ] All demos run on desktop without manual asset path config
- [ ] `trunk serve` displays game with all shaders loading via HTTP
- [ ] WASM release build option with all shaders embedded (<500KB binary)
- [ ] Loading screen shown during shader compilation
- [ ] Hot-reload works on desktop with `--features dev`
- [ ] CI validates all shader syntax with `naga`
- [ ] Documentation: `SHADERS.md` describes all 5 shaders + editing workflow
- [ ] Zero shader-related errors in browser console (Chrome/Firefox/Safari)
- [ ] Visual parity confirmed: Desktop screenshots match WASM screenshots

---

## Future Enhancements (Post-Sprint)

1. **Shader Variants**: Generate multiple shader versions (low/medium/high quality)
2. **Texture Compression**: Use Basis Universal for any future texture assets
3. **Audio Assets**: Apply same embedding strategy to sound effects/music
4. **Font Embedding**: Embed default UI font for WASM single-file builds
5. **Asset Bundles**: Pack shaders into compressed archives for faster HTTP fetch
6. **Progressive Loading**: Load non-critical shaders after initial render (e.g., effects layer)
7. **Shader Debugging**: Integrate RenderDoc or browser shader inspectors

---

## References

- [Bevy Cheatbook: Asset Loading](https://bevy-cheatbook.github.io/assets/assetserver.html)
- [Bevy Cheatbook: WASM Deployment](https://bevy-cheatbook.github.io/platforms/wasm/webpage.html)
- [bevy_embedded_assets Docs](https://docs.rs/bevy_embedded_assets)
- [bevy_asset_loader GitHub](https://github.com/NiklasEi/bevy_asset_loader)
- [Bevy Example: Embedded Asset](https://bevy.org/examples/assets/embedded-asset/)
- [WGSL Specification](https://www.w3.org/TR/WGSL/)
- [Naga Shader Validator](https://github.com/gfx-rs/naga)

---

## Notes

- Current `Cargo.toml` has `file_watcher` feature already **removed** for WASM compatibility (good!)
- Workspace uses Bevy 0.16.1 with `webgpu` feature (WebGPU support confirmed)
- `metaballs_test` demo already overrides asset path; this pattern needs elimination
- No fonts currently in `assets/fonts/` (potential future work for UI)
- Shaders are well-documented inline (preserve this in any refactor)

---

**Sprint Owner**: TBD  
**Reviewers**: Graphics Team, WASM Deployment Lead  
**Estimated Effort**: 4 weeks (1 week per phase)  
**Actual Effort**: TBD (track in sprint retrospective)
