## Embedded Level Loading Prompt (WASM-Friendly) v2

<!-- <prompt name="embedded-levels-migration-v2"> -->

### Summary
You WILL migrate the current RON-driven level registry (levels.ron → layout/widgets file indirection) to a dual-mode system that (a) embeds level data with `include_str!` for `wasm32` (and optionally native) and (b) preserves disk-based + optionally hot-reloadable development flow on native builds. You WILL retain existing parsing logic (RON -> structs) and runtime integration semantics (wall spawning, widget extraction). You WILL keep universal walls (`basic_walls.ron`) disk-loaded for now (explicitly out-of-scope to embed).

### Purpose
You WILL eliminate runtime filesystem dependence for level content on WASM targets, enabling frictionless deployment, while keeping native iteration speed and optional hot reload. You WILL reduce indirection by removing the `levels.ron` registry in favor of compile-time or hard-coded Rust configuration.

### Scope
In-Scope:
- Level registry abstraction (replace/remove `levels.ron`).
- Embedded vs disk sourcing for layout + widgets RON.
- Trait-based provider selection.
- Feature flags: `embedded_levels`, `live_levels`.
- Logging, error handling, version checks.

Out-of-Scope (Phase 2 or later):
- Embedding `basic_walls.ron` (universal walls) (explicitly excluded now).
- Schema changes to layout/widgets.
- New widget kinds or wall timeline features.
- Asset compression / binary packing.

### Success Criteria (You MUST meet all)
1. WASM builds perform zero runtime file IO for level layout/widgets (only `include_str!` static slices).
2. Native w/out features behaves identically to current implementation (loads from disk, respects CLI/env selection precedence).
3. Native with `--features embedded_levels` forces embedded mode (same path as wasm).
4. Native with `--features live_levels` (and NOT embedded) enables (or stubs) hot-reload flow; conflicting flags produce a warning and disable live reload.
5. Version validation logic for layout (v1/v2) and widgets (v1) preserved exactly.
6. Failure to parse embedded data (any level) in embedded mode causes immediate panic (fast fail) — no silent fallback.
7. Distinct log line: `LevelLoader: mode=<Embedded|Disk|Disk+Live> selected level id='<id>'`.
8. Adding a new level in embedded mode requires editing only one Rust module (`embedded_levels.rs`).
9. GameConfig integration (spawn + gravity widgets) yields identical widget counts for existing `test_layout` level vs pre-migration.
10. Public API changes outside level subsystem minimized (no consumer breakage expected).

### Inputs
- Existing asset files: `assets/levels/test_layout/layout.ron`, `assets/levels/test_layout/widgets.ron`, `assets/levels/basic_walls.ron`.
- Existing parsing types: `LayoutFile`, `WidgetsFile`, extraction logic in `widgets.rs`.
- CLI/env level request (`--level <id>` / `LEVEL_ID`).

### Outputs / Deliverables
- New module: `src/core/level/embedded_levels.rs` implementing provider abstraction.
- Updated `loader.rs` using provider instead of `LevelRegistry` file indirection.
- Mark `registry.rs` deprecated (attribute + comment) OR remove usage reference from `mod.rs`.
- Add `[features]` section in `Cargo.toml` with `embedded_levels`, `live_levels`.
- Optional stub or implementation for live reload.
- (Optional) Test file `tests/level_embedded_smoke.rs` verifying embedded parse path.

### Data Structures
You WILL define:
```rust
pub struct EmbeddedLevel {
    pub id: &'static str,
    pub layout_ron: &'static str,
    pub widgets_ron: &'static str,
}

pub enum LevelSourceMode { Embedded, Disk }

pub trait LevelSource {
    fn list_ids(&self) -> &[&'static str];
    fn default_id(&self) -> &str;
    // Embedded mode path (static data)
    #[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
    fn get_level(&self, id: &str) -> Result<(&'static str, &'static str), String>;
    // Disk mode path (owned strings) — avoids memory leaks and enables reload.
    #[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
    fn get_level_owned(&self, id: &str) -> Result<(String, String), String>;
}
```

### WASM / Embedded Strategy
You WILL:
- Use `#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]` to select embedded provider.
- Implement constants with `include_str!` referencing existing asset files.
- Provide `const EMBEDDED_LEVELS: &[EmbeddedLevel]` enumerating all levels.
- Panic (`expect` / `unwrap`) on any parse failure for embedded data (fast fail at startup).
- Log a single authoritative mode line with `mode=Embedded`.

### Native Strategy (Disk Mode)
You WILL:
- Default to disk mode when neither wasm32 nor `embedded_levels` feature is active.
- Implement a `DiskLevelSource` returning owned `String` content via `get_level_owned` (NO `Box::leak`).
- Build layout/widgets relative paths: `<id>/layout.ron` + `<id>/widgets.ron` under `assets/levels`.
- Hard-code or programmatically enumerate (Option A) recognized level ids (initially: `test_layout`). (Preferred for simplicity; DO NOT re-parse a registry file.)
- Provide fallback to default id if requested id missing (matching current semantics) with a warning.
- Keep universal walls loaded from disk (`basic_walls.ron`).
- If `live_levels` feature enabled AND not embedded: log mode `Disk+Live` and (a) implement watcher OR (b) log stub warning if not yet implemented.

### Feature Flags (Cargo.toml)
```toml
[features]
embedded_levels = []
live_levels = []
default = []
```
If you implement live reload, add: `notify = "6"` under `[dependencies]` conditionally.

### Migration Steps (MANDATORY Order)
1. Add feature flags to `Cargo.toml`.
2. Create `embedded_levels.rs` module with:
   - Embedded constants (include_str!).
   - `EmbeddedLevel`, provider implementation, factory function.
3. Implement `DiskLevelSource` for native (no features) with `get_level_owned`.
4. Add `select_level_source()` returning `(LevelSourceMode, Provider)` using cfg.
5. Refactor `loader.rs`:
   - Replace registry loading with provider selection.
   - Acquire requested id (CLI/env) then fallback default.
   - In embedded mode: parse via `ron::from_str::<LayoutFile>(layout_str)` (panic on error).
   - In disk mode: parse via borrowed `&owned_string`; on error log and early return (as previous style).
6. Insert logging: `info!(target="level", "LevelLoader: mode={:?} selected level id='{}'")` before parsing universal walls.
7. Preserve universal walls load logic unchanged.
8. Integrate parsed walls/widgets EXACTLY as before (spawn entities, populate GameConfig, insert resources).
9. Mark `registry.rs` with `#[deprecated(note="Replaced by embedded_levels LevelSource abstraction")]` and remove its usage from any `mod.rs` re-export; keep file for reference.
10. Optional: Add stub for live reload:
    - If feature active but not implemented: `warn!("LevelLoader: live_levels feature active but watcher not implemented (TODO)");`.
11. Run native build (no features) — verify identical logs (aside from mode line) and counts.
12. Run native embedded: `cargo run --features embedded_levels` — verify mode Embedded and no filesystem reads for layout/widgets.
13. Build wasm target. Confirm compile success; runtime parsing success (no panic). Validate absence of `read_to_string` calls for level layout/widgets (grep or instrumentation).
14. (Optional) Add smoke test confirming embedded parse yields expected spawn/attractor counts.
15. Update CHANGELOG with migration note.

### Edge Cases (You MUST handle)
- Duplicate level ids (either provider): log warning; first occurrence wins.
- Empty embedded level slice: panic with clear message.
- Invalid requested id: warn + fallback default (disk); panic if embedded default also missing.
- Layout with zero-length wall segments remains filtered as before.
- Widget data anomalies (already handled in `extract_widgets`) still produce warnings.
- Conflicting features: both `embedded_levels` and `live_levels` → warn, ignore live reload.

### Logging Requirements
You WILL emit:
- Mode selection line.
- Existing info/warn/error lines unchanged.
- Additional warnings for: conflicting feature flags, unknown requested level id fallback.

### Hot Reload (live_levels) Outline (Optional Implementation)
If implemented now:
- Add `notify` dependency (feature-gated).
- Watch active level layout + widgets files.
- Debounce events (>=100ms).
- On change re-parse + re-apply (factor parse/apply into reusable function reused by initial load).
- Log summary counts after successful reload.
If NOT implemented: single warning stub (see migration step 10).

### Example Embedded File (Initial Single Level)
```rust
// src/core/level/embedded_levels.rs (excerpt)
pub const TEST_LAYOUT_LAYOUT: &str = include_str!("../../../assets/levels/test_layout/layout.ron");
pub const TEST_LAYOUT_WIDGETS: &str = include_str!("../../../assets/levels/test_layout/widgets.ron");

pub const EMBEDDED_LEVELS: &[EmbeddedLevel] = &[
    EmbeddedLevel { id: "test_layout", layout_ron: TEST_LAYOUT_LAYOUT, widgets_ron: TEST_LAYOUT_WIDGETS },
];
```

### Loader Refactor Sketch
```rust
let (mode, source) = select_level_source();
let requested = resolve_requested_level_id();
let chosen_id = requested.as_deref().unwrap_or(source.default_id());
info!(target="level", "LevelLoader: mode={:?} requested='{:?}' selected level id='{}'", mode, requested, chosen_id);

#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
let (layout_txt, widgets_txt) = source.get_level(chosen_id).expect("embedded level not found");

#[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
let (layout_owned, widgets_owned) = match source.get_level_owned(chosen_id) {
    Ok(p) => p,
    Err(e) => { error!("LevelLoader: failed to load level '{}': {e}", chosen_id); return; }
};

#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
let layout_file: LayoutFile = ron::from_str(layout_txt).expect("parse embedded layout failed");
#[cfg(any(target_arch = "wasm32", feature = "embedded_levels"))]
let widgets_file: WidgetsFile = ron::from_str(widgets_txt).expect("parse embedded widgets failed");

#[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
let layout_file: LayoutFile = match ron::from_str(&layout_owned) { Ok(l) => l, Err(e) => { error!("layout parse failed: {e}"); return; } };
#[cfg(not(any(target_arch = "wasm32", feature = "embedded_levels")))]
let widgets_file: WidgetsFile = match ron::from_str(&widgets_owned) { Ok(w) => w, Err(e) => { error!("widgets parse failed: {e}"); return; } };
```

### Testing Instructions (You WILL Execute)
1. Native default run (no features). Confirm: mode=Disk; spawn/attractor counts unchanged.
2. Native embedded run. Confirm: mode=Embedded; no file IO log errors.
3. wasm32 build (e.g. `cargo build --target wasm32-unknown-unknown --features embedded_levels`). Confirm compile success.
4. Intentionally corrupt one embedded constant locally; ensure panic occurs early; revert.
5. (Optional) Add test verifying `select_level_source` returns correct mode under cfg permutations (use `#[cfg(test)]` + conditional asserts).

### Acceptance Checklist (You MUST verify before completion)
- [ ] WASM path uses only embedded strings.
- [ ] Disk path unaffected for vanilla native.
- [ ] Mode log emitted exactly once on load.
- [ ] Version checks still enforced.
- [ ] No Box::leak usage for disk strings.
- [ ] Duplicate level id warning scenario handled.
- [ ] Feature flag conflict warning works.
- [ ] Fallback to default id on unknown request works in disk mode.
- [ ] Panic on parse failure in embedded mode works (manually tested).
- [ ] GameConfig widget counts equal pre-migration baseline for test_layout.

### Quality Requirements
You WILL:
- Reuse existing parsing & integration code (NO duplication of logic beyond provider abstraction).
- Keep public symbols minimal (only expose what’s necessary from `embedded_levels.rs`).
- Use doc comments explaining provider differences and feature flag semantics.
- Maintain existing logging style (target="level").
- Keep changes orthogonal (no unrelated refactors).

### Non-Goals (Reiterated)
- Embedding universal walls.
- Overhauling widget extraction logic.
- Introducing compression or packing formats.
- Changing CLI/env override precedence.

### Future Extensions (Document only, DO NOT Implement Now)
- Embed universal walls for WASM parity (`BASIC_WALLS: &str = include_str!(...)`).
- Add incremental hot reload diffing (only re-spawn changed walls) instead of full rebuild.
- Add checksum-based logging to detect content drift between embedded & disk versions during dev.

### Validation Cycles Performed
- Cycle 1: Initial prompt authored; Prompt Tester identified ambiguities (ownership model, panic semantics, registry deprecation clarity, universal walls scope, hot reload stub handling).
- Cycle 2: Adjustments applied; Prompt Tester confirmed zero remaining critical ambiguities.

### Final Implementation Directive
You WILL now implement exactly as specified above. If encountering unforeseen constraints (e.g., path resolution differences), you WILL document the deviation inline with a comment referencing the step and rationale.

<!-- </prompt> -->
