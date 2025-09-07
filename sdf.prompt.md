<!--
Prompt Name: SDF Metaball Shape Atlas Integration
Purpose: Guide an AI / contributor to (1) generate a Signed Distance Field (SDF) atlas of primitive shapes, (2) load it into the existing Bevy + WGSL pipeline, and (3) replace per-fragment analytic circle field contribution with per-shape SDF sampling so each palette color (cluster) uses a distinct shape instead of always a circle. The end state: metaballs still blend via existing accumulation, but the underlying "ball" silhouette field source is sampled from a texture tile rather than analytically computed, allowing different shapes (circle, square, triangle, diamond, ring, etc.) mapped to palette indices.
Scope: Minimal viable integration without breaking existing clustering, palette, metadata, noise, or debug systems. Must remain WASM-compatible.
CRITICAL: Follow existing architectural & style conventions described in `.github/copilot-instructions.md` and keep binding order stable unless explicitly instructed to extend.
-->

## 1. High-Level Goal
You WILL modify the rendering path so that for each cluster (palette entry) the implicit circle field is replaced by sampling a precomputed single‑channel SDF texture tile (in an atlas). Each cluster chooses a shape index deterministically derived from its `color_index` (or a new mapping), producing visually distinct silhouettes with the existing metaball blending still functioning (field accumulation + iso test + mask). The original analytic circle polynomial stays available behind a feature toggle or fallback for debugging.

Success Criteria:
1. Generator: Running `cargo run --bin gen_sdf_atlas -- --out-dir assets/sdf` produces `sdf_atlas.png` + `sdf_registry.json` (already scaffolded or to be created if missing).
2. Runtime Loader: A new lightweight loader plugin loads atlas image + registry into a resource (`SdfAtlas`).
3. GPU Binding: The atlas is bound as a sampled 2D texture + sampler at new stable binding slots (do NOT disturb existing group(2) layout used by current uniforms/storage buffers). Prefer group(3) or extend group(2) after last used binding ONLY if group(3) architectural constraints appear. Provide both native + WASM support.
4. Shader Update: `metaballs_unified.wgsl` extended with shape sampling function. Accumulation uses per-ball radius only for world->tile UV transform; analytic inside test replaced (or conditionally branched). Palette index -> shape tile UV selection stable & deterministic.
5. Visual: Each distinct palette color produces a different base shape: e.g. palette index 0 = circle, 1 = square, 2 = triangle, 3 = diamond, 4 = ring, cycling through available shapes.
6. Fallback: If atlas resource missing OR shape index out-of-range, revert to existing analytic circle logic.
7. No panics; logging via target `sdf` or `metaballs` with concise one-shot init messages.
8. WASM build still compiles; additional assets copied/included.

## 2. Constraints & Non-Goals
You MUST NOT:
- Break existing metadata mode encodings or palette storage buffer layouts.
- Expand large uniform arrays or exceed current alignment without justification.
- Introduce per-frame allocations in hot accumulation loop.
- Rely on derivative ops (dpdx/dpdy) for the new field; reuse existing AA mask strategy.

You MAY:
- Add a config toggle (RON) `sdf_shapes.enabled` (default true) & `sdf_shapes.force_fallback=false` to gate the feature.
- Add a system set ordering clause consistent with current plugin patterns.

## 3. Implementation Steps
Follow in order. Each step MUST compile before proceeding; run `cargo clippy --all-features` after shader + binding edits.

### Step 1: Add / Confirm SDF Atlas Generator Binary
If not present, create `src/bin/gen_sdf_atlas.rs` (see Section 7 Reference) with primitive shapes (circle, square, triangle, diamond, ring). Output single channel distance encoded as described (0.5 = surface). Store in both R and A for flexibility (already proposed design). Keep command-line args for `--tile-size`, `--range`.

### Step 2: Add Loader Plugin
File: `src/rendering/sdf_atlas.rs` (new module re-export from `rendering::mod.rs`).
Responsibilities:
1. On `Startup`, read `assets/sdf/sdf_registry.json` (synchronous read acceptable).
2. Insert `SdfAtlas` resource with: `image_handle`, `entries: HashMap<String, AtlasEntry>`, `range`, `tile_size`.
3. Log once: `info!(target = "sdf", "Loaded SDF atlas: {} entries (tile={} range={})", entries.len(), tile_size, range);`
4. If missing, warn & set `SdfAtlasMissing` marker or `Option<SdfAtlas>` pattern.
5. Provide helper: `fn shape_uv_for_index(idx: usize) -> Option<[f32;4]>` cycling through sorted entry names to ensure deterministic mapping.

### Step 3: Extend Game Plugin Composition
Insert `SdfAtlasPlugin` before metaball palette upload or at least before first frame that might read it (typically same `Startup` stage). Keep ordering explicit; add `.after(ConfigLoadSet)` if such a set exists (verify actual set names first).

### Step 4: GPU Bindings (Rust Side)
1. Add an `Image` asset handle + `Sampler` to a bind group layout dedicated for atlas sampling. Preferred: new bind group index `@group(3)` to avoid altering existing group(2) structure (choose the smallest free group consistent across pipelines). Confirm no collision by scanning existing pipeline specialization code.
2. Create a small wrapper component / resource `SdfAtlasGpu { texture: Handle<Image>, sampler: Sampler }` inserted when atlas loads.
3. Update pipeline creation (where `metaballs_unified.wgsl` is loaded) to include an optional layout for the atlas group. Expose a flag `has_sdf_atlas` to shader via uniform OR rely on presence test using uniform lane (simpler: extend existing `MetaballsData.v5.x` as boolean if free; otherwise add dedicated small uniform struct). If extending `v5` ensure alignment and meaning documented.

### Step 5: Shader Integration
Edit `assets/shaders/metaballs_unified.wgsl`:
1. Add at bottom of existing uniform/storage definitions:
```
@group(3) @binding(0) var sdf_atlas_tex: texture_2d<f32>;
@group(3) @binding(1) var sdf_atlas_sampler: sampler;
```
2. Introduce constants / function:
```
fn sample_shape_sdf(tile_uv: vec2<f32>, uv_rect: vec4<f32>, atlas_enabled: bool) -> f32 {
    if (!atlas_enabled) { return 0.0; } // neutral; caller handles fallback
    let uv = vec2<f32>(
        mix(uv_rect.x, uv_rect.z, tile_uv.x),
        mix(uv_rect.y, uv_rect.y + (uv_rect.w - uv_rect.y), tile_uv.y)
    );
    let v = textureSample(sdf_atlas_tex, sdf_atlas_sampler, uv).r;
    return v; // 0..1, 0.5 == iso
}
```
3. Replace analytic per-ball interior polynomial in `accumulate_groups_tile` with conditional path when atlas enabled:
   - Compute local tile space coordinate for point `p` relative to ball center: `d = p - center;` normalized radius domain -> tile UV = `(d / scaled_r * 0.5) + 0.5`.
   - Reject early if uv outside [0,1] (skip contribution) (keeps tile square cropping consistent for non-circular shapes).
   - Fetch shape uv rect for cluster (or for ball's material/cluster index). You MUST pass shape index via cluster->color mapping: `shape_index = (cluster_color_index % SHAPE_COUNT)`.
   - Convert SDF sample to field contribution consistent with legacy: current field fi = (1 - d2/r2)^3. We need a monotonic 0..1 inside mapping from SDF value `s` (0..1 where 0.5 surface). Proposed mapping: `let signed = (s - 0.5) * 2.0; let inside = clamp(0.5 - signed * 0.5, 0.0, 1.0); let fi = pow(inside, 3.0);` This preserves softness & iso interplay.
   - Preserve gradient path: when needs_gradient true and atlas path used, approximate gradient numerically with small offset epsilon in tile UV (2 extra samples) OR keep existing analytic gradient for fallback when circle used. For MVP you MAY set gradient = vec2(0) for SDF shapes when bevel/metadata modes disabled; but since bevel & metadata rely on gradient you SHOULD implement finite difference with `eps = 1.0 / 64.0` in tile space scaled by inverse radius to world.
4. Add boolean uniform lane `atlas_enabled` (from Rust). If false, keep legacy analytic logic.
5. Document new mapping with `// TODO(SDF_GRAD_OPT): Replace finite difference with atlas derivative atlas packing for performance.`

### Step 6: Rust Field Mapping
Where palette is built (`metaballs.rs` palette upload logic), capture `color_indices` already present. Use those indices to derive shape index on GPU: Provide a small storage buffer OR encode into existing palette color alpha lane if currently always 1.0 (but alpha is meaningful for color). Safer: introduce a parallel tiny storage buffer `group_shape_indices: array<u32>` at new binding after palette (requires shader change) OR reuse high bits of cluster id (avoid—affects metadata). Minimal path: Add new storage buffer at `@group(2) @binding(7)` with `u32` per palette entry; extend pipeline layout accordingly. Populate with `color_index % shape_count` each frame after palette build.

### Step 7: Config Toggle
Extend `GameConfig`:
```
#[serde(default)]
pub struct SdfShapesConfig { pub enabled: bool, pub force_fallback: bool }
impl Default for SdfShapesConfig { fn default() -> Self { Self { enabled: true, force_fallback: false } } }
```
Add field `pub sdf_shapes: SdfShapesConfig` to root config struct + validation logging warning if `enabled && tile_size > 256` (performance caution). Use this to set `atlas_enabled` uniform lane.

### Step 8: Testing / Validation
1. Run generator, ensure assets exist.
2. Launch game; verify log: `Loaded SDF atlas: N entries`.
3. Observe distinct shapes per color (visually). If clusters exceed shape count, shapes repeat cyclically.
4. Toggle `force_fallback=true` in `game.local.ron`; confirm revert to circle silhouettes.
5. WASM: build and load in browser; shapes appear (ensure atlas files copied to `dist`/`wasm` bundle).
6. Regression: debug modes, metadata mode still functional (metadata path uses gradient—confirm numerical gradient not destabilizing iso threshold visually > 1px jitter). Adjust epsilon if artifact observed.

### Step 9: Performance Notes
You MUST avoid extra per-ball heap allocations. Finite difference adds up to 2 extra texture samples for gradient when needed; accept initially. If profiling reveals cost, plan follow-up: pack signed distance & gradient in RG/BA or precompute derivative atlas.

### Step 10: Logging & Error Handling
Only one info log on successful load. Warnings on:
- Missing registry file.
- Shape count zero.
- Shape buffer length mismatch vs palette length (log once; auto clamp to min length).

## 4. Data Contracts
Atlas Registry JSON fields (must match generator):
```
{
  atlas: String,      // filename
  width: u32,
  height: u32,
  tile_size: u32,
  range: f32,
  entries: [ { name, x,y,w,h, uv:[u0,v0,u1,v1] }, ... ]
}
```
Shader expects uv rect as `[u0, v0, u1, v1]` per shape.

## 5. Edge Cases & Handling
- Atlas missing: fallback analytic path; log once.
- Palette grows above shapes: shapes repeat via modulo.
- Tile size not power-of-two: still supported (no assumption in sampling; only gradient epsilon uses `1.0/tile_size`).
- Gradient disabled modes: skip finite difference sampling to save cost.

## 6. Follow-Up (Document but Defer)
- Multi-channel MSDF for crisper edges (RGB for angle correction) – out of scope.
- Per-shape custom iso bias field or thickness parameter.
- Storage buffer refactor merging palette colors + shape index to reduce binding count.
- GPU compute pass to rasterize analytic shapes into a transient texture (fully procedural, removes PNG shipping).

## 7. Reference Snippets (Do NOT copy verbatim if already implemented)
<!--
Rust: shape index buffer population
```
let shape_count = atlas.entries.len().max(1);
let shape_indices: Vec<u32> = cpu_palette.color_indices.iter().map(|ci| (*ci as u32) % shape_count as u32).collect();
// Upload to storage buffer at binding(7)
```

WGSL: converting sampled SDF value to field fraction
```
let s = textureSample(sdf_atlas_tex, sdf_atlas_sampler, uv).r; // 0..1, 0.5 surface
let signed = (s - 0.5) * 2.0; // [-1,1]
let inside01 = clamp(0.5 - signed * 0.5, 0.0, 1.0); // 1 center, 0 outside
let fi = inside01 * inside01 * inside01; // mimic legacy cubic falloff
```
-->

## 8. Completion Checklist
You MUST verify all before merging:
- [ ] Generator binary present & runnable.
- [ ] Atlas + registry loaded; resource inserted.
- [ ] New bind group with texture + sampler established; no binding collisions.
- [ ] Shader compiles with new group/bindings.
- [ ] Config toggle integrated; default enabled.
- [ ] Distinct shapes visible for first 5+ palette entries.
- [ ] Fallback path works when atlas missing or force flag set.
- [ ] Clippy & tests pass; add a minimal test asserting config defaults + shape index modulo mapping for small synthetic palette.
- [ ] README or internal dev docs note new generator & usage.

## 9. Rollback Strategy
Keep all changes guarded by `atlas_enabled` uniform + config toggle. Reverting is deleting the new module + shader lines + removing uniform lane, leaving analytic circle logic intact.

## 10. Final Notes
Maintain determinism: mapping from `color_index` -> shape index MUST stay stable across runs for reproducibility and screenshot diffs. Document mapping inline in code with comment enumerating sequence of shape names.

<!-- End of Prompt -->
