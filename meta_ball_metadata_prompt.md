## Metaballs Metadata Shader Implementation Prompt

You WILL implement a new metaballs rendering mode and shader whose fragment output encodes per‑pixel metadata (NOT a final visual). This mode is used for compositional post pipelines and interaction (picking) later. Do NOT optimize prematurely beyond current project conventions.

### Purpose
Produce an RGBA output where each channel encodes structured information:
R (Distance Field): A normalized signed distance (SDF proxy) to the metaball iso‑surface (0.5 = on surface).
G (Clickable Mask): 1.0 where pixel is inside any clickable metaball aggregate; 0.0 otherwise.
B (Non‑Clickable Mask): 1.0 where pixel is inside any non‑clickable metaball aggregate; 0.0 otherwise.
A (Color Index): Encoded cluster (or orphan ball) color index in [0,1]. Later stages will decode to u8 = floor(A * 255 + 0.5).

Initially (bootstrap phase) ALL metaballs are treated as clickable, so:
G = classic mask
B = 0.0

You MUST structure the implementation so future differentiation (per‑ball click flags) requires minimal changes.

### Constraints & Alignment
1. You MUST keep uniform buffer layout fully compatible with existing `metaballs_unified.wgsl` (same @group(2)/@binding indices 0..2) to reuse `MetaballsUnifiedMaterial` unless a compelling reason emerges.
2. You MUST add a new foreground mode enum variant `Metadata` in Rust (`MetaballForegroundMode`) so runtime toggling works like existing modes.
3. You MUST keep WASM embedding pathway (OnceLock handles) intact; if adding a new shader file, embed for wasm target just like the unified shader(s).
4. You MUST NOT break existing modes; all prior visual output remains identical unless `Metadata` mode selected.
5. You MUST keep all loops bounded by `MAX_BALLS`, `MAX_CLUSTERS`, `K_MAX` constants (no dynamic unbounded loops).
6. You MUST maintain 16‑byte alignment in WGSL uniform structs (reuse existing packing; do NOT add scalars mid‑struct). Any new scalar control should reuse spare vec4 lanes (e.g., `v1.w` currently debug_view) or surface noise flags if safe. Prefer repurposing `debug_view` only if NOT already needed simultaneously—otherwise introduce a separate material or dedicated lane after structured review.

### High‑Level Steps
1. Add a new shader file `assets/shaders/metaballs_metadata.wgsl` (or integrate logic into unified shader guarded by `fg_mode == Metadata`). Prefer unified integration to avoid pipeline swaps.
2. Extend `MetaballForegroundMode` enum with `Metadata` variant; update `ALL` array and any mode cycling logic (ensure key cycling still works; consider placing Metadata at end to avoid disrupting user muscle memory).
3. Update config mapping if necessary (`GameConfig.metaballs_shader.fg_mode`) to allow selecting metadata mode; clamp logic already uses `ALL.len() - 1` so it will function automatically.
4. Inside WGSL fragment: when `fg_mode == Metadata` you WILL bypass normal background compositing and write metadata directly. Return the RGBA metadata vector (opaque alpha semantics replaced by encoded index). Skip mixing with background color.
5. Implement field accumulation using existing `accumulate_clusters` logic. Reuse best cluster detection.
6. SDF Approximation:
   - Let `f_best = best_field` for dominant cluster (or optionally total field—document choice). For consistency with masks compute mask from best cluster (current logic).
   - Let `iso` threshold from uniforms.
   - Let `grad_vec = grad` (dominant gradient).
   - Compute gradient length `g_len = max(length(grad_vec), 1e-5)`.
   - Signed distance proxy: `signed_d = (iso - f_best) / g_len`. This yields positive outside, negative inside.
   - Normalize to [0,1]: Choose a scale window `d_scale` (MANDATORY constant for now, e.g. 4.0 * radius_scale or empirically 8.0 pixels). Then:
     `r_channel = clamp(0.5 - 0.5 * signed_d / d_scale, 0.0, 1.0)`.
   - You MUST document chosen constant near code with rationale and TODO for adaptive scaling (future improvement: per‑pixel curvature aware scaling or noise modulation).
7. Clickability:
   - Bootstrap: treat all clusters as clickable. Set `click_mask = mask` and `non_click_mask = 0.0`.
   - Future: Add per‑ball flag. Reserve approach: encode a bit in the w component of ball vec4 (currently holds cluster index). Plan: reinterpret cluster_index float as `cluster_index + (flag * 1024)` where flag=1 denotes non‑clickable or clickable. Document this packing plan in comments WITHOUT implementing yet.
8. Color Index Encoding (A channel): Use cluster slot index (the same value currently stored in `b.w` when writing balls) normalized to [0,1] with 1/255. Guarantee clamp to <= 255 else wrap via `min(cluster_idx, 255u)`. Write as float: `a = f32(cluster_u8) / 255.0`.
9. Edge Cases:
   - If `acc.used == 0u`: output (R=1.0,G=0,B=0,A=0) representing “far outside any field” (distance normalized to outside). Document sentinel.
   - If gradient length extremely small (<1e-5): force `r_channel = select(0.5, r_channel, false)` (i.e., set to 0.5) to avoid banding / NaN.
   - If ball_count == 0: early return sentinel.
10. Tests:
   - Pure function test for normalization mapping (extract small helper in Rust mirroring WGSL formula for signed distance mapping; compare expected center pixel inside: distance mapped >0.5, far pixel <0.5?).
   - Ensure new enum variant count increments; test cycling wraps properly.
   - Shader load test: instantiate app, select metadata mode, run a frame, ensure material `v1.y` equals new variant discriminant.
11. Performance: Early exit branch for non‑metadata modes unchanged; metadata path avoids expensive lighting / background noise, so net cost is not higher.
12. Logging: Log once when entering metadata mode: `info!(target="metaballs", "Foreground mode -> Metadata (metadata RGBA output)")` following existing pattern.

### WGSL Fragment (Pseudocode Snippet)
<!-- <example> -->
```
if (fg_mode == FG_MODE_METADATA) {
    if (acc.used == 0u) { return vec4<f32>(1.0, 0.0, 0.0, 0.0); }
    let dom = dominant(acc);
    let f_best = acc.field[dom];
    let gradv = acc.grad[dom];
    let g_len = max(length(gradv), 1e-5);
    let signed_d = (iso - f_best) / g_len;
    let d_scale = 8.0; // TODO: tie to typical radius & pixel density
    let r_channel = clamp(0.5 - 0.5 * signed_d / d_scale, 0.0, 1.0);
    let mask = compute_mask(f_best, iso); // existing ramp
    // Bootstrap click logic
    let clickable = mask; // all clickable
    let non_clickable = 0.0;
    let cluster_idx = acc.indices[dom];
    let cluster_u8 = min(cluster_idx, 255u);
    let a_channel = f32(cluster_u8) / 255.0;
    return vec4<f32>(r_channel, clickable, non_clickable, a_channel);
}
```
<!-- </example> -->

### Future Clickability Plan (Document NOW, Implement LATER)
You WILL store per‑ball clickability in a spare flag bit. Candidate approach:
1. Use high range of radius (unlikely) or pack into cluster index lane: store as `b.w = cluster + (clickable_flag * 4096)`.
2. In shader: `let raw = u32(b.w + 0.5); let clickable = ((raw / 4096u) & 1u) == 1u; let cluster = raw & 4095u;` (cap cluster slots <= 4095; current max 256 so safe).
3. Aggregate separate fields for clickable vs non‑clickable by branching inside accumulation: maintain two parallel accumulators OR single accumulator plus bitset arrays.
4. Then set G = clickable_mask, B = non_clickable_mask.
You MUST revisit uniform packing if additional flags exceed encoded capacity; prefer adding explicit bit flags only after evaluating maintainability.

### Rust Integration Details
1. Enum update:
   ```rust
   pub enum MetaballForegroundMode { ClassicBlend, Bevel, OutlineGlow, Metadata }
   pub const ALL: [Self; 4] = [Self::ClassicBlend, Self::Bevel, Self::OutlineGlow, Self::Metadata];
   ```
2. Cycling logic automatically works; no extra change except length.
3. When mode is Metadata, skip blending in Rust host code (no change required if shader handles direct output). Ensure alpha mode stays default (opaque) since we want unmodified RGBA data; mixing would corrupt metadata.
4. If existing pipeline enforces mixing FG over BG, adjust shader to return BG unmodified only when not metadata. (Current design: FG mix applied unconditionally at end; adapt by gating that mix.)
   - Add conditional before final mix: if metadata mode, return metadata_vec early.
5. Optional: Implement a helper `fn is_metadata(&Res<MetaballForeground>) -> bool` for clarity.

### Fallback / Debug Considerations
If debug_view flag currently used (v1.w): Leave unchanged; metadata is orthogonal. If both debug_view & metadata selected, metadata takes precedence (explicit early return) and you MUST document that debug grayscale view is suppressed in metadata mode.

### Validation / Success Criteria
You MUST confirm:
1. Build passes (native + wasm) with no new clippy warnings related to changed code.
2. Mode cycling includes Metadata; log line appears once when entered.
3. Pixel in clear background returns sentinel (1,0,0,0).
4. Pixel near blob center returns R < 0.5 or > 0.5? (Inside surface we expect negative signed_d => r_channel > 0.5). Provide a quick screenshot or numeric sample in dev log.
5. A cluster index N yields alpha approximately N/255 (within float error < 0.5/255).
6. Performance: Frame time not measurably worse than ClassicBlend (metadata path cheaper—no lighting / noise). Spot test with 1000 balls.

### Potential Risks & Mitigations
Risk: SDF approximation quality near flat gradients. Mitigation: fallback to 0.5 (surface) when |grad| tiny.
Risk: Future addition of per‑ball flags complicates cluster packing. Mitigation: reserved packing scheme documented now.
Risk: Post‑processing expecting premultiplied colors misreads metadata. Mitigation: For metadata mode downstream passes MUST treat texture as raw data (document in pipeline integration spec later).

### Tooling (MCP) Guidance (HOW YOU WILL IMPLEMENT & VERIFY)
You WILL leverage available MCP / assistant tools to accelerate safe, correct implementation:

1. Locate Relevant Code
   - Use `semantic_search` with queries: "MetaballForegroundMode", "update_metaballs_unified_material", "MetaballsUnifiedMaterial" to gather all touch points before editing.
   - Use `grep_search` (fast path) for exact tokens: `grep_search: "MetaballForegroundMode" includePattern=src/**` to confirm no missed enum usages.

2. Add Enum Variant
   - After editing `metaballs.rs`, re-run `grep_search` for the enum to ensure new discriminant is compiled in all match / indexing logic.

3. Shader Authoring
   - Copy structure from `assets/shaders/metaballs_unified.wgsl` using `read_file` (if not in context) to preserve binding order. Add metadata branch.
   - Use `grep_search` for `@group(2) @binding(0)` across shaders to ensure uniform layout consistency.

4. Bevy API Reference
   - Use `context7` docs retrieval: resolve Bevy library (already resolved) then `get-library-docs` topic: "Material2d Trait Methods" or "shader material 2d" when uncertain about extending material behavior.

5. Build & Lint Cycle
   - Run `cargo check` and then `cargo clippy --all-targets --all-features -q` via terminal after edits. If WASM target required, run `cargo build --target wasm32-unknown-unknown` (optional early compile to catch shader path issues under cfg).

6. Runtime Smoke Test
   - Run `cargo run` and cycle foreground modes (Home/End keys) until Metadata log appears. Capture log line (search in terminal output) to confirm instrumentation.

7. Shader Hot Reload Validation (Native)
   - Modify a comment inside the new WGSL file while app runs (if hot reload enabled by asset server) to confirm pipeline picks changes (optional; if not configured, skip).

8. Unit Tests
   - Add a pure helper in Rust replicating the mapping: `fn map_signed_distance(d: f32, scale: f32) -> f32 { (0.5 - 0.5 * d / scale).clamp(0.0, 1.0) }`.
   - Use `cargo test --lib sdf` (naming a test module) to ensure expected mapping (inside negative => >0.5, outside positive => <0.5, zero => 0.5).

9. Regression Scan
   - After changes, run `semantic_search` with query: "return vec4<f32>(out_rgb" to ensure only visual modes still composite color while metadata early returns.

10. Documentation Sync
   - If README enumerates modes, search with `semantic_search` query: "Foreground mode" and update if necessary.

11. Performance Spot Check
   - (Optional) Run release build: `cargo run --release` then observe frame time (if diagnostics available). Compare enabling / disabling metadata mode; no regression expected.

12. Future Clickability Flag Prep
   - Before implementing flag packing, use `grep_search` for any existing bit packing patterns to stay consistent.

You MUST record any deviations or tool errors as comments in the PR or commit message to maintain traceability.

### TODO Tags to Insert in Code
// TODO: metadata-mode SDF scaling adapt to radius & screen resolution.
// TODO: implement per-ball clickability flag packing (see meta_ball_metadata_prompt.md).
// PERF: confirm metadata path branch cost negligible vs existing.

### Deliverables (This Task)
You WILL ONLY add this prompt file now. NO shader or Rust code changes yet. Next task will enact these instructions.

### Ready For Implementation When
Stakeholder signs off on channel semantics (R/G/B/A). No further changes requested here.

---
Authored: Metadata Mode Prompt v1.0
Date: (auto‑generated)
Status: Draft – awaiting implementation task.
