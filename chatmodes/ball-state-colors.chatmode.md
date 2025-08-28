## Ball State & Dual Palette Integration Prompt

<!--
Purpose: Implement Enabled/Disabled ball state machine, secondary palette based coloring with tweening, and non-merging visual separation for non-poppable clusters without modifying shaders.
Audience: AI assistant contributing to this Bevy/Rust codebase (see Copilot instructions).
CRITICAL: Follow existing architectural patterns (plugins, system sets, config validation) and performance guidelines.
-->

### <context>
The project renders large numbers of balls using Bevy + Rapier + a unified metaballs WGSL shader. Clusters are computed each frame (`compute_clusters`) and cluster popping interaction logic exists (min ball count & total area). We now need explicit per-ball state (Enabled / Disabled) with tweened color transitions and clear visual distinction for non-clickable (Disabled) balls/clusters. Non-clickable balls must not visually merge (metaball field blending) so users can instantly recognize actionable clusters.
</context>

### <goals>
You WILL implement:
1. A `BallState` component storing current enabled flag and the time of last state change (seconds from `Time::elapsed_secs`).
2. A secondary fixed color palette (artist-defined) parallel to existing `BASE_COLORS` to use for Disabled state (rather than algorithmic darkening).
3. Tweening between primary (Enabled) and secondary (Disabled) colors over a configurable duration.
4. Logic to classify clusters as Enabled (poppable) or Disabled (non-poppable: size/area below thresholds) each frame after clusters are computed.
5. Rendering logic that:
   - Uses shared cluster color slot for Enabled clusters (merging behavior unchanged).
   - Assigns a UNIQUE color slot per Disabled ball so their fields do not accumulate/merge ("reject each other").
6. Config additions exposing tween duration (seconds). Secondary palette colors are static constants; no config for them initially.
7. Validation and graceful handling of edge cases (e.g., exceeding `MAX_CLUSTERS`).

### <nonGoals>
You MUST NOT:
- Change WGSL shader interface or uniform binary layout.
- Introduce runtime panics on config issues (follow existing validation pattern).
- Add per-frame heap growth or excessive logging.
- Alter existing cluster pop gameplay semantics beyond coloring/state tagging.

### <configChanges>
Add to `GameConfig` (and `Default`, `serde(default)`, validation):
```rust
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct BallStateConfig {
    pub tween_duration: f32, // seconds, > 0
}
impl Default for BallStateConfig { fn default() -> Self { Self { tween_duration: 0.35 } } }
```
Extend `GameConfig` with `pub ball_state: BallStateConfig`. Validation rules:
- Warn & clamp `tween_duration` to 0.01 if <= 0.

### <palette>
Modify `rendering/palette/palette.rs`:
```rust
pub const BASE_COLORS: [Color; N] = [ ...existing... ];
pub const SECONDARY_COLORS: [Color; N] = [
    // Artist-defined darker/alternative variants matching indices of BASE_COLORS.
    // Choose visually distinct yet harmonious colors (avoid pure brightness scaling artifacts).
    Color::srgb(0.55, 0.08, 0.12),
    Color::srgb(0.10, 0.36, 0.62),
    Color::srgb(0.70, 0.70, 0.10),
    Color::srgb(0.10, 0.52, 0.30),
];
#[inline] pub fn secondary_color_for_index(i: usize) -> Color { SECONDARY_COLORS[i % SECONDARY_COLORS.len()] }
```
Retain `color_for_index` unchanged for Enabled state.

### <component>
In `core/components.rs` add:
```rust
#[derive(Component, Debug, Copy, Clone)]
pub struct BallState {
    pub enabled: bool,
    pub last_change: f32,
}
impl BallState { pub fn new(now: f32) -> Self { Self { enabled: true, last_change: now } } }
```
Attach lazily (insert if missing) in state update system to avoid touching spawn code immediately.

### <classificationLogic>
Add a system `update_ball_states` in a new `BallStatePlugin` (place under `gameplay/state/` or `physics/clustering/` module; keep separation). System runs in `Update` after `compute_clusters` and inside `PostPhysicsAdjustSet` to ensure clusters just computed.

Pseudo:
```rust
fn update_ball_states(
    time: Res<Time>,
    clusters: Res<Clusters>,
    cp_cfg: Res<GameConfig>,
    mut q: Query<(Entity, Option<&mut BallState>)>,
) {
    let cp = &cp_cfg.interactions.cluster_pop; // cluster pop assumed enabled normally
    let now = time.elapsed_secs();
    // Build map entity -> cluster info (size, area) by iterating clusters
    for cl in clusters.0.iter() {
        let enabled = cl.entities.len() >= cp.min_ball_count && cl.total_area >= cp.min_total_area;
        for &e in &cl.entities { /* set BallState */ }
    }
    // Orphans (if any) not in clusters (should not happen) => treat enabled.
}
```
State change detection: If `prev.enabled != enabled`, set `ball_state.enabled = enabled` and `ball_state.last_change = now`.

### <tweening>
Interpolation done during material assembly (NOT stored per-frame to avoid duplicate state). Use linear interpolation on linear RGBA arrays:
```rust
fn lerp_color(a: Color, b: Color, t: f32) -> Color { /* convert to linear, mix, return Color::linear_rgba */ }
```
Calculate `t = ((now - last_change) / tween_duration).clamp(0.0, 1.0)`.
If currently enabled: color = lerp(disabled_color, enabled_color, t).
Else: color = lerp(enabled_color, disabled_color, t).

### <materialUpdate>
Modify `update_metaballs_unified_material`:
1. Collect Enabled clusters first:
   - Assign one color slot per enabled cluster (color slot = index into `cluster_colors`).
   - Record mapping `Entity -> slot` for all balls in that cluster; compute tween color using any (first) ball's state (they should share state by definition; if mismatch, choose majority or recompute first-consistent, but mismatch should not occur).
2. Collect Disabled clusters:
   - For each ball, allocate its own color slot (tween per ball).
3. Populate `mat.data.cluster_colors[slot]` with computed color (srgb values stored like existing).
4. Populate `mat.data.balls[i] = Vec4(x, y, radius, slot as f32)`.
5. Track total slot count <= `MAX_CLUSTERS`; on overflow:
   - Log (target: "metaballs") once per run (store bool) and fallback to grouping disabled balls by their base palette index (merging may reappear but safe).

### <shaderImpact>
No WGSL changes required because per-ball isolation is achieved by giving each disabled ball a unique cluster index, preventing accumulation in `accumulate_clusters` (logic aggregates by identical cluster index only).

### <systemOrdering>
Ordering summary:
```
compute_clusters (PostPhysicsAdjustSet)
  -> update_ball_states (after compute_clusters, same set)
  -> update_metaballs_unified_material (already later in frame)
```
Ensure plugin insertion order preserves this chain (add `BallStatePlugin` after cluster plugins in `GamePlugin`).

### <validation>
Add validation entries:
- `ball_state.tween_duration <= 0` -> warn & clamp.

### <testing>
Add minimal tests:
1. Pure function tests for `lerp_color` (t=0,0.5,1) & palette secondary mapping.
2. App test creating < min cluster size -> states disabled & separate slots (count of cluster_colors equals number of balls). Then add balls to exceed threshold -> all share one slot & states enabled.

### <logging>
Log at INFO (target: "ball_state") only when a ball transitions (throttle? only if debug feature enabled) or when cluster slot overflow fallback triggers.

### <edgeCases>
- Zero clusters: no updates (early return).
- Disabled tween_duration ~ very small: effectively instant color swap.
- Rapid oscillation (ball entering/leaving threshold): last_change updated each toggle; smooth cross-fade restarts.
- Overflow of slots: documented fallback.

### <performance>
Keep per-frame allocations bounded: reuse temporary Vecs sized via `with_capacity(ball_count)` inside system; they drop each frame but avoid repeated growth. Avoid formatting inside loops.

### <successCriteria>
You MUST verify:
1. Enabled clusters visually merge (current look preserved).
2. Disabled clusters & single balls show secondary colors and do NOT merge (discrete blobs).
3. Transition between states smoothly interpolates colors over configured duration.
4. No shader modifications; build passes native & wasm.
5. No panics / warnings except intended validation messages.

### <implementationSteps>
1. Add config struct & integrate into `GameConfig` + validation.
2. Add secondary palette constants & helper function.
3. Add `BallState` component.
4. Implement `BallStatePlugin` & `update_ball_states` system (ordering after `compute_clusters`).
5. Refactor metaball material update system for dual path (enabled clusters vs disabled per-ball slots) + tween.
6. Add color math helpers (inline or small module).
7. Add tests.
8. Update README (feature bullet) & CHANGELOG.
9. Run `cargo clippy --all-targets --all-features` & fix new warnings.
10. Spot test runtime: confirm visual behavior.

### <notes>
Future extension: Provide config toggle for isolation strategy (per-ball slot vs radius shrink) and allow dynamic tertiary palette for “about-to-pop” highlighting.

---
You MUST now implement exactly as specified above unless instructed otherwise by a follow-up prompt.
