## Level Loader Implementation Prompt (Data-Driven Levels v1)

### Goal
Implement a minimal, robust, data-driven level loading system that:
1. Loads a universal walls fragment (`assets/levels/basic_walls.ron`) implicitly.
2. Loads a selected level's layout (`layout.ron`) and widget definitions (`widgets.ron`) based on a registry (`levels.ron`).
3. Spawns static wall colliders from the combined wall list.
4. Spawns widgets (initially: SpawnPoint, Attractor) translating them into existing runtime systems (re-using or minimally adapting `SpawnWidgetConfig` + gravity widgets).
5. Cleanly deprecates/removes any hard-coded or synthesized placeholder geometry / widgets previously baked into startup logic.

### Non-Goals (This Iteration)
- No hot reload of level files (may come later).
- No scripted events, timed bursts, or complex shape types beyond segments.
- No per-level GameConfig overrides beyond widget + wall specification.
- No save/progression system.

### Files In Scope
- `assets/levels/levels.ron` (registry)
- `assets/levels/basic_walls.ron` (universal walls fragment)
- `assets/levels/<level_id>/layout.ron`
- `assets/levels/<level_id>/widgets.ron`

### Current Minimal Schemas
`levels.ron`:
```
(
  version: 1,
  default: "test_layout",
  list: [ ( id: "test_layout", layout: "test_layout/layout.ron", widgets: "test_layout/widgets.ron" ) ],
)
```

`basic_walls.ron`:
```
(
  version: 1,
  walls: [ ( segment: ( from:(x:-640,y:-360), to:(x:640,y:-360), thickness:24 ) ), ... ]
)
```

`layout.ron` (example):
```
(
  version: 1,
  walls: [ ( segment: ( from:(x:0,y:-360), to:(x:0,y:360), thickness:20 ) ) ],
)
```

`widgets.ron` (example):
```
(
  version: 1,
  widgets: [
    ( type:"SpawnPoint", id:0, pos:(x:-400,y:-200),
      spawn:( interval:0.60, batch:4, area_radius:80.0,
              ball_radius:(min:6.0,max:14.0), speed:(min:40.0,max:200.0) ) ),
    ( type:"Attractor", id:100, pos:(x:0,y:0), strength:600.0, radius:0.0,
      falloff:"InverseLinear", enabled:true ),
  ],
)
```

### High-Level Architecture
1. Add a `level` module under `src/core/level/` containing:
   - `registry.rs` (types + loader for `levels.ron`)
   - `layout.rs` (wall parsing types)
   - `widgets.rs` (widget spec enum + parsing)
   - `loader.rs` (public orchestration: select level, load files, validate, produce resources)
2. Introduce resources:
   - `LevelSelection { id: String }` (final chosen id)
   - `LevelWalls(Vec<WallSegment>)`
   - `LevelWidgets { spawn_points: Vec<SpawnPointSpec>, attractors: Vec<AttractorSpec> }`
3. Add a `LevelLoaderPlugin` that:
   - Runs at `Startup` BEFORE physics/setup plugins that might depend on colliders.
   - Inserts above resources.
   - Spawns static colliders.
   - Spawns attractors (mapping to existing gravity widget system OR bypassing if direct integration preferred).
   - Spawns spawn points (mapping into or replacing existing `SpawnWidgetsPlugin`).

### Data Type Sketches
```
// registry.rs
#[derive(Deserialize)] struct LevelRegistry { version: u32, default: String, list: Vec<LevelEntry> }
#[derive(Deserialize)] struct LevelEntry { id: String, layout: String, widgets: String }

// layout.rs
#[derive(Deserialize)] struct LayoutFile { version: u32, walls: Vec<WallDef> }
#[derive(Deserialize)] struct WallDef { segment: SegmentDef }
#[derive(Deserialize)] struct SegmentDef { from: Vec2Def, to: Vec2Def, thickness: f32 }
#[derive(Deserialize)] struct Vec2Def { x: f32, y: f32 }

pub struct WallSegment { pub from: Vec2, pub to: Vec2, pub thickness: f32 }

// widgets.rs
#[derive(Deserialize)] struct WidgetsFile { version: u32, widgets: Vec<WidgetDef> }
#[derive(Deserialize)] struct WidgetDef { #[serde(rename="type")] kind: String, id: u32, pos: Vec2Def, #[serde(default)] spawn: Option<SpawnSpecRaw>, /* attractor fields raw */ strength: Option<f32>, radius: Option<f32>, falloff: Option<String>, enabled: Option<bool> }
#[derive(Deserialize)] struct SpawnSpecRaw { interval: f32, batch: u32, area_radius: f32, ball_radius: RangeF32, speed: RangeF32 }
#[derive(Deserialize)] struct RangeF32 { min: f32, max: f32 }

pub struct SpawnPointSpec { id: u32, pos: Vec2, /* plus spawn fields */ }
pub struct AttractorSpec { id: u32, pos: Vec2, strength: f32, radius: f32, falloff: FalloffKind, enabled: bool }
```

### Collider Generation Strategy
Each `WallSegment` spawns either:
- A thin Rapier `Collider::segment(from, to)` if thickness <= segment length * 0.1 (simplest) OR
- Two-phase: For now ALWAYS use `Collider::segment`. Thickness stored for future upgrade (e.g. to capsule or box).

### Integration With Existing Systems
1. Remove synthetic spawn widgets from `spawn_widgets.rs` startup (when list empty) — replaced by explicit loaded spawn points.
2. Option A (quick): Convert `SpawnPointSpec` into fake `SpawnWidgetConfig` instances and reuse existing plugin.
3. Option B (cleaner, later): Write new spawn system keyed to `LevelWidgets` resource.
4. Gravity widgets: Option A (quick): Translate `AttractorSpec` -> `GravityWidgetConfig` and insert into `GameConfig.gravity_widgets.widgets` BEFORE `GravityWidgetsPlugin` runs.
5. Remove reliance on legacy `gravity.y` when attractors exist (warn if both present).

### Deprecation & Removal Plan
| Legacy Behavior | Action | Timing |
|-----------------|--------|--------|
| Auto-generated 4 corner spawn widgets when config empty | Remove generation logic; require explicit SpawnPoint entries | This iteration |
| Implicit single gravity widget from `gravity.y` | Keep fallback only if no attractors loaded (warn) | This iteration |
| Using `spawn_widgets.widgets` from `game.ron` | Ignore if widgets file loaded (warn once) | This iteration |
| Future: `spawn_widgets.global_max_balls` | Keep for now; may move into widgets file later | Later |

### Validation & Error Handling
- Missing registry file: abort with clear message.
- Empty level list: abort.
- Selected level id not found: fall back to registry.default; if that also missing -> abort.
- Missing layout or widgets file: abort and list expected path.
- Wall segment with identical endpoints: warn & skip.
- Overlapping duplicate widget IDs: warn; keep first, skip rest.
- Attractor with negative strength: clamp to 0, warn.
- SpawnPoint intervals <= 0: clamp to minimum 0.05, warn.
- Ranges where min > max: swap & warn.

### CLI / Env Selection (Optional Minimal)
- Read `--level <id>` from `std::env::args()`.
- Else read `LEVEL_ID` env var.
- Else registry.default.

### System Ordering
Add `LevelLoaderPlugin` before `GamePlugin` (or inserted inside `GamePlugin` prior to physics & widget spawn plugins). Sequence:
1. Load level files & mutate `GameConfig.gravity_widgets/widgets` if needed.
2. Insert `LevelWalls` & `LevelWidgets` resources.
3. Spawn static colliders.
4. Proceed with existing plugins (they see already-populated configs/resources).

### Step-by-Step Implementation Plan
1. Create module `src/core/level/` with the four files.
2. Implement parsing helpers (RON -> struct) using `serde`.
3. Implement registry selection logic (args/env fallback).
4. Load & merge walls (universal + level-specific) into `LevelWalls`.
5. Load widgets, split into spawn vs attractor lists.
6. Adapt `GameConfig` pre-insertion: replace `gravity_widgets.widgets` if attractors present.
7. Remove synthetic spawn widget fallback (delete that branch in `spawn_spawn_widgets`).
8. If spawn points exist: build `spawn_widgets.widgets` vector in `GameConfig` from specs.
9. Add `LevelLoaderPlugin` called early in `main.rs` (before inserting `GamePlugin`).
10. Add logging + validation warnings.
11. Add a small integration test (loads registry + level) under `tests/` (e.g. `level_loader_smoke.rs`).

### Minimal Test Scenarios
1. Happy path: default level loads, walls > 0, spawn point appears, attractor inserted.
2. Missing widgets file: error.
3. Duplicate widget IDs: only first kept, warning emitted.
4. Attractor + gravity.y set: warning gravity.y ignored.

### Future Extension Hooks (Document Only)
- Additional wall shape kinds (circle, box, polygon).
- Scripted events file.
- Hot reload (watcher on level directory).
- Per-level overrides patch applied to `GameConfig`.

### Success Criteria (Definition of Done)
- Running the game uses data from the level files (confirmed via logging output showing loaded wall + widget counts).
- No synthetic corner spawn widgets appear when level defines spawn points.
- Universal walls always present even if not referenced in registry.
- Gravity widget fallback only triggers when no attractors defined.
- Clear warnings on any recovered validation cases.

### Implementation Notes / Constraints
- Keep public API surface minimal; internal modules can stay private inside `core::level` except plugin and resource types.
- Avoid over-generalizing shape representation until >1 shape is required.
- Do not introduce new dependencies beyond `serde` & existing crates.
- All new types derive `Debug` and relevant serde traits.

### Open Questions (Can Defer with Defaults)
| Question | Default Assumption |
|----------|--------------------|
| Limit max walls? | No hard cap; rely on physics perf. |
| Max widget count? | Implicit (vector length). |
| Distinguish repulsor now? | Not yet (only Attractor). |
| Global spawn cap location? | Remains in `GameConfig.spawn_widgets.global_max_balls`. |

### Deprecation Warnings (Exact Strings)
- "LevelLoader: ignoring GameConfig.spawn_widgets.widgets (data-driven widgets present)."
- "LevelLoader: gravity.y legacy value ignored (attractors defined)."
- "LevelLoader: duplicate widget id {id}, skipping subsequent occurrence."
- "LevelLoader: wall segment endpoints identical; skipped."

---
End of prompt.
