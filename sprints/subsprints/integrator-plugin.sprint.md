# Subsprint: Scaffold Integration Plugin ("crates/scaffold" crate)

## Intent

Create a minimal, opinionated `scaffold` crate that demos (and later the main `game` crate) can depend on to instantly wire up the full rendering + physics + assets + input + diagnostics stack with sensible defaults. Demos should typically only call:

```rust
app.add_plugins(ScaffoldIntegrationPlugin::default())
    // + demo specific systems/resources
```

## Outcomes / Definition of Done

- New crate `crates/scaffold_integration` added to workspace members.
- Provides `ScaffoldIntegrationPlugin` (default-config, zero required params) that:
  - Configures demo asset root via `game_assets::configure_demo`.
  - Adds: `DefaultPlugins` (with appropriate asset folder settings), `GameAssetsPlugin`, `GameCorePlugin`, `GamePhysicsPlugin`, `GameRenderingPlugin`, `BackgroundRendererPlugin`, `MetaballRendererPlugin` (with hardcoded world bounds -256..256), `WidgetRendererPlugin`, `EventCorePlugin`, diagnostics (frame time), optional Rapier debug (behind a feature flag or togglable resource if needed later — initial version may always enable or leave off; choose simplest now).
  - Inserts baseline resources (render surface base resolution 1280x720, metaball texture 512x512, world bounds centered at 0 sized 512, zero or mild gravity, default background config).
  - Spawns: camera, physics arena walls, HUD.
  - Registers standard input systems (layer toggles, exposure adjust, camera zoom/reset, pause physics, gravity adjust, background cycle, metaball mode cycle, HUD toggle, exit on escape).
- Refactor ONE demo (`compositor_test`) to use the plugin with minimal surface area (keeps only unique systems & resources).
- Documentation: short README section (in crate) describing purpose + usage + extension points.
- No introduction of circular deps; layering preserved.
- CI/build: `cargo build --all` and `cargo test --all` remain green; no new clippy warnings.

## Non-Goals (This Subsprint)

- No advanced configuration builder (defer until real variation pressure).
- No preset profiles (e.g. physics-only, rendering-only).
- No dynamic plugin enable/disable UI overlay (future enhancement).
- No WASM-specific conditional logic beyond what existing crates already handle.

## Architectural Constraints / Principles Applied

- Single aggregation point; does NOT define new domain components (kept inside existing crates).
- All defaults bias toward the compositor + metaballs pipeline being visible immediately.
- Avoid leaking renderer internals into demos; demos interact via their own systems + events.
- Minimal public API: primarily just the `CoreIntegrationPlugin` and (optionally) a small helper module re-exporting standard system labels (if needed for ordering overrides later).

## Task Breakdown

1. Scaffolding
   - [ ] Add crate directory + `Cargo.toml` with dependencies on required internal crates + `bevy`.
   - [ ] Update root `Cargo.toml` workspace members.
   - [ ] Create `src/lib.rs` plus minimal module files: `systems/` (camera, arena, input, hud), `resources.rs` (HUD visibility, demo state), `mod.rs` for systems if needed.

2. Core Plugin Implementation
   - [ ] Implement `ScaffoldIntegrationPlugin` (derive `Default` with `demo_name: "Unnamed Demo"`).
   - [ ] In `build`, call `configure_demo` (assets) before adding project plugins.
   - [ ] Insert render + metaball + background + physics + diagnostics resources (hardcoded defaults).
   - [ ] Add plugin bundle.
   - [ ] Add startup systems: spawn camera, spawn arena walls, spawn HUD.
   - [ ] Add update systems: input hub, HUD update, exit on escape.
   - [ ] (Optional) Gate Rapier debug with a simple const or feature.

3. Standard Systems
   - [ ] `spawn_game_camera` — single 2D/orthographic camera aligned with compositor expectations.
   - [ ] `spawn_physics_arena` — four static colliders forming bounds at ±256.
   - [ ] `spawn_performance_hud` + `update_performance_hud` — show FPS, entity count, toggles status.
   - [ ] `handle_universal_inputs` — consolidates all keybindings (document keys in README).
   - [ ] `exit_on_escape` (reuse or reimplement lightweight version if not centrally exposed).

4. Demo Refactor (Compositor Test)
   - [ ] Replace manual plugin assembly with `ScaffoldIntegrationPlugin::default()`.
   - [ ] Keep only unique resource inits + systems (burst forces, wall pulses, effect animation, spawning patterns).
   - [ ] Verify functionality parity: layers toggle, metaballs visible, physics arena intact.

5. Documentation
   - [ ] Add `crates/core_integration/README.md` with: purpose, quick start snippet, defaults table, extension points, roadmap bullets (config builder, presets, dynamic toggles overlay).
   - [ ] Mention in root `design-doc.md` or `north-star-structure.md` (minimal note) if required (optional for now; can defer).

6. Quality Gates
   - [ ] Run `cargo build --all`.
   - [ ] Run `cargo test --all` (ensure no regression; add a trivial unit test for plugin compile if desired).
   - [ ] Run `cargo clippy --all -- -D warnings` (resolve new warnings).
   - [ ] `cargo fmt --all`.

7. Validation / Manual QA
   - [ ] Run refactored `compositor_test` (native) to confirm baseline visuals & controls.
   - [ ] Optionally run WASM (if existing script) to ensure no missing features in browser.

## Key Defaults (Hardcoded)

| Concern | Default |
|---------|---------|
| Base Resolution | 1280 x 720 |
| Metaball Texture | 512 x 512 |
| World Bounds (Metaballs) | Rect center (0,0) size (512,512) → -256..256 |
| Physics Arena Walls | Square at ±256 (thickness small constant) |
| Gravity | (0.0, 0.0) (adjustable via arrow keys) |
| Background | `BackgroundConfig::default()` |
| HUD Visible | true |
| Rapier Debug | Enabled by default (no feature flags in initial version) |

## Input Map (Document in README)

`1-5` layer toggles, `[`/`]` exposure, `-`/`=` zoom, `R` reset camera, `Space` shake, `P` pause physics, arrows adjust gravity, `B` cycle background, `F1` HUD toggle, `Esc` exit.

## Risks / Mitigations

- Risk: Over-including unneeded plugins → Slower startup. Mitigation: Start lean; add toggles only when real need appears.
- Risk: Demos needing variant configs soon. Mitigation: Add simple opt-in config builder in follow-up subsprint.
- Risk: Input collisions with demo-specific keys. Mitigation: Namespace future demo keys; allow disabling universal input system (future feature).

## Additional Notes (New Decisions)

- Chosen name: `crates/scaffold` to emphasize immediate wiring of subsystems; avoids implying ownership of core domain model.
- Rapier debug will be included by default in initial scaffold (no feature flag) to maximize observability; can be follow-up toggled once first abstraction pressure appears.
- No feature flags introduced in the initial version per request.

## Future Enhancements (Backlog)

- Config builder (resolution override, gravity preset, enable/disable subsystems).
- Preset profiles: `physics_only()`, `render_only()`, `minimal()`.
- Dynamic on-screen controls legend (auto-generated from active bindings).
- Runtime layer introspection panel.
- Feature-gated native-only improvements (file watching, richer diagnostics).

## Acceptance Review Checklist

- [ ] Crate compiles and exports plugin.
- [ ] `compositor_test` runs with same visible behavior (minus intentional simplifications) using new plugin.
- [ ] No layering violations introduced.
- [ ] README present with clear usage.
- [ ] All quality gates pass (build / test / clippy / fmt).
- [ ] Keybindings operate as documented.

## Owner / Collaboration

Primary: Integration engineer (this subsprint). Hand-off path: after stabilization, migrate additional demos in parallel small PRs.

## Tracking

Link this subsprint in the parent sprint planning board and mark tasks as they land in PRs.
