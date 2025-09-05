<!-- text-widget.prompt.md -->
<!-- Purpose: Authoritative implementation instructions for introducing a new data‑driven TextSpawn widget that renders tappable bubble/voxel text using existing ball + metaball + clustering systems. -->

# TextSpawn Widget Implementation Prompt

You WILL implement a new widget kind `TextSpawn` that converts provided text into a field of target points, spawns standard physics balls at randomized positions, and attracts each ball toward its unique target so letters/words form and become tappable clusters.

## High-Level Goals
1. Data-driven declaration in existing `widgets.ron` (same version 1) alongside `SpawnPoint` & `Attractor`.
2. Non-invasive integration: reuse existing `Ball` entity archetype, metaball rendering, clustering & popping logic.
3. Provide per-ball metadata (word / char indices) enabling future interactions (e.g., word pop events).
4. Deterministic-ish layout: given same config and RNG seed (optional), produce consistent letter shapes.
5. Avoid performance regressions: modest allocation, O(point_count) each frame for attraction.

## Source Files To Update / Create (MANDATORY)
1. Modify `src/core/level/widgets.rs` to:
   - Extend `WidgetDef` with optional TextSpawn raw fields.
   - Add `TextSpawnSpec` runtime struct & color mode enum.
   - Extend `ExtractedWidgets` (+ default) and `extract_widgets` to parse & validate `TextSpawn` entries.
2. Modify `src/core/level/loader.rs`:
   - Add `text_spawns: Vec<TextSpawnSpec>` into `LevelWidgets` resource.
   - Log counts including text spawns.
3. (If needed) Modify `src/core/config/config.rs` ONLY if global caps / parameters beyond existing `spawn_widgets.global_max_balls` required. Prefer reuse; do NOT add config unless essential.
4. Create new file `src/gameplay/text_spawn.rs` (plugin + systems described below).
5. Modify main/game plugin registration site (where `SpawnWidgetsPlugin` & `ClusterPopPlugin` are added) to include `TextSpawnPlugin` ordering before physics & clustering updates.
6. Update `assets/levels/menu/widgets.ron` replacing the sample SpawnPoint with a `TextSpawn` block (example provided below) OR add alongside existing.
7. (Optional) Add small unit tests under `tests/` verifying extraction & basic attraction convergence.

## RON Schema Additions (Raw Fields)
You WILL append these optional fields to `WidgetDef` (serde default):
```
text: Option<String>,
font_px: Option<u32>,
cell: Option<f32>,            // sampling grid world units
jitter: Option<f32>,          // initial random spawn jitter radius (AABB based)
radius: Option<RangeF32>,     // ball radius range
speed: Option<RangeF32>,      // initial speed range (random direction)
attraction_strength: Option<f32>,
attraction_damping: Option<f32>,
snap_distance: Option<f32>,   // threshold world distance to mark settled
color_mode: Option<String>,   // "RandomPerBall" | "WordSolid" | "Single"
word_colors: Option<Vec<usize>>, // palette indices mapping by word order
```

If `text` is missing or empty => emit warning & skip.

## Runtime Specification Struct
```
pub enum TextColorMode { RandomPerBall, WordSolid, Single }

pub struct TextSpawnSpec {
    pub id: u32,
    pub pos: Vec2,
    pub text: String,
    pub font_px: u32,
    pub cell: f32,
    pub jitter: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub attraction_strength: f32,
    pub attraction_damping: f32,
    pub snap_distance: f32,
    pub color_mode: TextColorMode,
    pub word_palette_indices: Vec<usize>,
}
```
Default values (apply during extraction):
```
font_px: 140
cell: 14.0
jitter: 42.0
radius: 7.0..13.0
speed: 0.0..50.0
attraction_strength: 60.0
attraction_damping: 6.5
snap_distance: 3.2
color_mode: RandomPerBall
word_palette_indices: []
```
Validation adjustments:
* Swap radius min/max if reversed.
* Clamp all mins to > 0 (use small epsilon 0.01).
* If `cell <= 1.0` set to 8.0 and warn.
* Unknown color_mode => fallback RandomPerBall (warn).

## Components to Introduce
```
#[derive(Component)]
pub struct TextSpawnRoot { pub id: u32 }

#[derive(Component)]
pub struct TextBall {
    pub word_index: u16,
    pub char_index: u16,
    pub target_local: Vec2,
    pub settled: bool,
}

#[derive(Component)]
pub struct TextAttractionParams {
    pub strength: f32,
    pub damping: f32,
    pub snap_distance: f32,
}
```

## TextSpawnPlugin Systems
You WILL create `TextSpawnPlugin` with:
1. `instantiate_text_spawns` (PostStartup, AFTER LevelLoader) – builds text voxel points & spawns entities.
2. `apply_text_attraction` (Update in `PrePhysicsSet` BEFORE physics step) – applies per-ball velocity adjustments.
3. (Optional, future) `detect_word_cluster_pops` – integrate with `ClusterPopped` events (deferred for initial implementation).

Ordering: run attraction before clustering & metaball updates (similar ordering to `run_spawn_widgets` preceding `MetaballsUpdateSet`). If a dedicated ordering set exists (e.g., `PrePhysicsSet`), insert into it.

## Voxelization Algorithm (MANDATORY Implementation Details)
You WILL implement a helper function (pure; no ECS parameters):
```
fn rasterize_text_points(text: &str, font: &Font, font_px: u32, cell: f32) -> Vec<(usize /*word*/, usize /*char*/, Vec2 /*local*/)>;
```
Steps:
1. Split into words via `split_whitespace` capturing word boundaries; maintain mapping char->word.
2. For each character, get glyph metrics & bitmap (use existing font resource or add `fontdue` crate if not present). If adding dependency, update `Cargo.toml`.
3. Iterate sample grid: for y in (0..h).step_by(cell_px), x similarly; keep alpha > 0.5.
4. Compose glyph-local to line baseline; accumulate horizontal pen advance.
5. After collecting all points, compute bounding box & recenter so (0,0) is midpoint (center-aligned horizontally & vertically) – this yields local coordinates relative to root.
6. De-duplicate points closer than `cell * 0.4` (hash grid or simple O(n log n) sorting) to avoid density spikes.

Performance Constraints: Accept O(n) to O(n log n) where n ≤ ~800 typical.

## Entity Instantiation
For each `TextSpawnSpec`:
1. Create root entity at `spec.pos` with `TextSpawnRoot` + `TextAttractionParams`.
2. For each voxel point (truncated if exceeding capacity defined by `spawn_widgets.global_max_balls` minus existing `Ball` count):
   - Random initial offset within AABB of all points expanded by `jitter` (uniform square) OR within circle using rejection sampling.
   - Radius: uniform in `[radius_min, radius_max]`.
   - Initial velocity magnitude uniform in `[speed_min, speed_max]` with random direction.
   - Components: `Ball`, `BallRadius`, `RigidBody::Dynamic`, collider, damping & restitution from global `GameConfig.bounce`, `TextBall`.
   - Color / variant selection:
     * RandomPerBall: choose random variant index (reuse palette logic from `spawn_single_ball`).
     * WordSolid: variant index = `word_palette_indices[word_index % len]` if non-empty else 0.
     * Single: always 0.
   - Parent the ball to root (`Parent`).

## Attraction System Behavior
For each `(Transform, Velocity, TextBall, Parent(root))` where root has `TextAttractionParams` and entity lacks `PaddleLifecycle`:
1. Compute world target = root_world_translation.xy + `target_local`.
2. delta = target - current_pos.
3. If `delta.length() < snap_distance` AND `velocity.linvel.length() < snap_distance * 2.0` → set `TextBall.settled=true`; skip force (except mild damping `vel.linvel *= 0.90`).
4. Else apply damped spring acceleration: `accel = strength * delta - damping * vel.linvel`.
5. Integrate: `vel.linvel += accel * dt` (simple explicit Euler; stable with moderate parameters).

## Logging & Warnings
You WILL log (info target="text_spawn") per widget:
```
TextSpawn: id={} text="..." points={} truncated={} (if truncated) radius=[{:.1},{:.1}] cell={:.1}
```
Warnings for: swapped radii, unknown color_mode, empty text, cell adjusted, point truncation.

## Example widgets.ron Entry
```
(
    version: 1,
    widgets: [
        (
            type: "TextSpawn",
            id: 100,
            pos: ( x: 0.0, y: 40.0 ),
            text: "BALL MATCHER",
            font_px: 140,
            cell: 14.0,
            jitter: 42.0,
            radius: ( min: 7.0, max: 13.0 ),
            speed: ( min: 0.0, max: 50.0 ),
            attraction_strength: 60.0,
            attraction_damping: 6.5,
            snap_distance: 3.2,
            color_mode: "WordSolid",
            word_colors: [2,5],
        ),
    ],
)
```

## Success Criteria (MANDATORY)
1. Compiles with no new warnings (or documented unavoidable ones) on native target (and wasm if applicable).
2. `cargo test` passes existing tests plus any new text voxel tests.
3. Running the game with `menu` level displays centered bubble text forming within 2 seconds (approx) and settling.
4. Balls are standard `Ball` entities; cluster popping still works—popping part of a letter triggers normal cluster animation.
5. No panic if `word_colors` shorter than word count (safe modulo or fallback).
6. Global ball limit respected (no spawn beyond cap).
7. Removing the TextSpawn block removes all text balls with no residual systems errors.

## Minimal Test Cases
You WILL (if time permits) add tests:
1. `tests/text_spawn_extract.rs`: Provide inline RON with TextSpawn -> assert spec fields normalized.
2. `tests/text_attraction.rs`: Spawn one root + one ball far from target; run system; assert distance decreases.

## Edge Case Handling
| Case | Handling |
|------|----------|
| Empty text | Warn & skip widget |
| Excessive points ( > remaining capacity ) | Truncate; warn with counts |
| Very small cell | Clamp to >= 4.0 & warn |
| Invalid color mode | Fallback RandomPerBall |
| radius_min > radius_max | Swap & warn |
| No palette indices for WordSolid | Fallback to 0 default | 

## Implementation Order (You MUST follow)
1. Extend data model (`WidgetDef`, `ExtractedWidgets`, `LevelWidgets`).
2. Implement parsing & validation in `extract_widgets`.
3. Add new spec types & enums.
4. Adjust loader to store & log text spawns.
5. Create components & plugin file `text_spawn.rs`.
6. Implement voxelization helper (pure function; unit test friendly).
7. Implement instantiation system.
8. Implement attraction system with ordering.
9. Register plugin in main app builder.
10. Update menu `widgets.ron`.
11. Add tests (extraction + attraction).
12. Run full build & tests; fix issues.

## Prohibited / Avoid
You NEVER modify unrelated configuration keys.
You NEVER add global resources if per-entity/components suffice.
You NEVER block the main thread with long CPU loops (voxelization must be lightweight; no multi-threading required initially).
You NEVER duplicate existing color / palette logic—reuse patterns in `spawn_widgets.rs`.

## Optional Enhancements (DEFERRED Not in initial PR)
* Multi-line text with `\n` + alignment modes.
* Animated color ramps per word.
* Caching voxelization results for repeated text across screens.
* Word-level pop event emission.

## Completion Checklist (You MUST verify)
- [ ] Parsing supports `TextSpawn` without breaking existing widget kinds.
- [ ] `TextSpawnSpec` instances appear in logs.
- [ ] Balls spawn and converge into legible glyph shapes.
- [ ] Settled flag eventually true for majority of `TextBall` components.
- [ ] No unbounded growth in entity count.
- [ ] Cluster popping unaffected for ordinary spawn points (regression check).

## Final Developer Notes
* Keep code style consistent with existing modules (naming, logging targets).
* Keep ordering deterministic where possible (sort voxel points by (word_index, char_index) before spawn) to improve reproducibility.
* Document new public structs with concise rustdoc comments.
* If adding a dependency (`fontdue`), pin a sensible version and feature subset.

---
You WILL now implement these steps EXACTLY. If any ambiguity remains, you WILL log a clear TODO comment referencing this prompt section. Proceed.

<!-- End of text-widget.prompt.md -->
