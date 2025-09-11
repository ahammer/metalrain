# Compute No‑Op Metaballs Prepass Prompt

Tags: #codebase #file:metaballs_unified.wgsl

Goal: Introduce a no‑op compute shader pass that *always executes* (and completes) each frame **before** the existing metaballs unified material render, without changing visuals or altering current uniform / binding layouts.

Acceptance:
- Game runs identically (pixel output unchanged).
- Compute pass dispatches exactly once per view (or once per frame) before the metaballs material draw.
- No changes to `metaballs_unified.wgsl` content or binding order.
- WASM + native both load the compute shader (embed on wasm).
- Safe no‑op: zero side effects; future extension point.

## Required Steps (Implement Exactly)
1. Add new WGSL shader file at `assets/shaders/metaballs_noop_compute.wgsl` containing:
   ```wgsl
   // Metaballs precompute no-op (placeholder)
   // Workgroup size 1 to minimize overhead.
   struct MetaballsData { v0: vec4<f32>; v1: vec4<f32>; v2: vec4<f32>; v3: vec4<f32>; v4: vec4<f32>; v5: vec4<f32>; v6: vec4<f32>; v7: vec4<f32>; };
   @group(2) @binding(0) var<uniform> metaballs: MetaballsData; // match existing layout for forward compatibility (unused)
   @compute @workgroup_size(1)
   fn cs_main() { /* intentionally empty */ }
   ```
   Rationale: Bind group(2) binding(0) matches existing uniform struct so later we can read/update without reworking layout.

2. WASM embedding:
   - In `MetaballsPlugin` (file `src/rendering/metaballs/metaballs.rs`), mirror existing embedded shader pattern:
     ```rust
     #[cfg(target_arch = "wasm32")] static METABALLS_NOOP_COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
     // During plugin build (wasm block): add Shader::from_wgsl(include_str!("../../../assets/shaders/metaballs_noop_compute.wgsl"), "metaballs_noop_compute_embedded.wgsl") and store in OnceLock.
     ```

3. Render world resources:
   - Define a new `MetaballsNoopComputePipeline` resource (render world) holding:
     ```rust
     pub struct MetaballsNoopComputePipeline { pub pipeline_id: Option<CachedComputePipelineId>, pub shader: Option<Handle<Shader>> }
     ```
     Default with `None` entries.

4. Pipeline preparation system (in `Render` schedule):
   - If `shader` None: load asset (native) via `asset_server.load("shaders/metaballs_noop_compute.wgsl")` else wasm handle from OnceLock.
   - If `pipeline_id` None: queue compute pipeline:
     ```rust
     let layout = vec![]; // no bind groups needed (uniform unused)
     let desc = ComputePipelineDescriptor { label: Some("metaballs.noop.compute".into()), layout, shader: shader_handle.clone(), entry_point: Cow::from("cs_main"), shader_defs: vec![] };
     pip.pipeline_id = Some(pipeline_cache.queue_compute_pipeline(desc));
     ```
     NOTE: Omit bind group layout to avoid forcing group(2) creation now; shader has an unused binding0 reference — acceptable because we DO NOT actually bind. If validation requires bound layout, alternatively create a layout with matching uniform bind (copy from material pipeline). Prefer minimal first; adjust only if wgpu validation fails.

5. Render graph node:
   - Create label `MetaballsNoopComputeNode` implementing `Node`.
   - In `run()`:
     ```rust
     let pipe_res = world.resource::<MetaballsNoopComputePipeline>();
     let Some(pid) = pipe_res.pipeline_id else { return Ok(()); };
     let cache = world.resource::<PipelineCache>();
     let Some(pipeline) = cache.get_compute_pipeline(pid) else { return Ok(()); };
     let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("metaballs_noop_precompute") });
     pass.set_pipeline(pipeline);
     pass.dispatch_workgroups(1,1,1);
     ```
     No bindings set.

6. Graph insertion ordering:
   - In render sub-app setup (after pipeline init), mutate `Core2d` subgraph:
     ```rust
     let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
     let sub = graph.get_sub_graph_mut(Core2d).unwrap();
     sub.add_node(MetaballsNoopComputeNodeLabel, MetaballsNoopComputeNode::default());
     // Edges: place before main metaballs draw (MainPass2d). Use existing enum variants:
     let _ = sub.add_node_edge(Node2d::StartMainPass, MetaballsNoopComputeNodeLabel);
     let _ = sub.add_node_edge(MetaballsNoopComputeNodeLabel, Node2d::MainPass);
     ```
     If `Node2d::StartMainPass` not present (version drift), fallback: edge from `Node2d::Prepass` or earliest available node preceding `MainPass`.

7. System registration:
   - In plugin `build` after material plugin insertion:
     - In app world: nothing extra.
     - In render sub-app: `init_resource::<MetaballsNoopComputePipeline>()` and `.add_systems(Render, prepare_noop_compute_pipeline);` before graph wiring.

8. Logging (debug only):
   - Once per first successful dispatch, log: `info!(target="metaballs", "No-op compute prepass active");` Guard with atomic / bool flag in pipeline resource (e.g., `logged: bool`). Do not log every frame.

9. Tests (optional smoke):
   - Add a test asserting pipeline queued (resource exists with Some(pipeline_id)) after one frame. (Skip if test harness not already building render graph easily.)

10. Performance: This pass is trivial; confirm no measurable frame time delta (<0.05 ms typical). Keep workgroup size=1.

## Non-Goals / Must NOT
- Do NOT modify `metaballs_unified.wgsl` content.
- Do NOT reorder existing material bind group layouts.
- Do NOT introduce dynamic allocations or per-frame logging.
- Do NOT change visual output or alpha semantics.

## Success Criteria Checklist
- [ ] Application runs (native + wasm) with no validation errors.
- [ ] Compute node executes before metaball material render (verify via temporary debug log order).
- [ ] Visual output binary identical (manual A/B or unaltered golden screenshot tests pass).
- [ ] No new warnings in wgpu logs about missing bind groups (if warnings occur, supply minimal bind group layout & bind a dummy uniform buffer).

## Future Extension Notes
- This node becomes staging area for:
  - SDF normal precomputation.
  - Cluster reduction into SSBO for fragment.
  - Early field culling masks.

Implement exactly as above. If validation complains about unused uniform binding, revise pipeline layout to include matching uniform bind group copied from material bind layout (defer until needed).
