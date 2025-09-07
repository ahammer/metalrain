<!--
Prompt Name: Glyph -> Distance Field Integration (Final Glue)
Purpose: You WILL finalize the end-to-end pipeline that maps textual glyph identifiers (A–Z, a–z, 0–9, punctuation subset) produced by the SDF atlas builder into the metaball shader’s signed distance sampling path so that each ball (or logical grouping) can render a glyph silhouette instead of (or in addition to) primitive shapes. You WILL ensure consistency with existing SDF atlas + metaball architecture already present in this repository.
Scope: Runtime glue + minimal config + assignment logic + tests. The atlas generator & shader SDF sampling path already exist; you are finishing the char lookup, shape index assignment, fallback policy, config layering, and documentation hooks.
CRITICAL: Maintain WASM compatibility, zero panics on missing assets, deterministic mapping, no per‑frame heap churn in hot paths.
References: See `src/rendering/sdf_atlas.rs`, `assets/shapes/sdf_atlas_schema.md`, `metaballs_unified.wgsl`, and packing helpers (`pack_shape_color`). Follow conventions in `.github/copilot-instructions.md`.
-->

## 1. High‑Level Goal
You WILL allow a configurable stream (or static string) of glyphs to drive the `BallShapeIndex` component assignment so that glyph silhouettes (SDF tiles) are rendered for balls instead of only the current deterministic material->shape modulo mapping. Analytic circle fallback MUST remain when:
1. SDF feature disabled via config or force fallback flag.
2. Atlas missing / shape index 0.
3. Ball radius below configured threshold (`sdf_shapes.use_circle_fallback_when_radius_lt`).
4. Glyph requested is absent from the atlas (graceful fallback -> circle or deterministic primitive shape if you extend).

## 2. Success Criteria (MANDATORY)
All MUST be satisfied:
1. Config Extension: `GameConfig.sdf_shapes` gains nested glyph assignment fields (see Section 4) with defaults + validation warnings (never panic) and RON layering compatibility.
2. Loader Enhancement: On atlas load you build an O(1) lookup mapping `char` -> `u16 shape_index` (index 0 still sentinel). Non‑glyph shapes (e.g. `circle`, `triangle`) MUST remain addressable by name for future mixed modes.
3. Resource Contract: Introduce `GlyphShapeMap` resource (struct wrapping `HashMap<char,u16>` plus ordered `Vec<char>` for deterministic iteration) inserted only once after atlas success.
4. Assignment System: New system (after spawning, before metaball GPU buffer build) attaches/updates `BallShapeIndex` for eligible balls using glyph text sequence policy (Section 5). It MUST early‑exit if atlas disabled or glyph mode off.
5. Determinism: Given identical config string and spawn order, shape indices are identical across runs (no reliance on `HashMap` iteration order—iterate stabilized `Vec<char>` or the source string).
6. Shader Path: No shader binding changes required beyond what already exists (SDF path is implemented); you only rely on the existing packed upper 16 bits of the per‑ball `GpuBall.data.w` for shape index.
7. Fallback Logic: Missing glyph -> shape index 0 (analytic circle). Log at most ONCE per missing distinct glyph (throttled) with target `sdf`.
8. Performance: Zero heap allocations per frame proportional to ball count; reuse buffers or assign once. Assignment runs only when new balls spawn or when glyph config hot‑reload diff is detected.
9. Tests: Minimum unit test verifying (a) mapping from 'A' and 'z' when present, (b) fallback path emits 0 for unknown glyph '@' if absent, (c) deterministic cycling modulo when more balls than glyph chars.
10. Docs: Update (or cause update script to include) a short section in README referencing glyph mode usage (invoke existing README generation script if required). If script not yet glyph-aware, add a TODO comment referencing this prompt.

## 3. Non‑Goals (You MUST NOT)
You MUST NOT:
1. Change shader binding indices or uniform layouts (v5/v6 meaning stays the same).
2. Introduce multi-channel (MSDF) logic beyond existing mode discriminant use.
3. Implement text layout, kerning, per‑glyph advance, or multi‑line shaping (future work).
4. Force rebuild of existing SDF atlas; assume builder already outputs glyph_* entries.
5. Add dynamic string parsing every frame—string is sourced from config & only reprocessed on change.

## 4. Config Additions
Extend `SdfShapesConfig` (preserving `#[serde(default)]` and `Default`) with:
```
pub struct SdfShapesConfig {
    pub enabled: bool,
    pub force_fallback: bool,
    pub max_gradient_samples: u32,
    pub gradient_step_scale: f32,
    pub use_circle_fallback_when_radius_lt: f32,
    // NEW:
    pub glyph_mode: bool,              // master toggle for glyph assignment
    pub glyph_text: String,            // sequence used for mapping to balls
    pub glyph_wrap: String,            // policy: "Repeat" | "Clamp" | "None"
    pub glyph_skip_whitespace: bool,   // skip spaces & control chars when true
}
```
Defaults:
```
glyph_mode=false
glyph_text="" (empty => system no‑op)
glyph_wrap="Repeat"
glyph_skip_whitespace=true
```
Validation (append to existing warnings):
* Warn if `glyph_mode && glyph_text.is_empty()`.
* Warn if `glyph_wrap` not one of allowed tokens (auto fallback -> `Repeat`).
* Warn (not error) if `glyph_text` contains characters not present in atlas (list distinct up to 8 then "+N more").

## 5. Glyph → Ball Mapping Policy
You WILL implement the following deterministic mapping:
1. Gather `effective_chars`: iterate `glyph_text` left→right, filter out whitespace if `glyph_skip_whitespace`, collect into `Vec<char>`.
2. If `effective_chars` is empty -> early return (no assignment).
3. For each ball needing a shape (existing sentinel shape index 0 or newly spawned without `BallShapeIndex`), assign based on its stable spawn ordinal (monotonic counter resource `BallSpawnOrdinal(u64)`). Index = `ordinal` → character selection using wrap policy:
   * Repeat: `char = effective_chars[ordinal % len]`.
   * Clamp: `char = effective_chars[min(ordinal, len-1)]`.
   * None: if `ordinal >= len` -> leave existing shape index (keep analytic fallback) and stop further processing (optional early break for performance).
4. Lookup shape index from `GlyphShapeMap` (hash map). If found use it. Else set 0 and, if not previously reported, log one warning listing the missing glyph. Maintain a `HashSet<char>` of already-reported missing glyphs inside a `Local` state for the system.
5. Respect radius threshold: if ball radius (post multiplier) < `use_circle_fallback_when_radius_lt` set shape index to 0 (even if glyph exists) to avoid undersampled aliasing.
6. Do NOT modify shape indices for balls that already have a non‑zero shape index unless `glyph_mode` was toggled from off->on (track previous toggle state in a resource `GlyphModeState { prev_enabled: bool }`). On transition off->on you MAY reassign all sentinel or previously glyph‑assigned indices.

## 6. Loader Enhancements
During `load_sdf_atlas` (or immediately after), after constructing `SdfAtlas`:
1. Iterate `shapes` entries; for each `name` matching regex `^glyph_.$` OR `^glyph_[A-Za-z0-9]$` capture the trailing char.
2. Insert into `GlyphShapeMap` mapping (char -> shape.index as u16). If duplicates (should not happen) keep the first and log a single warning for duplicates.
3. Sort collected glyph chars & store in the resource (vector) for deterministic tests & iteration.
4. Log once: `info!(target="sdf", "Glyph map built: {} glyphs", glyph_map.len())` when `glyph_mode` is true OR debug feature flag enabled.
5. Gracefully skip if no glyph_* entries (no warnings unless glyph_mode requested).

## 7. System Scheduling
Add new system `assign_ball_glyph_shapes`:
* Stage: `Update`.
* Ordering: `.after(assign_ball_shapes_once)` (so material-based fallback runs first) and `.before(MetaballsUpdateSet)`.
* Early returns: missing atlas resource, atlas.disabled, `!cfg.sdf_shapes.enabled`, `!cfg.sdf_shapes.glyph_mode`.
* Use queries filtered with `With<Ball>` retrieving `(Entity, &BallRadius, Option<&mut BallShapeIndex>)`.
* Maintain / update `BallSpawnOrdinal` in the spawning logic (increment for each new ball; can be added where balls currently spawn in `spawn_widgets` or equivalent). If retrofitting is complex, you may derive ordinal by insertion order of query only ONCE at first run (store mapping) but preferred is explicit ordinal increment at spawn time.

## 8. Data Contracts & Packing (Recap)
* Per-ball GPU encoding already: upper 16 bits of packed u32 = shape_index (u16), lower 16 bits = color group id (u16).
* Shape index 0 => analytic circle.
* `SdfShapeGpuMeta[shape_index]` provides uv0, uv1, pivot; pivot presently center (0.5,0.5). You MUST NOT reinterpret fields.
* Distance decoding formula already in shader: `signed_d_px = -(sample - 0.5) * distance_range`.

## 9. Performance & Safety Requirements
You MUST:
1. Avoid reallocating `effective_chars` every frame: cache last processed string + hash; rebuild only when config changes (track `prev_hash` in a resource `GlyphSequenceCache`).
2. Use small `Vec<char>` (<= length of glyph_text) acceptable; no per-ball allocation.
3. Avoid `Query::single*` panics—iterate safely.
4. Cap warning logs: no more than 1 line per missing glyph.
5. Keep gradient sampling unchanged (glyph integration already uses uniform radial scaling; shader handles shape dist). No shader edits required for this glue.

## 10. Testing Strategy
You WILL add tests under `tests/` or a module `#[cfg(test)]` adjacent to the loader:
1. `glyph_mapping_present`: Build a fake `SdfAtlas` with two glyph entries `glyph_A`, `glyph_z` -> assert map length=2, indices stored.
2. `assignment_repeat_policy`: Simulate N=5 balls, glyph_text="AB", wrap=Repeat -> shape sequence A,B,A,B,A.
3. `assignment_clamp_policy`: glyph_text="AB", wrap=Clamp, 5 balls -> A,B,B,B,B.
4. `assignment_none_policy`: glyph_text="AB", wrap=None, 5 balls -> A,B,0,0,0.
5. `missing_glyph_fallback`: glyph_text contains unlisted char; resulting shape index 0.
6. `radius_threshold_fallback`: set `use_circle_fallback_when_radius_lt` large so all assigned become 0.
NOTE: For brevity you MAY construct minimal `App` and insert prebuilt `SdfAtlas` + config resources, then manually spawn test entities.

## 11. Implementation Steps (Ordered)
1. Update config struct + `Default` + validation (append new warnings; do NOT break existing serializer).
2. Enhance `load_sdf_atlas` to build and insert `GlyphShapeMap` resource.
3. Add new resources: `GlyphModeState`, `BallSpawnOrdinal`, `GlyphSequenceCache`.
4. Add ordinal increment in ball spawn code (`spawn_widgets.rs` or central spawn API). Store ordinal on entity if needed for later deterministic assertions (optional).
5. Implement `assign_ball_glyph_shapes` system: a) detect config diff; b) rebuild cached sequence; c) iterate new or sentinel balls; d) assign shape index.
6. Schedule system with proper ordering.
7. Add tests.
8. (Optional) README update or TODO.
9. Run quality gates: build, clippy, tests. Confirm zero new warnings aside from intentional config warnings.

## 12. Edge Cases & Handling
* Empty glyph_text while glyph_mode=true: log warning once, no assignment.
* Duplicate glyph characters inside glyph_text: allowed; they produce repeated shapes in sequence.
* Characters beyond BMP (e.g. emoji): ignore (not matched by `glyph_` single-char names); log once per distinct unsupported char if present.
* Atlas without glyph_* entries but glyph_mode requested: log warning and skip (fallback circle remains).
* Hot reload turning glyph_mode off: do NOT clear existing shape indices (visual continuity). Turning back on reassigns only sentinel or previously glyph-driven entries (you may tag glyph-driven shape indices by storing last applied ordinal in a parallel component if needed; otherwise reassign all non-zero entries—document choice).

## 13. Logging (Targets & Messages)
Use `target="sdf"`:
* INFO: on successful glyph map build (include count) (only if glyph_mode enabled OR debug feature).
* WARN: missing glyph(s) summary, invalid wrap token fallback, empty glyph_text while enabled, no glyph entries present while enabled.

## 14. Follow‑Up (Deferred Improvements)
Document but DO NOT implement:
* Multi-character glyph names & variable advance for text layout.
* Packed per-glyph metrics buffer (advance, bearing) for future text baseline support.
* Gradient precomputation (store d/dx,d/dy in atlas channels to eliminate finite differencing in shader).
* Per-ball glyph overrides (component specifying char).

## 15. Completion Checklist
You MUST verify before merging:
- [ ] Config fields added + defaults + validation warnings tested.
- [ ] `GlyphShapeMap` resource inserted only when atlas loads & glyph entries present.
- [ ] Deterministic mapping confirmed across two runs (manual or test hash of first N assignments).
- [ ] Assignment system respects wrap policies.
- [ ] Fallback behaviors exercised by tests.
- [ ] No extra per-frame allocations (inspect with instrumentation or logging in debug build if unsure).
- [ ] WASM build succeeds (no unsupported APIs used).
- [ ] README updated OR explicit TODO logged.

## 16. Rollback Strategy
Disable by config (`glyph_mode=false`) OR remove `assign_ball_glyph_shapes` system & `GlyphShapeMap` resource insertion; analytic + existing shape mapping path remains intact without shader changes.

<!-- End of Prompt v1 -->

## **Prompt Builder**: Requesting Validation
Prompt Tester, please follow the glyph-integration prompt using this scenario:
* Atlas contains glyphs: glyph_A (index=1), glyph_B (2), glyph_C (3), glyph_z (4)
* Config: glyph_mode=true, glyph_text="ABz", glyph_wrap="Repeat", glyph_skip_whitespace=true.
* Spawn 7 balls with increasing radii all above threshold.
Demonstrate assigned shape indices per ordinal, any logs that would emit, and identify ambiguities or missing instructions.

## **Prompt Tester**: Following glyph-integration Prompt Instructions (Cycle 1)
Following the glyph-integration instructions, I would:
1. Extend config struct adding new fields (glyph_mode, glyph_text, glyph_wrap, glyph_skip_whitespace) with defaults and validation warnings (implementation detail: augment validate()).
2. On atlas load, parse shapes filtering names starting with "glyph_" and length >=7 where last char is taken as glyph. Build map: {'A':1,'B':2,'C':3,'z':4}. Log: info!(target="sdf", "Glyph map built: 4 glyphs").
3. Build effective_chars from glyph_text "ABz" -> ['A','B','z'].
4. Spawn 7 balls; each spawn increments BallSpawnOrdinal -> ordinals 0..6.
5. Apply Repeat wrap: mapping ord->char: 0:A,1:B,2:z,3:A,4:B,5:z,6:A.
6. Lookup indices: [1,2,4,1,2,4,1]. Assign to BallShapeIndex (u16). All radii above threshold so none forced to 0.
7. GPU packing already handled later; no extra per-frame allocations beyond initial vector.
8. No missing glyphs -> no warnings.

Ambiguities / Potential Gaps:
* Wrap policy token values case-sensitivity not specified (assume exact match).
* Behavior for glyph_wrap=None after exceeding length: instruction says "stop further processing" but not explicit whether subsequent newly spawned balls later should also remain analytic; likely yes—clarify.
* Interaction with existing `assign_ball_shapes_once`: if glyph mode toggled on AFTER that system ran, do we overwrite previously assigned shape indices (non-zero) or only sentinel zeros? Prompt prefers not to modify non-zero unless toggled; but toggle detection mechanism for previously analytic assignments needs clarity.
* Whether whitespace inside glyph_text should insert gaps (currently skipped)—spec clarifies skip when flag true; if false, do spaces map to shape index 0 or produce no assignment? Not specified.
* Missing explicit rule for more balls than glyphs under Clamp vs when new balls spawn in subsequent frames (should remain last glyph) – implied but could be made explicit.

All success criteria otherwise actionable.

Feedback: Add explicit casing rule for `glyph_wrap`, detail whitespace handling when `glyph_skip_whitespace=false`, and clarify persistence semantics for None / Clamp across later spawns.

<!-- Validation Cycle 1 complete -->
