# Sprint: Cross-Platform Asset Management & Shader Optimization

**Status**: Not Started  
**Priority**: High  
**Target**: Desktop & WASM Compatibility  
**Dependencies**: Current shader system, asset loading patterns

---

## Executive Summary

This sprint refactors the asset management system to follow Bevy best practices for cross-platform deployment (Desktop & WASM). The primary focus is on shaders, with particular attention to loading strategies, embedding options, and ensuring consistent behavior across platforms. The current codebase uses mixed patterns (AssetServer paths, potential embedded shaders) that need standardization.

---

## High & Medium Priority Architectural Constraints (Added)

These constraints codify architectural guardrails aligned with the philosophy of the `game_assets` crate (single authoritative asset lifecycle, path abstraction, future-proof embedding/hot-reload). All implementation tasks must respect them.

### High Priority

1. Centralized Embedding Logic
   - All shader (and future font) embedding resides inside `game_assets` (via `GameAssetsPlugin`), not scattered across renderer or demo crates.
   - Renderer crates must only consume typed handles from `GameAssets` and never call `embedded_asset!` directly.
2. Feature Flags & Policy Enum Established Early
   - Introduce feature flags controlling embedding breadth, hot-reload, and preload state integration before other phases proceed.
   - Provide a `ShaderEmbeddingPolicy` (or equivalent) applied uniformly during plugin build.

### Medium Priority

1. Asset Root Auto-Detection
   - Preserve `AssetRootMode` but add an `auto` detection path so demos/games can call a single configure function without manually specifying relative paths.
   - Do NOT remove existing mode variants; maintain explicit override capability.
2. Readiness Signaling (Not UI)
   - `game_assets` exposes a readiness resource/event (e.g., `GameAssetsReady` or state enum) instead of directly owning any loading screen visuals.
   - UI/UX layers decide how to render loading feedback.

### Additional Supporting Practices

1. Hot-Reload Encapsulation
   - `hot_reload` feature flag enables Bevy `file_watcher` only on non-wasm targets inside the crate.
2. Validation Hooks
   - Provide optional internal test / helper to assert shader path existence; CI can invoke it.
3. Hybrid Embedding Policy
   - Support policies: `None`, `CoreOnly` (compute shaders), `All`. Selection is compile-time (features) or plugin parameter—not ad hoc logic in downstream crates.

---

---

## Current State Analysis

### Assets Structure

```text
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
   - Implement `configure_auto()` in `game_assets` (will later be used in Phase 1)
   - Replace per-demo manual `AssetPlugin` path logic with `configure_auto()`; retain explicit functions for overrides
   - Test: `cargo run -p metaballs_test` finds shaders without manual overrides

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
**Approach**: Centralize embedding inside `game_assets` plugin; downstream crates remain agnostic.

#### Task List

1. **Feature Flags (Centralized in `game_assets`)**

   Proposed additions to `crates/game_assets/Cargo.toml` (names may adjust during implementation):

   ```toml
   [features]
   # Embed only critical compute shaders
   embedded_core_shaders = []
   # Embed all shaders (implies core)
   embedded_all_shaders = ["embedded_core_shaders"]
   # Enable file watching for desktop hot-reload
   hot_reload = []
   # Integrate preload state machine (bevy_asset_loader)
   preload_states = []
   ```

   Mutually exclusive at run-time: logic will prefer `embedded_all_shaders` > `embedded_core_shaders` > none.

2. **Embed Critical Shaders (Inside Plugin)**
   - In `GameAssetsPlugin::build`, conditionally invoke `embedded_asset!` for compute shaders when `embedded_core_shaders` or `embedded_all_shaders` is active.
   - Fragment/material shaders embedded only if `embedded_all_shaders`.
   - Downstream materials should continue using `ShaderRef::Path("shaders/...".into())`; Bevy resolves embedded overrides transparently.

3. **Hybrid Approach Encoded as Policy**
   - `None`: All external (dev iteration speed & hot-reload)
   - `CoreOnly`: Embed `compute_metaballs.wgsl`, `compute_3d_normals.wgsl`
   - `All`: Embed all five shaders
   - Policy selected via feature flags or explicit `GameAssetsPlugin { embedding: ShaderEmbeddingPolicy::CoreOnly, .. }` initialization.

4. **Recommended Build Configurations**
   - Desktop Dev: `hot_reload` + no embedding features
   - Desktop Release: No embedding OR `embedded_core_shaders` if startup perf critical
   - WASM Debug: External for iteration (faster rebuilds)
   - WASM Release: `embedded_core_shaders` or `embedded_all_shaders` depending on reliability vs size

### Strategy 3: Hot-Reload for Desktop

**For**: Shader development, rapid iteration  
**Approach**: Re-enable `file_watcher` behind platform flag

#### Tasks

1. **Conditional Feature (Revised)**

```toml
[features]
hot_reload = []

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
bevy = { workspace = true, features = ["file_watcher"] }
```

1. **Document Workflow**
   - `cargo run -p metaballs_test --features dev` for hot-reload
   - Edit `present_fullscreen.wgsl` → auto-recompile on save
   - No hot-reload on WASM (browser limitation)

---

## Implementation Plan

### Phase 1: Path Standardization & Auto-Detection (Week 1)

#### Sub-Sprint 1.1: Asset Root Auto-Detection

- [ ] Add `configure_auto(app)` in `game_assets` that detects `AssetRootMode` (heuristic: probe relative paths for `assets/shaders`)
- [ ] Retain existing explicit `configure_demo/game_crate/workspace_root` APIs (no breaking change)
- [ ] Refactor demos to call `configure_auto()` (remove local manual `AssetPlugin` usage in demos if present)
- [ ] Document: Auto-detection fallback precedence & how to override explicitly

#### Sub-Sprint 1.2: WASM Asset Serving

- [ ] Create `Trunk.toml` with `[[copy]]` directive for `assets/`
- [ ] Run `trunk serve --open` and verify shaders load via DevTools Network tab
- [ ] Fix any CORS/path issues (unlikely with relative paths)
- [ ] Add deployment guide: "Upload `dist/` folder; keep `assets/` structure"

#### Validation

```bash
# Desktop
cd ball_matcher
cargo run -p metaballs_test  # Should find shaders

# WASM
trunk serve --port 8080
# Open http://localhost:8080, check console for shader load errors
```

### Phase 2: Centralized Shader Embedding (Week 2)

#### Sub-Sprint 2.1: Implement Feature Flags & Policy

- [ ] Add feature flags (`embedded_core_shaders`, `embedded_all_shaders`, `hot_reload`, `preload_states`) to `game_assets`.
- [ ] Introduce `ShaderEmbeddingPolicy` enum & add to `GameAssetsPlugin` (with default resolution logic from active features).
- [ ] Unit test: feature precedence (`all` overrides `core`).

#### Sub-Sprint 2.2: Embed via GameAssetsPlugin

- [ ] Add conditional `embedded_asset!` calls in `GameAssetsPlugin::build` based on resolved policy.
- [ ] Remove / prevent any embedding calls from renderer crates (audit & adjust if necessary).
- [ ] Confirm downstream shader loads still use plain `AssetServer.load("shaders/...")` or `ShaderRef::Path`.

#### Sub-Sprint 2.3: Policy-Based Build Matrix

- [ ] Add documentation table mapping build targets to recommended feature flags.
- [ ] WASM Release build test: ensure compute shaders do not trigger network requests when embedded.
- [ ] Collect binary size deltas for `core` vs `all` embedding (record in doc).

Note: Old per-crate embedding subtasks removed; superseded by centralized plugin policy.

#### Validation (Phase 2)

```bash
# Build and inspect
wasm-pack build --release
ls -lh target/wasm32-unknown-unknown/release/*.wasm
# Should see size increase (embedded shaders ~10-20KB total)

# Functional test
python3 -m http.server 8080 --directory target/wasm-bindgen
# No shader 404s in browser console
```

### Phase 3: Loading Infrastructure & Readiness Signaling (Week 3)

#### Sub-Sprint 3.1: Optional Preload State (Feature-Gated)

- [ ] If `preload_states` feature enabled: integrate `bevy_asset_loader` internally in `game_assets` to populate `GameAssets` after load barrier.
- [ ] Provide `GameAssetsState` (enum: `Loading`, `Ready`) or fire `GameAssetsReady` event.
- [ ] Export helper: `fn all_shaders_loaded(&Assets<Shader>, &ShaderAssets) -> bool` for consumers.

#### Sub-Sprint 3.2: External UI Integration Guidance

- [ ] Document how downstream crates observe readiness (no UI code inside `game_assets`).
- [ ] Provide example snippet in docs demonstrating a loading screen system living outside the asset crate.

#### Sub-Sprint 3.3: Error Handling Facilities

- [ ] Add optional debug assertion / log if any shader handle is still `Weak` after timeout.
- [ ] Document recommended fallback strategies (kept outside asset crate to preserve separation of concerns).

#### Validation (Phase 3)

```bash
# Simulate slow network
chrome --user-data-dir=/tmp/chrome --throttling.downloadThroughputKbps=50
# Loading screen should appear, then fade to game
```

### Phase 4: Hot-Reload & Dev Experience (Week 4)

#### Sub-Sprint 4.1: Desktop Hot-Reload (Centralized)

- [ ] Implement `hot_reload` feature inside `game_assets` enabling Bevy `file_watcher` only for non-wasm.
- [ ] Document usage: `cargo run -p metaballs_test --features game_assets/hot_reload` (example pattern).
- [ ] Validate shader edits propagate without restart.

#### Sub-Sprint 4.2: Shader Validation

- [ ] Pre-commit hook: Validate WGSL syntax with `naga` CLI
- [ ] CI: Compile all shaders, catch errors before merge
- [ ] Linting: Check for common WGSL mistakes (uninitialized vars, etc.)

#### Sub-Sprint 4.3: Documentation

- [ ] Write `docs/SHADERS.md`:
  - Shader descriptions (what each does)
  - Uniform layouts (what data each expects)
  - Texture bindings
  - Editing guidelines (precision, WASM constraints)
- [ ] Update `README.md` with asset deployment instructions

#### Validation (Phase 4)

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
| Embedded shaders bloat WASM | Medium (larger download) | Policy enum + size measurement docs |
| Hot-reload conflicts with WASM | Low (dev-only feature) | `hot_reload` feature disabled on wasm |
| Shader compile errors on WASM | High (black screen) | Pre-validation with `naga` + readiness gating |
| Path resolution breaks demos | Medium (dev friction) | Auto-detect with explicit override retention |
| Fragmented embedding across crates | High (inconsistent behavior) | Centralize embedding in `game_assets` |
| Loading UI logic coupled to asset crate | Medium (reduced reuse) | Expose readiness only, delegate UI |

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
- [ ] WASM release build option with core or all shaders embedded (target binary size tracked & documented)
- [ ] Readiness signaling resource/event consumed by external loading screen (example provided)
- [ ] Hot-reload works on desktop with `game_assets` `hot_reload` feature
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
8. **Dynamic Policy Override**: Runtime selection of embedding policy for dev profiling
9. **Centralized Path Integrity Test**: `cargo test -p game_assets -- --include-asset-audit`

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
