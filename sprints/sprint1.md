# Sprint 1: Core Architecture & Workspace Setup

## Sprint Goal
Establish the foundational crate architecture and migrate existing code into a modular workspace structure. Create the scaffolding for all major subsystems while preserving existing metaball functionality.

## Deliverables

### 1. Workspace Restructure
- [ ] Update `Cargo.toml` workspace members to include new crate structure
- [ ] Create `game_core` crate with shared components/resources/events
- [ ] Migrate existing metaball renderer to proper crate location
- [ ] Set up dependency relationships between crates

### 2. Core Crate (`game_core`)
- [ ] Define base components: `Ball`, `Wall`, `Target`, `Hazard`
- [ ] Create shared resources: `GameState`, `ArenaConfig`
- [ ] Define event system: `BallSpawned`, `TargetDestroyed`, `GameWon`, `GameLost`
- [ ] Establish component bundles for common entity patterns

### 3. Build Infrastructure
- [ ] Configure workspace-level dependencies
- [ ] Set up shared feature flags
- [ ] Create build scripts for native and WASM targets
- [ ] Verify all crates compile independently

### 4. Demo: Architecture Validation
- [ ] Create `demos/architecture_test` that uses multiple crates
- [ ] Verify metaball rendering still works after migration
- [ ] Demonstrate component/resource sharing between crates
- [ ] Show event propagation across system boundaries

## Technical Specifications

### Crate Dependencies Graph
```
game_core (no game dependencies)
    ↑
    ├── game_physics
    ├── game_rendering ← metaball_renderer
    ├── game_gameplay
    └── game (integration)
```

### Component Definitions
```rust
// game_core/src/components.rs
#[derive(Component, Clone, Copy)]
pub struct Ball {
    pub velocity: Vec2,
    pub radius: f32,
    pub color: GameColor,
}

#[derive(Component)]
pub struct Wall {
    pub segments: Vec<LineSegment>,
}

#[derive(Component)]
pub struct Target {
    pub health: u8,
    pub color: Option<GameColor>,
}
```

### File Structure
```
crates/
├── game_core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── components.rs
│       ├── resources.rs
│       ├── events.rs
│       └── bundles.rs
├── metaball_renderer/ (existing, minimal changes)
└── game/
    ├── Cargo.toml
    └── src/
        └── lib.rs (plugin integration)
```

## Migration Strategy

### Phase 1: Create Structure (Day 1-2)
1. Create new crate folders
2. Set up Cargo.toml files
3. Create module structure
4. Add placeholder implementations

### Phase 2: Extract Shared Code (Day 3-4)
1. Identify reusable components from existing demos
2. Move to `game_core` with proper abstraction
3. Update existing demos to use new crate
4. Fix compilation issues

### Phase 3: Validate Architecture (Day 5)
1. Create architecture test demo
2. Verify all crates compile
3. Test WASM build
4. Document module boundaries

## Success Criteria

- ✅ All crates compile independently
- ✅ Existing metaball demos still function
- ✅ New architecture demo runs successfully
- ✅ Clear separation of concerns achieved
- ✅ No circular dependencies
- ✅ WASM build works

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing demos | High | Keep changes minimal, test after each step |
| Circular dependencies | Medium | Use dependency graph, enforce with CI |
| Over-engineering | Medium | Start with minimal viable structure |
| WASM compatibility | High | Test web build early and often |

## Dependencies

### External Crates
- `bevy = "0.15"` (workspace-level)
- `serde = { version = "1.0", features = ["derive"] }`

### Existing Code
- Current `metaball_renderer` crate
- Demo test cases for validation

## Definition of Done

- [ ] All crates in workspace compile
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` shows no warnings
- [ ] Architecture demo showcases modular structure
- [ ] README.md created for each crate
- [ ] Workspace dependency graph documented
- [ ] Build instructions updated for new structure

## Notes for Next Sprint

Sprint 2 will build upon this foundation by:
- Adding physics systems to `game_physics` crate
- Implementing ball movement and collisions
- Creating physics playground demo
- Integrating with existing metaball renderer

The architecture established here will determine the ease of future development, so taking time to get the structure right is critical.
