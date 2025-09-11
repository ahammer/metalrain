# Compute No‑Op Metaballs Prepass Prompt

Tags: #codebase #file:metaballs_unified.wgsl

Goal: Introduce a no‑op compute shader pass that executes each frame **before** the existing metaballs unified material render, without changing visuals or altering current material uniform / binding layouts.

Acceptance:
- Visual output identical (pixel parity).
- Compute pass dispatches exactly once per frame (or once per view) **before** the metaballs material draw in the Core2d graph.
- No changes to `metaballs_unified.wgsl` contents or material bind group layouts.
- Works on native + WASM (embedded on WASM).
- No wgpu validation warnings / errors (including bind group index continuity).

## Tested Issues & Corrections (from validation pass 1)
- Using a shader with only `@group(2)` while omitting groups 0/1 causes wgpu validation failure (bind group indices must be contiguous starting at 0). Removed the dummy uniform + binding entirely for the no‑op version.
- Empty `layout: vec![]` with a shader that declares no bindings is valid.
- Removed speculative plan to rely on an unused binding; simpler & safer to have zero bindings.

## Reference Patterns (Bevy Examples) – Rationale Mapping
Pulling minimal, directly relevant idioms from upstream examples under `external/bevy/examples/` to justify and guide the no‑op implementation:

1. Compute Pipeline Creation (no state machine needed here)
   - Source: `shader/compute_shader_game_of_life.rs` (`GameOfLifePipeline` in `FromWorld` + queued `ComputePipelineDescriptor`).
   - Our adaptation: Inline (single) `ComputePipelineDescriptor` queued from a render‑world system (`prepare_noop_compute_pipeline`) instead of `FromWorld`, because the pipeline depends only on a lazily loaded shader handle and has zero layouts.
   - Justification: Avoid unnecessary `FromWorld` impl & multi‑state orchestration (`Loading / Init / Update`) used in Game of Life; we only need a single steady state once pipeline cache resolves.

2. Render Graph Node Insertion & Ordering
   - Source: `shader/custom_post_processing.rs` (use of `.add_render_graph_edges` to enforce ordering around `Node3d` stages) and `shader/compute_shader_game_of_life.rs` (manual `add_node` + `add_node_edge`).
   - Our adaptation: Manual `add_node` + two `add_node_edge` calls to place the compute node strictly between `Node2d::StartMainPass` and `Node2d::MainPass` within `Core2d` sub‑graph. Mirrors the explicit edge pattern in Game of Life for deterministic placement.

3. Minimal Compute Pass Encoding
   - Source: `shader/compute_shader_game_of_life.rs` `run()` method creating a compute pass via `command_encoder().begin_compute_pass(...)` and immediately dispatching workgroups.
   - Our adaptation: Same pattern but with a single workgroup (1,1,1) and no bind groups (empty layout), ensuring zero overhead and no resource mutations inside `run()`.

4. Separation of Logging / Mutation from Node Execution
   - Source: `shader/custom_post_processing.rs` and `app/headless_renderer.rs` where pipeline / resource mutation occurs in systems, not within node execution paths except for pass encoding.
   - Our adaptation: One‑time log performed by a separate system (`log_noop_once`) after pipeline readiness to avoid mutating resources inside the node (which only has immutable access pattern).

5. WASM Asset Embedding Strategy
   - Source: Overall engine pattern (mirrors existing metaballs unified shader approach) rather than a single example; consistent with `include_str!` for deterministic deployment (`GameOfLife` uses asset server path, we embed for wasm parity & offline reliability).

## Verification Alignment with Examples
- Pipeline readiness: We intentionally skip a node internal state machine (Game of Life example) because absence of bindings + trivial shader means pipeline either exists or we early‑return; eventual dispatch only after `PipelineCache` resolves (`get_compute_pipeline(pid)` returns `Some`).
- Ordering: Like post processing example ensures effect after tonemapping, we enforce pre‑main pass placement; edges guarantee topological correctness without relying on registration order.
- Mutability: Resource mutation (shader handle assignment, pipeline queueing, log flag) restricted to systems—mirrors patterns where example nodes only encode commands.

## Required Steps (Implement Exactly)
1. Create WGSL file `assets/shaders/metaballs_noop_compute.wgsl`:
   ```wgsl
   // ============================================================================
   // Metaballs Precompute No-Op Pass
   // Placeholder compute stage dispatched before metaball rendering.
   // Future extension: field reductions, SDF normal prep, cluster prefix sums.
   // ============================================================================
   @compute @workgroup_size(1)
   fn cs_main() { /* intentionally empty */ }
   ```

2. WASM embedding (in `MetaballsPlugin` inside existing WASM shader embedding block in `src/rendering/metaballs/metaballs.rs`):
   ```rust
   #[cfg(target_arch = "wasm32")]
   static METABALLS_NOOP_COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();
   // After adding unified shader handle initialization:
   let noop_handle = shaders.add(Shader::from_wgsl(
       include_str!("../../../assets/shaders/metaballs_noop_compute.wgsl"),
       "metaballs_noop_compute_embedded.wgsl",
   ));
   METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get_or_init(|| noop_handle.clone());
   ```

3. Define render‑world resource:
   ```rust
   #[derive(Resource, Default)]
   pub struct MetaballsNoopComputePipeline {
       pub pipeline_id: Option<CachedComputePipelineId>,
       pub shader: Option<Handle<Shader>>,
       pub logged: bool,
   }
   ```

4. Preparation system (added to `Render` schedule in render sub‑app):
   ```rust
   fn prepare_noop_compute_pipeline(
       mut pipelines: ResMut<MetaballsNoopComputePipeline>,
       mut pipeline_cache: ResMut<PipelineCache>,
       asset_server: Res<AssetServer>,
   ) {
       if pipelines.shader.is_none() {
           #[cfg(target_arch = "wasm32")]
           { pipelines.shader = Some(METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get().unwrap().clone()); }
           #[cfg(not(target_arch = "wasm32"))]
           { pipelines.shader = Some(asset_server.load("shaders/metaballs_noop_compute.wgsl")); }
       }
       if pipelines.pipeline_id.is_none() {
           let shader = pipelines.shader.as_ref().unwrap().clone();
           let desc = ComputePipelineDescriptor {
               label: Some("metaballs.noop.compute".into()),
               layout: vec![], // no bindings
               push_constant_ranges: vec![],
               shader,
               entry_point: Cow::from("cs_main"),
               shader_defs: vec![],
           };
           pipelines.pipeline_id = Some(pipeline_cache.queue_compute_pipeline(desc));
       }
   }
   ```

5. Render graph node:
   ```rust
   #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
   struct MetaballsNoopComputeNodeLabel;

   #[derive(Default)]
   struct MetaballsNoopComputeNode;

   impl Node for MetaballsNoopComputeNode {
       fn run(&self, _graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
           let res = world.get_resource::<MetaballsNoopComputePipeline>().ok_or(NodeRunError::MissingResource)?;
           let Some(pid) = res.pipeline_id else { return Ok(()); };
           let cache = world.resource::<PipelineCache>();
           let Some(pipeline) = cache.get_compute_pipeline(pid) else { return Ok(()); };
           if !res.logged {
               // Defer logging: we cannot mutate resource here; instead rely on a separate system OR accept single log earlier after pipeline ready.
           }
           let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("metaballs_noop_precompute") });
           pass.set_pipeline(pipeline);
           pass.dispatch_workgroups(1, 1, 1);
           Ok(())
       }
   }
   ```
   Logging strategy: Add a small `log_noop_once` render‑world system running after preparation that logs and flips `logged` boolean (mutably) to avoid mutating inside the node.

6. Graph insertion (in render sub‑app setup after initializing resource & systems):
   ```rust
   let mut rg = render_app.world_mut().resource_mut::<RenderGraph>();
   let sub = rg.get_sub_graph_mut(Core2d).expect("Core2d graph exists");
   sub.add_node(MetaballsNoopComputeNodeLabel, MetaballsNoopComputeNode::default());
   // Ensure it runs before MainPass draw:
   let _ = sub.add_node_edge(Node2d::StartMainPass, MetaballsNoopComputeNodeLabel);
   let _ = sub.add_node_edge(MetaballsNoopComputeNodeLabel, Node2d::MainPass);
   // Fallback (if StartMainPass variant changes) — if edge addition fails, edge from Prepass instead.
   ```

7. Plugin wiring additions (in `MetaballsPlugin::build` render sub‑app block):
   ```rust
   render_app
       .init_resource::<MetaballsNoopComputePipeline>()
       .add_systems(Render, prepare_noop_compute_pipeline)
       .add_systems(Render, log_noop_once.after(prepare_noop_compute_pipeline));
   // then graph insertion code.
   ```

8. One‑time log system:
   ```rust
   fn log_noop_once(mut pipe: ResMut<MetaballsNoopComputePipeline>) {
       if pipe.pipeline_id.is_some() && !pipe.logged {
           info!(target="metaballs", "No-op compute prepass active");
           pipe.logged = true;
       }
   }
   ```

9. Testing checklist:
   - Run `cargo run` (native) observe single log message before first metaballs draw.
   - Enable wgpu backend validation (RUST_LOG=wgpu=trace or WGPU_BACKEND=... if needed) — confirm no warnings about missing bind groups.
   - Optional: Add a transient debug counter (frame resource) incremented inside node then read in a test to assert dispatch occurred.
   - Cross‑check: Compare ordering with a temporary instrumentation log inside `Node2d::MainPass` adjacent systems to confirm prepass runs earlier.

10. Performance: Dispatch (1,1,1) is negligible (<0.01 ms). Verify using existing frame timing if added later; otherwise assume trivial.

## Optional Extension Patterns (Not Implemented Now)
- Per‑view execution: If later a per‑camera precompute is needed (e.g., view‑dependent field culling), refactor to a `ViewNodeRunner` pattern similar to `custom_post_processing.rs`. Requires converting the node to implement `ViewNode` and adding via `.add_render_graph_node::<ViewNodeRunner<...>>(Core2d, Label)` plus appropriate edges.
- Pipeline creation via `FromWorld`: Mirror `GameOfLifePipeline` if additional bind group layouts or multiple entry points become necessary.

## Non-Goals / Must NOT
- Do NOT modify existing material or fragment shader.
- Do NOT introduce new bind groups or reorder existing ones.
- Do NOT log every frame.
- Do NOT add unused buffer allocations.

## Success Criteria Checklist
- [ ] No wgpu validation errors (native + wasm).
- [ ] Log appears exactly once.
- [ ] Visual parity confirmed (manual or screenshot diff).
- [ ] Compute node executes before `MainPass` (confirmed via log ordering or instrumentation counter).

## Future Extension Notes
- Node ready for: field reduction, SDF normal sampling, cluster prefix sums, occlusion masks.
- If future data needed: add bind group layout at group(0) for new uniform/storage; keep groups contiguous from 0.

Implement exactly as above.
