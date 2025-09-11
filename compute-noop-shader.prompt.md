# Compute No‑Op Metaballs Prepass Prompt (Updated)

Tags: #codebase #module:rendering/metaballs/compute_noop.rs #file:assets/shaders/metaballs_noop_compute.wgsl

Goal: Add a dedicated no‑op compute shader pass that runs every frame **before** the existing metaballs unified material render (compute -> material ordering), without changing visuals or altering current material uniform / binding layouts.

Acceptance:
- Visual output identical (pixel parity with prior build).
- Compute node dispatches exactly once per frame **before** the metaballs material draw in the Core2d render graph.
- No edits to `assets/shaders/metaballs_unified.wgsl` or its bind group layout (material remains solely at group(2)).
- No new bind groups introduced for this no‑op (pipeline layout is empty `[]`).
- Works native + WASM (shader embedded on WASM).
- Zero wgpu validation warnings / errors (bind group index continuity preserved).

Ordering Intent (Critical):
compute prepass (no‑op) -> metaballs unified material (fragment) -> rest of pipeline.

Rationale: Establish stable hook point for future GPU preprocessing (field reduction, prefix sums, SDF normal prep) while guaranteeing zero behavioral change today.

## Changes Since Previous Version
- Explicit module: `src/rendering/metaballs/compute_noop.rs` (avoid expanding legacy `metaballs.rs`).
- Added concrete import list & `render_app` acquisition pattern.
- Clarified WASM embedding is local to new module (parallel to material embedding) – no modification of existing material embed function.
- Reinforced that the compute pass keeps an empty layout; future passes adding resources must start at group(0) (independent) and MUST NOT force material group index shifts.
- Added optional dispatch counter resource + test guidance.

## Reference Patterns (Bevy Examples) – Rationale Mapping
1. Compute pipeline creation: mirrors `compute_shader_game_of_life.rs` but simplified (single steady state, empty layout).
2. Render graph insertion & ordering: uses explicit node edges like `custom_post_processing.rs` ensuring placement between `Node2d::StartMainPass` and `Node2d::MainPass`.
3. Minimal pass encoding: begin compute pass, set pipeline, dispatch(1,1,1).
4. Separation of setup vs execution: systems queue pipeline & log; node only encodes commands.
5. WASM embedding: mirrors existing unified metaballs embedding style with `OnceLock` and `include_str!`.

## Required Steps (Implement Exactly)

### 1. Create WGSL file `assets/shaders/metaballs_noop_compute.wgsl`
```wgsl
// ============================================================================
// Metaballs Precompute No-Op Pass
// Dispatched before metaball rendering. Future use: field reductions, SDF normal prep.
// ============================================================================
@compute @workgroup_size(1)
fn cs_main() { /* intentionally empty */ }
```

### 2. Create module `src/rendering/metaballs/compute_noop.rs`
Responsibilities: shader embedding (wasm), pipeline resource, preparation systems, graph node & insertion, one-time log, (optional) dispatch counter.

### 3. WASM shader embedding block (inside `compute_noop.rs`)
```rust
#[cfg(target_arch = "wasm32")] use std::sync::OnceLock;
#[cfg(target_arch = "wasm32")] static METABALLS_NOOP_COMPUTE_SHADER_HANDLE: OnceLock<Handle<Shader>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
pub fn init_wasm_noop_shader(world: &mut World) {
    let mut shaders = world.resource_mut::<Assets<Shader>>();
    let handle = shaders.add(Shader::from_wgsl(
        include_str!("../../../assets/shaders/metaballs_noop_compute.wgsl"),
        "metaballs_noop_compute_embedded.wgsl",
    ));
    METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get_or_init(|| handle);
}
```
(Do not modify existing material init; call this separately from plugin build when wasm.)

### 4. Define render-world pipeline resource
```rust
#[derive(Resource, Default)]
pub struct MetaballsNoopComputePipeline {
    pub pipeline_id: Option<CachedComputePipelineId>,
    pub shader: Option<Handle<Shader>>,
    pub logged: bool,
}
```
(Optional) dispatch counter:
```rust
#[derive(Resource, Default)] pub struct MetaballsNoopDispatchCount(pub u64);
```

### 5. Preparation system (runs in Render schedule)
```rust
use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::{render_resource::*, renderer::RenderDevice, render_graph::{RenderGraph, NodeRunError, RenderGraphContext}, RenderApp};
use bevy::render::renderer::RenderContext;

fn prepare_noop_compute_pipeline(
    mut pipe: ResMut<MetaballsNoopComputePipeline>,
    mut pipeline_cache: ResMut<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    if pipe.shader.is_none() {
        #[cfg(target_arch = "wasm32")] {
            pipe.shader = Some(METABALLS_NOOP_COMPUTE_SHADER_HANDLE.get().unwrap().clone());
        }
        #[cfg(not(target_arch = "wasm32"))] {
            pipe.shader = Some(asset_server.load("shaders/metaballs_noop_compute.wgsl"));
        }
    }
    if pipe.pipeline_id.is_none() {
        let shader = pipe.shader.as_ref().unwrap().clone();
        let desc = ComputePipelineDescriptor {
            label: Some("metaballs.noop.compute".into()),
            layout: vec![],              // empty layout => no bindings
            push_constant_ranges: vec![],
            shader,
            entry_point: Cow::from("cs_main"),
            shader_defs: vec![],
        };
        pipe.pipeline_id = Some(pipeline_cache.queue_compute_pipeline(desc));
    }
}
```

### 6. Render graph node definition
```rust
use bevy::render::render_graph::{Node, NodeLabel};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct MetaballsNoopComputeNodeLabel;

#[derive(Default)]
pub struct MetaballsNoopComputeNode;

impl Node for MetaballsNoopComputeNode {
    fn run(&self, _graph: &mut RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
        let res = world.get_resource::<MetaballsNoopComputePipeline>().ok_or(NodeRunError::MissingResource)?;
        let Some(pid) = res.pipeline_id else { return Ok(()); };
        let cache = world.resource::<PipelineCache>();
        let Some(pipeline) = cache.get_compute_pipeline(pid) else { return Ok(()); };
        let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor { label: Some("metaballs_noop_precompute") });
        pass.set_pipeline(pipeline);
        pass.dispatch_workgroups(1, 1, 1);
        if let Some(mut counter) = world.get_resource_mut::<MetaballsNoopDispatchCount>() { counter.0 += 1; }
        Ok(())
    }
}
```

### 7. One‑time log system
```rust
fn log_noop_once(mut pipe: ResMut<MetaballsNoopComputePipeline>) {
    if pipe.pipeline_id.is_some() && !pipe.logged {
        info!(target="metaballs", "No-op compute prepass active (compute -> material ordering)");
        pipe.logged = true;
    }
}
```

### 8. Plugin wiring (edit `MetaballsPlugin::build` in `metaballs.rs`)
Add near start:
```rust
#[cfg(target_arch = "wasm32")] { super::compute_noop::init_wasm_noop_shader(app.world_mut()); }
```
After existing material plugin registration & before end:
```rust
// Access render sub-app
let render_app = app.sub_app_mut(RenderApp);
render_app
    .init_resource::<super::compute_noop::MetaballsNoopComputePipeline>()
    .init_resource::<super::compute_noop::MetaballsNoopDispatchCount>()
    .add_systems(Render, super::compute_noop::prepare_noop_compute_pipeline)
    .add_systems(Render, super::compute_noop::log_noop_once.after(super::compute_noop::prepare_noop_compute_pipeline));

// Insert node into Core2d graph BEFORE MainPass
use bevy::core_pipeline::core_2d::{Core2d, graph::Node2d};
use super::compute_noop::{MetaballsNoopComputeNodeLabel, MetaballsNoopComputeNode};
let mut rg = render_app.world_mut().resource_mut::<RenderGraph>();
let sub = rg.get_sub_graph_mut(Core2d).expect("Core2d graph exists");
sub.add_node(MetaballsNoopComputeNodeLabel, MetaballsNoopComputeNode::default());
let _ = sub.add_node_edge(Node2d::StartMainPass, MetaballsNoopComputeNodeLabel);
let _ = sub.add_node_edge(MetaballsNoopComputeNodeLabel, Node2d::MainPass);
```
Add `pub mod compute_noop;` in `rendering/metaballs/mod.rs` and ensure re-export if needed (optional, not required for internal use).

### 9. Imports Summary (avoid missing items)
```rust
use bevy::prelude::*;
use bevy::render::{RenderApp, render_graph::{RenderGraph, Node, NodeRunError, RenderGraphContext, RenderLabel}, renderer::RenderContext, render_resource::*};
use std::borrow::Cow;
```

### 10. Testing Checklist
- Native run: `cargo run` -> single log line: "No-op compute prepass active" (only once).
- Visual parity: capture screenshots before/after; diff (expect identical).
- Render graph: ensure node appears between StartMainPass and MainPass (can temporarily add debug log in node run).
- Dispatch count resource increments (>0 after 1 frame). Optional automated test spins minimal `App` with headless renderer + one frame.
- WASM build: `wasm-bindgen` output loads without wgpu validation errors (check browser console).
- wgpu validation: run with `RUST_LOG=wgpu=warn` confirm no bind group continuity errors.

### 11. Performance Considerations
- Dispatch (1,1,1) cost negligible; early-returns until pipeline ready avoid unnecessary pass creation.
- No allocations per frame; counter increment optional and O(1).

### 12. Future Extension Guidance
- When adding actual compute work: introduce new bind group layouts starting at group(0). This does NOT require changing the material's current use of group(2); they are independent pipelines.
- If sharing data with material, write into storage buffers (group(0) for compute) then upload same buffer to material *only if* material shader is updated—otherwise keep compute outputs internal until a coordinated material update.
- Keep node positioning stable; additional prepasses should chain before this node if they feed it, or after if they consume its outputs, using explicit edges.

### 13. Non-Goals / Must NOT
- No modifications to `metaballs_unified.wgsl`.
- No new material bindings or reordering.
- No per-frame logging.
- No placeholder unused buffers just to satisfy layout continuity.

### 14. Success Criteria Checklist
- [ ] No wgpu validation errors (native + WASM).
- [ ] Log appears exactly once.
- [ ] Visual parity confirmed (no pixel differences within tolerance 0).
- [ ] Compute node recorded BEFORE material draw (inspect GPU capture or graph debug).
- [ ] Dispatch counter > 0 after first frame (if enabled).

### 15. Optional Test Sketch
```rust
#[test]
fn noop_compute_dispatches_once() {
    use bevy::prelude::*; use bevy::render::RenderPlugin;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::log::LogPlugin::default(), RenderPlugin, crate::rendering::metaballs::MetaballsPlugin));
    // Advance a few frames
    for _ in 0..3 { app.update(); }
    let count = app.world().get_resource::<crate::rendering::metaballs::compute_noop::MetaballsNoopDispatchCount>().unwrap().0;
    assert!(count > 0);
}
```
(Adjust imports / feature flags as needed for headless test harness.)

### 16. Risk Mitigations
- Pipeline unresolved frames: node safely no-ops.
- WASM embedding path stable relative to module file.
- Future additions documented to prevent accidental layout breakage.

Implement exactly as above.
