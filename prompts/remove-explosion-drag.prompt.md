# Remove Explosion & Drag Interactions (Consolidate to Cluster Pop Only) [ARCHIVED]

<!-- ARCHIVED NOTICE: This prompt has been fully executed. Explosion & Drag interactions were removed and replaced by a single Cluster Pop interaction with absolute impulse (cluster_pop.impulse). Retained only for historical context. Do NOT reintroduce removed concepts. -->

## Objective
You WILL permanently remove all Explosion and Drag interaction functionality (config, systems, resources, input bindings, overlays, prompts) so that Cluster Pop is the ONLY tap-based interaction. You WILL refactor any existing dependency on explosion impulse into the cluster pop configuration. You MUST leave the codebase compiling, behaviorally stable, and free of dead references.

## High-Level Outcomes
- Explosion & Drag configs, systems, resources, input actions, validation, overlays, and prompts are eradicated.
- Cluster pop continues to function (tap to pop qualifying clusters).
- No panics, no unused imports, no feature regression in unrelated systems.
- Build passes: native + (if applicable) wasm.
- Configuration file(s) (`game.ron`, `input.toml`) simplified to remove obsolete keys.
- Documentation and prompts no longer reference Explosion or Drag.

## Scope (Inclusions)
- Config structs: `ExplosionConfig`, `DragConfig`, their fields inside `InteractionsConfig` (or equivalent).
- Systems: `handle_tap_explosion`, all drag handling systems, `ActiveDrag` resource, `TapExplosionSet`.
- Input bindings: any Explosion or Drag binding in `assets/config/input.toml`.
- Validation logic referencing explosion or drag.
- Overlay / debug UI output referencing explosion or drag.
- Prompts referencing generic explosion fallback semantics.
- Cluster pop logic referencing `cfg.interactions.explosion.*` must be refactored.
- Remove imports pulling now‑deleted types.

## Explicit Exclusions (Non-Goals)
- Do NOT change unrelated physics, spawning, metaballs, clustering logic (except where explosion impulse was used as a scalar).
- Do NOT refactor cluster pop fade / timing behavior (unless strictly required by removed dependencies).
- Do NOT introduce new features beyond what is necessary to preserve cluster pop impulses.

## Replacement / Refactor Plan
1. Introduce a new absolute impulse field inside cluster pop config if current implementation multiplies explosion baseline:
   - Option A (preferred): Replace `impulse_scale` with `impulse` (absolute magnitude).
   - Option B: Keep `impulse_scale` but internally treat it as absolute (rename variable usages accordingly). (Pick Option A unless a wide surface area makes renaming high risk.)
2. Where code does:
   ```rust
   let base_impulse = cfg.interactions.explosion.impulse;
   magnitude = base_impulse * cluster_pop.impulse_scale * ...
   ```
   Replace with:
   ```rust
   let base_impulse = cluster_pop.impulse;
   magnitude = base_impulse * (ball_radius.0 / 10.0).max(0.1) * (1.0 + cluster_pop.outward_bonus);
   ```
   (Adjust math only to remove explosion dependency; preserve existing outward scaling semantics.)
3. Remove gating references to `TapExplosionSet`; reorder systems if now redundant.

## Detailed Step-by-Step Instructions

### 1. Configuration Layer
You WILL:
- Delete `ExplosionConfig` and `DragConfig` structs + `Default` impls.
- Remove their fields from the parent interactions config struct.
- Remove their validation branches.
- In `assets/config/game.ron`:
  - Delete lines:
    ```ron
    explosion: (...),
    drag: (...),
    ```
  - Inside `cluster_pop`, add:
    ```ron
    impulse: 500.0,
    ```
    (Select a default mirroring old `explosion.impulse` if previously relied upon.)
  - Remove now meaningless commas or trailing separators.
- Update `serde(default)` logic and `Default` for parent config to no longer reference removed fields.
- If `impulse_scale` existed, either:
  - Replace with `impulse` key in both struct + file.
  - Provide backward-compatible optional deserialization (ONLY if legacy file compatibility required; otherwise skip).

### 2. Input Mapping
You WILL:
- Remove Explosion entries from `assets/config/input.toml`:
  - Action definition block `Explosion = { ... }`
  - Any binding lines referencing `Explosion`.
- Remove any drag-related action definitions if they exist.

### 3. Interaction Systems
You WILL:
- In `interaction::input::input_interaction.rs`:
  - Remove `use` imports referencing `ExplosionConfig`, `DragConfig`, and `ActiveDrag`.
  - Delete the `ActiveDrag` resource definition + insertion.
  - Remove systems handling drag (start/continue/end) and explosion (`handle_tap_explosion`).
  - Remove `TapExplosionSet` label or system set definition.
  - Clean up schedule registration lines adding those systems.
- In `interaction::cluster_pop::mod.rs`:
  - Remove `use crate::interaction::input::input_interaction::{ActiveDrag, TapExplosionSet};`
  - Remove ordering constraints referencing `TapExplosionSet`.
  - Delete logic checking `active_drag` unless cluster pop gating needs it (if cluster pop previously skipped while drag active, decide: either keep a minimal bool in cluster pop module OR drop the condition—prefer removal unless design requires drag suppression).
  - Replace any reference to `cfg.interactions.explosion.impulse` with `cluster_pop.impulse` (or derived field).

### 4. Overlay / Debug
You WILL:
- In any overlay (e.g., `debug::overlay.rs`) delete sections rendering explosion / drag config fields.
- Remove imports referencing explosion or drag config types.

### 5. Prompts Cleanup
You WILL:
- Open existing cluster pop related prompts (`cluster-pop-on-tap.prompt.md`, `cluster-pop-explosive-fade.prompt.md`) and:
  - Remove narrative about “fallback to generic explosion.”
  - Remove references to `TapExplosionSet` or gating consumption for explosion fallback.
  - Replace any formulae referencing `explosion.impulse` with `cluster_pop.impulse`.
  - If fade prompt mentions explosion coexistence, rewrite to state cluster pop is now the sole interaction.

### 6. Documentation & Instructions
You WILL:
- In `.github/copilot-instructions.md` adjust interaction list: remove “drag, explosion” mention; restate as: “interaction – input mapping, cluster pop, parameter tweak.”
- Search for textual references “explosion” and “drag” outside code (prompts, docs); prune or rephrase as historical notes only if needed.
- Update any story or design docs citing “generic explosion” to reflect removal.

### 7. Code Hygiene
You WILL:
- Remove orphan `use` lines and fix warnings.
- Ensure no feature flags still gate removed code.
- Ensure compilation with `cargo build --all-features` (if features exist) and `cargo clippy --all-targets --all-features -D warnings` passes.

### 8. Refactor Safety / Migration
You WILL:
- Provide (in commit message or PR description) a Migration Note:
  ```
  BREAKING: Removed Explosion && Drag interactions. Added cluster_pop.impulse (absolute) replacing explosion baseline. Update config files accordingly.
  ```
- If backward compatibility is required (ONLY if explicitly mandated): add a temporary `#[serde(alias = "impulse_scale")]` or custom `deserialize_with` for cluster pop impulse; otherwise skip.

### 9. Testing / Validation
You WILL validate by:
- Running simulation: confirm tap on qualifying cluster still pops (with outward impulses).
- Confirm tapping empty space no longer triggers area impulses.
- Confirm no runtime warnings referencing missing config fields.
- Confirm no panics on deserialization of updated `game.ron`.
- (Optional) Add/Update a unit test verifying `GameConfig::default()` no longer includes explosion / drag fields and cluster_pop.impulse is > 0.

### 10. Performance / Cleanup
You WILL:
- Remove any per-frame branches that previously early-returned when explosion/drag disabled.
- Confirm cluster pop per-frame overhead unchanged or reduced.
- Ensure no leftover allocations related to drag state.

## Edge Cases to Consider
- Old `game.ron` still containing removed keys → Decide: either fail with warning (“ignored legacy explosion/drag config”) or remove silently. Prefer logging a single warning once: “Ignoring legacy interactions.explosion / interactions.drag (removed).”
- Cluster pop impulse zero or negative: Add existing validation check (≥ 0, warn if 0 disables effect).
- Accidental leftover `TapConsumed` gating logic—should be pruned if only used for explosion suppression.

## Validation Checklist (You MUST Satisfy All)
- [ ] Build succeeds (debug & release).
- [ ] No references to `ExplosionConfig`, `DragConfig`, `ActiveDrag`, `TapExplosionSet`.
- [ ] No `explosion:` or `drag:` blocks in any config.
- [ ] Overlay renders without sections for removed interactions.
- [ ] Cluster pop impulses still applied (verify visually or via log added temporarily).
- [ ] No clippy warnings from removals.
- [ ] Prompts updated; no instruction referencing fallback explosion.
- [ ] Instructions / docs updated (if repository policy requires alignment).

## Suggested Commit Granularity
1. Remove configs + adjust `game.ron`.
2. Remove systems/resources + refactor cluster pop.
3. Clean prompts/docs/overlays.
4. Final lint & validation adjustments.

## Risks & Mitigations
| Risk | Mitigation |
|------|------------|
| Orphaned imports cause warnings treated as errors | Run clippy early after removal step |
| Cluster pop still referencing removed field | Grep for `explosion.` after edits |
| Accidental retention of system ordering sets | Search for `TapExplosionSet` symbol |
| Legacy config breaks load | Add single warning for unknown keys; ensure serde ignores extra fields (default) |

## Commands (For Executor)
```bash
cargo build
cargo clippy --all-targets --all-features -D warnings
cargo run --release
rg "explosion" -g "!assets/config/game.ron"
rg "DragConfig"
```

## Success Definition
Task is complete when all checklist items pass, cluster pop remains functional with new impulse logic, and the codebase has zero explosion/drag references outside historical notes (if any remain intentionally).

---

<!-- End of removal prompt -->
