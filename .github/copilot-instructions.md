<!--
Ball Matcher - Copilot Project Instructions
Purpose: Equip the assistant (and future contributors) to produce high‑quality, idiomatic, performance‑aware, and maintainable changes in this Rust Bevy + Rapier + custom WGSL shader codebase.
This file is intentionally concise but comprehensive. Treat it as the operational handbook.
-->

# Copilot Instructions (Ball Matcher)

## 1. High‑Level Architecture
Simulation / game built with Bevy + Rapier 2D + a unified metaball shader. Focus: large ball counts, clustering, procedural + surface noise shading, gesture interaction, and feature‑gated debug tooling.

Key modules (`src/`):
* `app` – `GamePlugin` composes camera, materials, physics setup, gravity, spawning, clustering, metaballs, input actions, cluster pop, debug, hot reload (config & input), auto close.
* `core` – Components, ordering labels (`PrePhysicsSet`, `PostPhysicsAdjustSet`), layered config load & validation.
* `physics` – Rapier setup (`PhysicsSetupPlugin`), radial gravity, clustering passes, post‑physics adjustments.
* `rendering` – Camera, material palettes, unified metaballs material + noise uniforms, shader mode cycling.
* `interaction` – Custom input pipeline (collection + evaluation chain), cluster popping, session lifecycle.
* `debug` – Feature‑gated overlay, stats, visual override modes, optional Rapier wireframe (runtime toggle when not compiled with `debug` but config enables).
* `gameplay` – Ball spawning / entity construction.

Reference base engine idioms in `external/bevy` (submodule) for ECS scheduling, plugin structuring, material patterns. Prefer local established minimal wrappers while mirroring upstream naming where helpful.

Design style: small composable plugins; explicit ordering with named `SystemSet`s (not implicit insertion). Config inserted early then treated read‑only except for hot reload. WASM embeds shaders with `include_str!` + `OnceLock`; native uses asset paths. Unified metaball material packs all field + color data.

## 2. Core Conventions
Follow and preserve existing style unless a change yields a clear improvement.

### 2.1 Rust / Bevy
* Small focused plugins (one concern). `GamePlugin` just composes.
* Explicit `SystemSet`s for ordering:
	* `PrePhysicsSet` – forces / velocity edits before Rapier step.
	* Rapier simulation (plugin internal).
	* `PostPhysicsAdjustSet` – lightweight corrections / clustering.
	* `DebugPreRenderSet` (feature gated) – overlay & stats after physics.
	* `InputActionUpdateSet` (in `PreUpdate`) – deterministic input capture & evaluation chain before gameplay.
* Group related systems via tuple in one `.add_systems(Stage, (...))`; use `.chain()` when strict intra‑tuple ordering required.
* Minimize system params; avoid broad `ResMut<GameConfig>` in hot paths. Live reads (`Res<GameConfig>`) acceptable; mutation isolated to config hot‑reload systems.
* Component newtypes (`BallRadius`, `BallMaterialIndex`) for semantic clarity & packing.
* Encode ordering with `.after()` / `.in_set()`; never rely on registration order.
* Logging: targeted (`target = "metaballs"|"spawn"|"config"`). Avoid per‑entity per‑frame logs.
* Validation done at startup; warnings only. Panics only for platform policy (backend mismatch, missing WebGPU on wasm).
* Guard optional heavy systems with early exit toggles (`if !toggle.0 { return; }`).

### 2.2 Configuration
- `GameConfig` layering: maintain ability to overlay multiple RON files (`game.ron`, `game.local.ron`). When adding config fields:
	1. Extend the struct with `#[serde(default)]`.
	2. Update `Default` implementation.
	3. Add validation warnings where misuse could degrade performance, stability, or UX.
	4. Reflect new fields in WASM embedded loader (`include_str!`).
- Never hard panic on config issues; log warnings and fall back to defaults (current pattern).

### 2.3 Physics (Rapier)
* `PhysicsSetupPlugin` registers Rapier; `configure_gravity` currently sets gravity to zero (radial gravity/other forces handled elsewhere). If adding config gravity ensure no conflict with custom force systems.
* Keep scale moderate; large magnitudes require retuning damping.
* Cache reused handles to avoid re‑querying broad sets.
* If moving to fixed timestep logic, document & gate; safest current assumption is variable frame time.

### 2.4 Rendering & Shaders
* WASM: embed WGSL via `include_str!` -> `Assets<Shader>` + `OnceLock` handles. Native: path refs. Mirror this for new shader variants.
* Unified metaballs material structure:
	* Header vectors (`v0`, `v1`, `v2`) for counts, iso, modes, viewport size, time, normalization.
	* Fixed arrays: `balls[MAX_BALLS]`, `cluster_colors[MAX_CLUSTERS]`. Raising limits requires checking uniform buffer size limits; prefer storage buffer refactor over unbounded growth.
* All GPU structs `#[repr(C, align(16))]` + `ShaderType` (includes noise & surface noise uniforms) – preserve alignment.
* Orphan ball color slot logic ensures deterministic palette (prevents flicker). Preserve when altering cluster logic.
* Clamp iso & derived normalization to safe epsilon before pow/roots.
* Centralize palette operations in `palette::color_for_index`.

### 2.5 Interaction / Input Map
* Pipeline: `PreUpdate` -> `InputActionUpdateSet`: `system_collect_inputs` then `system_evaluate_bindings` (chained) for deterministic action states.
* Systems depend on symbolic actions (`MetaballIsoInc`) not raw key codes (except mode cycling keys intentionally separate).
* Hot reload for input map is feature gated (`debug`); keep file watch cost out of release.
* New actions: extend parser + tests; update debug overlay visibility.

### 2.6 Debug & Instrumentation
- Feature gate heavy debug systems to avoid overhead in release (keep `#[cfg(feature="debug")]`).
- Provide toggles via config or keybindings rather than compile‑time flags where runtime adjustment is valuable (as done with `rapier_debug` fallback).
- Use targeted log targets (e.g., `target: "metaballs"`) so external consumers can filter.

## 3. Performance Guidelines
Primary hotspots: metaball material uniform packing, clustering passes, mass spawning bursts, physics stepping.

Apply:
- Early exits (`if !toggle.0 { return; }`).
- Cap iteration using fixed maxima (`MAX_BALLS`, `MAX_CLUSTERS`) – any dynamic expansion should benchmark memory bandwidth & GPU transfer impact first.
- Avoid unnecessary allocations inside per‑frame systems (no `Vec` growth each frame). Reuse buffers or rely on fixed arrays.
- Release builds for profiling (`cargo run --release`). If a new heavy dependency is added, consider profile overrides for dev (Rapier example: raise opt level while keeping fast dev compile for the rest).
- Log frequency throttling: for repeated param tweaks, log only on change (current approach). Maintain this pattern.

## 4. Safety & Robustness
* Use early returns for optional single queries (`let Ok(x) = ... else { return; }`).
* Sanitize config at application & on live read (surface noise amp clamp, iso epsilon, radius multiplier floor).
* Color slot ordering: cluster slots allocate sequentially; orphan slots per material index. If cross‑run determinism becomes required replace `HashMap` with ordered map.
* Parallelism: current hot systems (metaballs update) intentionally single‑threaded; evaluate parallelization only after profiling shows consistent benefit.

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
Consult this file AND upstream examples in `external/bevy`. If conflicting patterns:
1. Choose the *safest* (least panic, clamped values, additive config changes).
2. Preserve existing public config keys (additive > breaking rename).
3. Optimize only after measuring (avoid speculative micro‑tuning).
4. Provide a design diff / spike for large refactors referencing system sets & config impact.

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
Update when:
* New system set names or ordering constraints are added.
* Metaball uniform shapes / maxima change.
* Gravity policy shifts (e.g., enabling config gravity instead of zero baseline).
* Config layering strategy changes.
* New hot reloadable domains introduced.

---
Generated with accessibility and maintainability in mind. Review periodically & adapt as architecture evolves.

