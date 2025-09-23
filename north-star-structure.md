<a name="top"></a>

# Metaball Clusters Game – North Star Design Spec

Concise, living blueprint for a 2D metaball‑visualized, cluster‑centric physics game. Emphasizes strict separation of (a) simulation, (b) GPU rendering, (c) orchestration, enabling parallel evolution and targeted testing.

---

## 1. Vision
Create expressive, squishy “blob” objects composed of multiple particles (balls) whose motion is governed by cluster physics while a GPU metaball field provides a cohesive visual skin. Rendering is cosmetic; simulation never depends on sampled field values.

## 2. Scope (Initial Release)
* 2D world: X,Y ∈ [-256, 256]
* Fixed orthographic camera framing that matches physics coordinates
* Clusters composed of ≥1 balls (soft or pseudo‑rigid via constraints)
* Ball–ball collisions only across different clusters / environment
* GPU metaball field (compute + fragment pass) driven by per‑frame (x,y,radius)
* Deterministic(ish) update loop with fixed physics timestep
* Minimal input (optional) to nudge or drive a single cluster

## 3. Goals
1. Modularity: Independent crates with minimal public surface.
2. Testability: Each crate demoable & unit testable in isolation.
3. Performance: Keep CPU free of field math; push blending to GPU.
4. Maintainability: Clear, stable terminology (Ball / Cluster / Metaball).
5. Extensibility: Room for later features (splitting, AI) without redesign.

## 4. Non‑Goals (Now)
* Metaball field as collision / gameplay surface
* Network / multiplayer / rollback
* Runtime cluster splitting/merging logic
* ECS / large engine abstraction layer
* 3D rendering / perspective camera

## 5. Core Terminology
| Term      | Meaning |
|-----------|---------|
| Ball (Particle) | Small physics circle with position, velocity, radius, (optional mass) |
| Cluster   | Logical grouping of Balls evolving together (with constraints) |
| Metaball  | Visual influence region corresponding to a Ball for GPU blending |
| PhysicsWorld | Container managing clusters, integration, collisions |

## 6. Coordinate System
* World units == render units (no conversion layer)
* Orthographic projection covers the full square [-256,256]^2
* Positions leaving this region are either clamped, wrapped, or allowed (policy TBD; start with soft boundary / simple wall collisions)

## 7. Workspace & Crate Architecture
```
game_app (bin)
 ├─ depends on: metaball_renderer, cluster_physics
 ├─ owns: main loop / input / orchestration
 │
 ├─ metaball_renderer (lib)
 │    ├─ GPU pipelines, shaders, uniform/SSBO packing
 │    └─ Public: MetaballRenderer { update_metaballs, render_frame }
 │
 └─ cluster_physics (lib)
      ├─ Ball, Cluster, PhysicsWorld
      └─ Public: step/update, add_cluster, enumerate balls

demo crates:
  metaball_demo  → uses only metaball_renderer
  physics_demo   → uses only cluster_physics (headless or minimal draw)
```

## 8. Crate Specifications
### 8.1 `metaball_renderer`
Responsibilities:
* Maintain GPU buffers of ball sources (x,y,r)
* Issue compute (optional) + render pass to produce smooth field
* Fixed camera setup for defined world bounds

Design Notes:
* Pure consumer of positions; no simulation authority
* Avoid CPU-side field sampling; push thresholds & normals into shader
* API returns Result where fallible (resource creation)

Public Sketch:
```rust
pub struct MetaballRenderer { /* internal state */ }
impl MetaballRenderer {
    pub fn new(window: &Window) -> anyhow::Result<Self> { /* ... */ }
    pub fn update_metaballs(&mut self, balls: &[(f32,f32,f32)]) { /* x,y,r */ }
    pub fn render_frame(&mut self) { /* encode + submit */ }
}
```

Testing:
* Demo animates synthetic balls (orbit / sine wave)
* Assert pipeline creation & buffer resize logic in unit tests

### 8.2 `cluster_physics`
Responsibilities:
* Own Ball & Cluster data
* Integrate motion (semi-implicit Euler or velocity Verlet)
* Enforce intra‑cluster constraints (distance / spring)
* Resolve inter‑cluster collisions (circle vs circle)

Core Types:
```rust
pub struct Ball { pub pos: Vec2, pub vel: Vec2, pub radius: f32, pub mass: f32 }
pub struct Cluster { pub balls: Vec<usize>, /* indices into balls store */ }
pub struct PhysicsWorld { /* balls, clusters, params */ }
```
APIs:
```rust
impl PhysicsWorld {
    pub fn new() -> Self;
    pub fn add_cluster(&mut self, cluster: Cluster);
    pub fn step(&mut self, dt: f32); // integrate + constraints + collisions
    pub fn balls_export(&self, out: &mut Vec<(f32,f32,f32)>);
}
```
Design Choices:
* Ignore collisions within same cluster
* Constraints: pairwise springs or precomputed rest-length graph
* Optional damping factor to stabilize soft oscillations

Testing:
* Unit: two-ball elastic collision momentum sanity
* Unit: constraint length preservation tolerance
* Headless soak: many steps without NaNs / divergence

### 8.3 `game_app`
Responsibilities:
* Window + event loop
* Fixed timestep accumulator for physics
* Bridges physics → rendering each frame
* Input mapping to cluster impulses (if enabled)

Loop Sketch:
```rust
const DT: f32 = 1.0 / 60.0;
let mut acc = 0.0;
while running {
    let frame_time = timer.delta();
    acc += frame_time;
    process_input(&mut world);
    while acc >= DT { world.step(DT); acc -= DT; }
    export.clear(); world.balls_export(&mut export);
    renderer.update_metaballs(&export);
    renderer.render_frame();
}
```

### 8.4 Demo Crates
* `metaball_demo`: visual stress & shader tuning
* `physics_demo`: textual / minimal graphical verification
* Keep dependencies minimal to preserve isolation

## 9. Data Flow (One Frame)
1. Input events mutate desired forces / cluster control
2. Physics steps in fixed increments (0..N times) updating Ball states
3. Export pass collects (x,y,r) into staging buffer
4. Renderer uploads / maps buffer and draws field
5. Present

## 10. Physics Design Details
* Integration: start with semi‑implicit Euler (stable & simple)
* Collision: circle overlap -> positional correction + impulse response
* Constraint Solve Order: (a) integrate velocities, (b) apply constraints (Gauss–Seidel few iterations), (c) collision resolution
* Stability Guards: clamp max velocity, early NaN check
* Time Determinism: fixed dt; rendering interpolates implicitly by using latest state only (no interpolation layer initially)

## 11. Rendering Design Details
* Uniform / storage buffer of balls; max count chosen conservatively (profile later)
* Field evaluation in fragment shader or optional compute pre-pass to texture
* Normal derivation via screen‑space gradient or analytic partials
* Threshold (iso) & color ramp adjustable constants
* Avoid CPU copies: single mapped write per frame

## 12. Public API Contract (Summary)
Guiding principles:
* Fallible construction returns Result
* Per-frame methods infallible & fast
* Export uses caller-provided Vec to avoid allocations

## 13. Testing Strategy
| Layer | Method | Focus |
|-------|--------|-------|
| Physics unit | dt, collision, constraints | Correctness / invariants |
| Physics soak | long run | Stability / no drift |
| Renderer unit | pipeline init, buffer growth | Resource correctness |
| Renderer demo | visual | Shading / blending |
| Integration | small scripted loop | Data handoff sanity |

Automation: integrate quick headless physics tests in CI, renderer smoke tests (build + create pipeline) without requiring GPU output validation.

## 14. Example Use Cases (Condensed)
1. Pure metaball visualization: animate 3 synthetic points → verify smooth merge/separation.
2. Cluster drop: square of 4 balls falls & bounces → constraint lengths stable.
3. Player control: apply directional impulse via input → cluster translates & squishes on collisions.

## 15. Naming & Style
* Crates: `metaball_renderer`, `cluster_physics`, `game_app`
* Types: `Ball`, `Cluster`, `PhysicsWorld`, `MetaballRenderer`
* Methods: snake_case; no `get_` unless ambiguous
* Constants: `MAX_BALLS`, `DEFAULT_RADIUS`
* Comments: rustdoc for public, inline rationale for non-obvious constraints

## 16. Future Extensions (Not Designed Yet)
* Cluster splitting (constraint break thresholds)
* GPU compute pass for distance field atlas / SDF caching
* Editor / interactive parameter panel
* Save/load state snapshots

## 17. Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Constraint instability | Limit iterations; damping; velocity caps |
| GPU buffer overrun | Enforce MAX_BALLS; assert + graceful skip |
| Performance regression | Bench demos; add frame time logging flag |
| Architectural coupling creep | Keep renderer API flat (positions only) |

## 18. Definition of Done (Initial Milestone)
* Physics demo: two clusters collide stably for 60s
* Metaball demo: 100 balls @ 60 FPS (release) on target machine
* Integrated: visible cluster responding to input, collisions mirrored visually
* No panics / leaks in debug & release runs

## 19. Contribution Guidelines (Doc Alignment)
* New field? Update this spec + crate README + rustdoc
* Keep public APIs minimal; prefer internal modules over premature exposure
* Add tests alongside new simulation logic

## 20. Reference Pseudocode (End-to-End)
```rust
fn frame(dt_acc: &mut f32, world: &mut PhysicsWorld, renderer: &mut MetaballRenderer, balls_tmp: &mut Vec<(f32,f32,f32)>, frame_dt: f32) {
    *dt_acc += frame_dt;
    while *dt_acc >= DT { world.step(DT); *dt_acc -= DT; }
    balls_tmp.clear();
    world.balls_export(balls_tmp);
    renderer.update_metaballs(balls_tmp);
    renderer.render_frame();
}
```

---
End of spec. Keep concise; expand sections only when actual complexity grows.

[Back to Top](#top)
