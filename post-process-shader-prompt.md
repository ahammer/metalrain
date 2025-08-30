## Two-Phase Metaballs Post-Process Pipeline Prompt (Inversion PoC)

You WILL implement a two‑phase metaball rendering pipeline that separates (1) the existing metaballs unified material pass (foreground/background composition exactly as today) from (2) a subsequent full‑screen post‑process pass that (for this PoC) simply inverts the color output. This establishes the render graph + pipeline scaffolding for future, more complex screen‑space effects (bloom, palette remap, metadata compositing, picking overlays, etc.).

For this task you WILL only follow these instructions when implementing (a future task). This prompt codifies all required architectural, shader, and Rust side changes for the minimal inversion effect.

Status: Draft v1.0  
Date: (auto‑generated)  
Scope: Post‑process pipeline scaffold + inversion shader (no extra effects yet)

---

### 1. Purpose & High‑Level Overview
1. Preserve existing metaballs rendering path producing an sRGB (or linear internally) color buffer.
2. Introduce a post‑processing pass that samples the camera / view texture and writes to the final surface after inversion: `out.rgb = 1.0 - in.rgb; out.a = in.a`.
3. Keep the change optional / toggleable (`GameConfig.metaballs_post.invert_enabled` – default false) to avoid altering current visuals automatically.
4. Reuse Bevy 0.16 post‑process facilities (#context7 post_process module) with a custom node & specialized pipeline (fullscreen triangle) to minimize boilerplate.

### 2. Constraints & Alignment (MANDATORY)
1. You MUST NOT change existing metaballs shader uniform layouts or material indices.
2. You MUST insert the post‑process step after the metaballs have rendered but before final presentation (i.e., after tonemapping / core pipeline if compatible). For simplicity, place after built‑in post_process stack or integrate as a custom node attached to the camera view graph.
3. You MUST gate inversion with a runtime resource / config bool; disabled path adds near‑zero overhead (node early exit or not added to graph).
4. You MUST design the node so future chaining (multiple passes) can extend via additional enums / effect list without re‑wiring low‑level render graph fundamentals.
5. You MUST ensure WASM build compatibility: embed the inversion WGSL similarly to metaballs shaders using `OnceLock` when on `wasm32`.
6. You MUST keep alpha channel unmodified (pass through) because later blending / UI overlays may rely on it.
7. You MUST keep the implementation additive and avoid refactoring unrelated rendering code in this first step.

### 3. Architectural Additions
Component / Resource / Plugin Summary:
* `struct PostProcessToggle { invert: bool }` (Resource) – inserted from config.
* `pub struct MetaballsPostProcessPlugin;` – registers extraction + render graph node + pipeline.
* `struct InversionPostProcess;` (marker component) – tagged onto the primary 2D camera (or use resource-based control if you prefer global toggle).
* (Future) A settings uniform buffer for more effects; NOT needed for inversion (no uniforms besides the source texture & sampler).

Render Graph Integration:
1. On plugin build (in render app world), add a custom render graph node after the main pass output. In Bevy 0.16 you can either:
   - Use the built‑in post processing pattern (`ViewTarget::post_process_write()`) inside a custom `Node` implementation (see #context7 examples: the repeated `post_process_write()` code snippet) OR
   - Add a simple fullscreen pass reading the main view texture before final copy.
2. The node acquires source/destination via `view_target.post_process_write()` ensuring double buffering semantics are respected.
3. The node binds:
   - Binding 0: `texture_2d<f32>` source view
   - Binding 1: `sampler` (linear clamp)
   (No uniforms -> pipeline layout with a single bind group layout of two entries.)
4. The node dispatches a fullscreen triangle (draw 3 vertices) – no vertex buffer required.

### 4. Shader Specification (`assets/shaders/post_invert.wgsl`)
You WILL create a WGSL shader containing both vertex & fragment stages:
```wgsl
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;

struct VSOut { @builtin(position) pos: vec4<f32>; @location(0) uv: vec2<f32>; };

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VSOut {
    // Fullscreen triangle (NDC): ( -1,-3 ), ( -1, 1 ), ( 3, 1 )
    var positions = array<vec2<f32>,3>(vec2(-1.0,-3.0), vec2(-1.0,1.0), vec2(3.0,1.0));
    let p = positions[vi];
    var out: VSOut;
    out.pos = vec4(p, 0.0, 1.0);
    out.uv = 0.5 * (p + vec2(1.0,1.0));
    return out;
}

@fragment
fn fs(in: VSOut) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(src_tex, src_samp, in.uv, 0.0);
    return vec4(1.0 - color.rgb, color.a);
}
```
WASM Embedding: For `wasm32`, embed via `include_str!("../../../assets/shaders/post_invert.wgsl")` and store in a `OnceLock<Handle<Shader>>` similar to the metaballs shader pattern.

### 5. Rust Side Implementation Steps
1. Config: Extend `GameConfig`:
   ```rust
   #[derive(Serialize, Deserialize, Clone)]
   pub struct MetaballsPostConfig { #[serde(default)] pub invert_enabled: bool }
   // Add to root GameConfig: pub metaballs_post: MetaballsPostConfig
   impl Default for MetaballsPostConfig { fn default() -> Self { Self { invert_enabled: false } } }
   ```
   Validation: none required beyond boolean.
2. Resource Initialization (Startup):
   ```rust
   fn init_post_toggle(mut commands: Commands, cfg: Res<GameConfig>) { commands.insert_resource(PostProcessToggle { invert: cfg.metaballs_post.invert_enabled }); }
   ```
3. Camera Tagging: Modify camera setup to insert marker `InversionPostProcess` if toggle true (or always add and branch inside node).
4. Plugin Build:
   * In `MetaballsPostProcessPlugin::build`, register extraction of marker / toggle into render world (use `ExtractResource` if needed) and add custom render node.
   * Acquire / create `RenderApp` via `app.get_sub_app_mut(RenderApp)` then insert pipeline resources and graph modifications.
5. Pipeline Setup:
   * Define `struct InversionPipeline { layout: BindGroupLayout, pipeline_id: CachedRenderPipelineId, sampler: Sampler }` (insert as resource in render world).
   * Specialize pipeline once on startup (vertex & fragment shader loaded, color target = view main texture format). Retrieve format from `Msaa / ViewTarget` or use standard `TextureFormat::bevy_default()` if accessible.
6. Node Implementation:
   * Node queries `(Entity, &ViewTarget, Option<&InversionPostProcess>)` (or rely on component presence) plus `Res<PostProcessToggle>`.
   * Early exit if toggle false or component absent.
   * Invoke pattern from #context7: get pipeline from `PipelineCache`, create bind group each frame (because source/dest swap), begin pass, set pipeline + bind group, issue `draw(0..3,0..1)`.
7. Ordering: Add node after built‑in `tonemapping` / `post_process` node. If unsure, place near end of `CoreSet::PostUpdate` equivalent for render graph (explicit order using graph edges). Example sketch:
   ```rust
   let mut graph = render_app.world.resource_mut::<RenderGraph>();
   graph.add_node("metaballs_inversion", InversionNode::default());
   graph.add_node_edge(bevy::core_pipeline::core_2d::graph::node::TONEMAPPING, "metaballs_inversion").unwrap();
   graph.add_node_edge("metaballs_inversion", bevy::core_pipeline::core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING).unwrap();
   ```
   (Adjust node labels to actual constants for Bevy 0.16; verify with source if names differ.)

### 6. Performance Considerations
* Fullscreen triangle cost is trivial (<0.1 ms) vs metaballs accumulation; acceptable baseline.
* Only create bind group per frame; no dynamic allocations in hot loops beyond that.
* Branch once per view: if toggle disabled, node returns early (microseconds).
* Future multi‑effect chain: Add a `PostProcessMode` enum + trait or ordered list; each pass reads previous output using the same ping‑pong mechanism.

### 7. Testing & Validation (MANDATORY)
1. Unit Test: None strictly required (shader). Add a Rust test verifying config default (invert disabled) and enabling it attaches marker.
2. Visual Smoke: Run `cargo run`, enable invert via config (or add temporary key to toggle). Confirm overall scene colors are inverted (light areas dark, etc.).
3. Alpha Preservation: Sample a pixel with known transparency (if any UI later) to ensure alpha unchanged.
4. WASM Build: `cargo build --target wasm32-unknown-unknown` to ensure embedded shader compiles (guard behind `cfg(target_arch="wasm32")`).
5. Clippy: Ensure no new warnings in changed modules relating to unused resources when feature disabled.

### 8. Logging & Diagnostics
* Log once on activation: `info!(target="postprocess", "Metaballs inversion post-process enabled");`
* No per‑frame logs.
* Use target `postprocess` distinct from `metaballs` for filtering.

### 9. Future Extensions (Documented, NOT Implemented Now)
* Color grading LUT pass
* Palette remap / quantization
* Bloom prefilter + separable blur chain
* Metadata buffer compositing (mix metadata or highlight clusters)
* FXAA / SMAA custom variant (if built‑in insufficient) – reuse same chain stage pattern
* Multi‑target outputs (store field metrics in R32F for analysis) – would require extra render graph edges.

### 10. Risk & Mitigations
* Node ordering mismatches final expected pass – mitigate by verifying final graph debug print.
* Texture format mismatch – query actual view target format and use that for pipeline color target.
* WASM performance: negligible (single simple shader). If issues arise, allow disabling via config.

### 11. Implementation Checklist (You MUST follow sequentially)
1. Extend `GameConfig` with `MetaballsPostConfig` (default invert=false) + validation.
2. Add `post_invert.wgsl` shader (native path + wasm embedded path).
3. Create `MetaballsPostProcessPlugin` in new module `src/rendering/postprocess/mod.rs`.
4. Initialize pipeline resources (bind group layout, sampler, pipeline) in render sub‑app.
5. Implement `InversionNode` performing:
   * Acquire pipeline (bail if not ready).
   * Call `post_process_write()`.
   * Create bind group with `source` + sampler.
   * Begin pass writing to `destination`.
   * Draw fullscreen triangle (3 vertices).
6. Insert node into graph after tonemapping and before main pass end node.
7. Add toggle resource & camera marker at startup (conditionally).
8. Log activation when toggle true.
9. Run native build & visually confirm inversion.
10. Build WASM target to confirm embedded shader compiles.
11. Document new config field in README (search for existing config section; update sample RON file) – include comment: `// Post-process inversion (debug/testing)`.

### 12. Success Criteria
* When enabled, observed frame colors inverted relative to prior (screenshot diff acceptable).
* When disabled, rendering identical byte‑wise (allowing for pipeline ordering neutrality) – visually no differences.
* No panics; no additional per‑frame allocations beyond bind group creation.
* Clippy passes; tests pass.
* WASM & native both compile successfully.

### 13. TODO / PERF Tags to Insert in Code
```rust
// TODO: extend post-process chain with additional effects (palette, bloom, metadata composite)
// PERF: evaluate batching multiple simple color ops into single shader before adding >3 passes
```

### 14. Reference (#context7)
Implementation mirrors #context7 post-processing node examples (fullscreen triangle, `post_process_write()` usage, pipeline cache handling). Keep layout minimal (texture + sampler only) for this PoC.

---
Deliverable of THIS task: This prompt file only. NO code changes yet.  
Ready for implementation when stakeholder approves scaffold & checklist.
