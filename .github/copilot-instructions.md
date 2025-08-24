<!--
Ball Matcher - Copilot Project Instructions
Purpose: Equip the assistant (and future contributors) to produce high‑quality, idiomatic, performance‑aware, and maintainable changes in this Rust Bevy + Rapier + custom WGSL shader codebase.
This file is intentionally concise but comprehensive. Treat it as the operational handbook.
-->

# Copilot Instructions (Ball Matcher)

## 1. High‑Level Architecture
The project is a Bevy game / simulation featuring large numbers of bouncing balls with physics (Rapier 2D), clustering, custom metaball rendering (WGSL shaders), interaction modes, debug overlays, and hot‑reloadable configuration.

Top modules (`src/`):
- `app` – High level `GamePlugin` wiring of sub‑plugins.
- `core` – Foundational components (`Ball`, radii), configuration loading + validation (`GameConfig` and sub‑configs).
- `physics` – Rapier integration, gravity / separation / clustering logic.
- `rendering` – Materials, palette, background, camera setup, metaballs (custom shaders) and alternate visual modes.
- `interaction` – Input mapping, interaction systems (drag, explosion, parameter tweak), session lifecycle helpers.
- `debug` – Logging, debug modes, overlays, stats, (feature‑gated) Rapier debug render.
- `gameplay` – Spawning and higher level game mechanics.

Design style: modular Bevy plugins, additive systems grouped by schedule (Startup / Update). Configuration centralization via `GameConfig` resource inserted at startup and validated with warnings (non‑fatal). Rendering supports switching metaball render modes at runtime. WASM builds embed shaders to avoid fetch overhead; native loads from `assets/` paths.

## 2. Core Conventions
Follow and preserve existing style unless a change yields a clear improvement.

### 2.1 Rust / Bevy
- Prefer *small focused plugins* implementing `Plugin::build` that only register their systems & resources.
- System functions: Keep inputs minimal; prefer queries over broad `Res` usage. Group related systems in a tuple within a single `.add_systems(Schedule, (...))` call for clarity.
- Use `#[derive(Component)]` newtypes (like `BallRadius`) for semantic clarity instead of raw primitives.
- Keep **order independence** where possible. If ordering is required, encode it explicitly (e.g., using system sets / ordering labels) rather than relying on insertion order.
- Favor immutable access (`Res<GameConfig>`) except when mutation is required; isolate mutation to dedicated update systems.
- Log with structured context (`info!(target: "…", ... )`) for filterability. Avoid noisy per‑frame logs in hot loops.
- Validate external inputs (config fields) once at startup; store sanitized values (clamped, positive, etc.) if needed to avoid repeated runtime guards.
- Use feature flags (`debug`) judiciously to keep release binary lean.
- Keep hot path branching minimal (e.g., metaballs updates bail early on toggle off) – replicate pattern.

### 2.2 Configuration
- `GameConfig` layering: maintain ability to overlay multiple RON files (`game.ron`, `game.local.ron`). When adding config fields:
	1. Extend the struct with `#[serde(default)]`.
	2. Update `Default` implementation.
	3. Add validation warnings where misuse could degrade performance, stability, or UX.
	4. Reflect new fields in WASM embedded loader (`include_str!`).
- Never hard panic on config issues; log warnings and fall back to defaults (current pattern).

### 2.3 Physics (Rapier)
- Ensure gravity and scale remain consistent; if enabling custom gravity from config, set through `RapierConfiguration` in a startup system. Keep in mind Rapier guidance (SI units ideal; avoid giant coordinate magnitudes). Reference: rapier.rs common mistakes (gravity non‑zero, mass, collider presence).
- Avoid per‑frame expensive broad queries; cache handles or use query filters.
- When adjusting forces/impulses, consider stable timestep assumptions; if adding variable timestep logic, gate behind explicit design decision.

### 2.4 Rendering & Shaders
- WGSL shader handles: For WASM keep `OnceLock` pattern for embedding; for native rely on asset path – replicate this dual path for any new shader pair.
- Uniform structs: Maintain 16‑byte alignment (use `#[repr(C, align(16))]` and `ShaderType`). Keep arrays sized with compile‑time constants (e.g., `MAX_BALLS`); if increasing sizes, assess GPU uniform buffer limits.
- Minimize divergence between variant materials (classic vs bevel). If logic duplication grows, refactor to shared helper (not yet critical – note as potential tech debt).
- Keep color calculation consistent (`color_for_index`). If adding palette logic, centralize mapping rather than scattering conversions.

### 2.5 Interaction / Input Map
- Input bindings should be symbolic (“MetaballIsoInc”) not hard‑coded key checks spread across systems. Reuse existing `InputMap` resource pattern. Add tests / hot reload support if new bindings are introduced.

### 2.6 Debug & Instrumentation
- Feature gate heavy debug systems to avoid overhead in release (keep `#[cfg(feature="debug")]`).
- Provide toggles via config or keybindings rather than compile‑time flags where runtime adjustment is valuable (as done with `rapier_debug` fallback).
- Use targeted log targets (e.g., `target: "metaballs"`) so external consumers can filter.

## 3. Performance Guidelines
Primary hotspots: metaball material updates, ball spawning, cluster updates, physics stepping.

Apply:
- Early exits (`if !toggle.0 { return; }`).
- Cap iteration using fixed maxima (`MAX_BALLS`, `MAX_CLUSTERS`) – any dynamic expansion should benchmark memory bandwidth & GPU transfer impact first.
- Avoid unnecessary allocations inside per‑frame systems (no `Vec` growth each frame). Reuse buffers or rely on fixed arrays.
- Release builds for profiling (`cargo run --release`). If a new heavy dependency is added, consider profile overrides for dev (Rapier example: raise opt level while keeping fast dev compile for the rest).
- Log frequency throttling: for repeated param tweaks, log only on change (current approach). Maintain this pattern.

## 4. Safety & Robustness
- Handle `Query::single*` fallibility with graceful `else { return; }` patterns to prevent panics; continue using this guard approach.
- Clamp or sanitize user‑driven / config values before use in math sensitive code (e.g., iso threshold clamp). Replicate that for any nonlinear parameters.
- Prefer deterministic ordering when iterating collections where color/mode mapping matters (document assumptions if order becomes significant).
- If adding parallel systems (e.g., via `bevy::tasks`), ensure Send/Sync boundaries of resources & avoid racey interior mutability.

## 5. Adding New Features – Workflow Checklist
1. Define config additions (if needed) – defaults + validation.
2. Create a focused plugin encapsulating systems/resources.
3. Insert plugin from `GamePlugin` (avoid bloating `main.rs`).
4. Provide toggles (config flag or runtime key) for experimental or heavy features.
5. Add logging on initialization and on user‑visible mode changes only.
6. Write a minimal test (if logic is pure / config validation) under appropriate module; for ECS integration rely on Bevy test utilities or keep logic factored for unit testing.
7. Update documentation: this file (only if conventions shift), README (auto‑generated script), and config sample.

## 6. Code Style Specifics
- Keep related small structs / impls on a single line only when trivially simple (current style mixes condensed and expanded – prefer expanded with line breaks for multi‑field impls or long function bodies for readability, but do not mass‑reformat existing concise code without cause).
- Avoid trailing `pub use` expansions unless they provide ergonomic crate API surface (current curated re‑exports acceptable; extend only for widely consumed types).
- Favor explicit module paths in cross‑domain references to clarify ownership (e.g., `crate::rendering::palette::palette::color_for_index`). If commonly used, consider a local `use` at top.
- Maintain `#[allow(dead_code)]` only where transitional; remove once referenced or intentionally deprecated (track as tech debt).

## 7. Configuration Validation Patterns
When adding validations follow existing pattern:
```
if new_field < 0.0 { w.push("new_field must be >= 0".into()); }
```
Avoid panics; consolidate warnings then log them after load (as in `main.rs`).

## 8. Testing Guidance
- Prefer pure functions for any math / transformation logic extracted from ECS systems, making them unit testable.
- For config load layering, add tests verifying override precedence.
- Mock or stub queries sparingly; often easier to spin a minimal `App` and add required components/resources.
- Keep per‑test entity counts minimal for speed.

## 9. Shaders / GPU Considerations
- Align uniform buffer structures (16‑byte) and keep scalar packing predictable – consult WGSL rules when adding fields (group scalars into `vec4` slots if needed for alignment).
- Limit uniform array growth; if dynamic resizing is required, consider using storage buffers & feature gate it.
- WASM path: ensure any newly embedded shaders use consistent relative paths and unique debug names for caching.

## 10. Input Mapping Extensions
- Centralize new actions and expose human‑readable names for debug UIs.
- Provide `just_pressed` semantics for discrete toggles; continuous effects should check `pressed` each frame with damped or capped application.

## 11. Logging & Observability
- Use distinct log targets per subsystem (`metaballs`, `config`, `spawn`, `interaction`).
- Keep high cardinality data (per‑ball state) out of logs.
- Consider adding an optional frame timing diagnostic plugin if performance optimization tasks emerge.

## 12. Performance Red Flags (Avoid)
- Allocations or `format!` within per‑entity loops each frame.
- Excessive `Query::single*` panics (should never panic in release).
- Copying large arrays (`balls`, `cluster_colors`) more than once per frame.
- Unbounded entity spawning without backpressure or configuration gating.

## 13. Tech Debt Handling
If duplication between classic & bevel metaball update logic grows, extract shared function (e.g., `populate_metaball_uniform`).
Track any TODO with one of: `// TODO:`, `// PERF:`, `// SECURITY:`, `// REFACTOR:` tags. Provide concise rationale.

## 14. Security / Safety (Minimal Surface)
No network / file IO outside config loading; keep file reads constrained to `assets/config/`. On future additions (save/load), sanitize paths (no `..` traversal) and handle IO errors gracefully.

## 15. WASM Specific Notes
- Maintain `console_error_panic_hook` setup.
- Avoid threading / unsupported system calls; keep features used compatible with wasm32 target.
- Embed essential shaders & configs; large dynamic loads risk latency.

## 16. When Unsure
Before large refactors:
1. Propose a small spike / design diff referencing this file.
2. Preserve observable behavior (config compatibility, deterministic visuals unless feature flagged).
3. Benchmark representative releases before & after (frame time, ball count scaling).

## 17. Contribution Ready Definition
A change is ready when:
- Builds & runs on native + WASM (if applicable).
- No new clippy warnings relevant to modified code (run `cargo clippy --all-targets --all-features`).
- Added/modified config fields documented & validated.
- Performance characteristics of hot paths not regressed (spot check with a release run).
- Tests (if affected logic is testable) updated or added.

## 18. Quick Reference Snippets
Startup system adding gravity from config (pattern reminder):
```rust
fn apply_gravity(mut q: Query<&mut RapierConfiguration>, cfg: Res<GameConfig>) {
		if let Ok(mut r) = q.single_mut() { r.gravity = Vect::new(0.0, cfg.gravity.y); }
}
```

Adding a new toggleable visual mode plugin skeleton:
```rust
pub struct NewVisualPlugin;
impl Plugin for NewVisualPlugin { fn build(&self, app: &mut App) {
		app.init_resource::<NewVisualParams>()
				.add_systems(Startup, setup_new_visual)
				.add_systems(Update, (update_new_visual,));
}}
```

## 19. Accessibility & Inclusivity Notes
Although primarily a simulation/game, when adding UI or text: use clear, high‑contrast colors, avoid color‑only differentiation, and prepare future text elements for localization by avoiding hard‑coded concatenations.

## 20. Keep This File Current
Update this document when:
- Introducing a new subsystem pattern.
- Changing max buffer sizes impacting shaders.
- Altering configuration layering strategy.
- Adopting parallel execution or new scheduling phases.

---
Generated with accessibility and maintainability in mind. Review periodically & adapt as architecture evolves.

