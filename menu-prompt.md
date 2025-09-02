## **Prompt Builder**: Implement Menu + Level Lifecycle (concise)

Goal
- Add a minimal Main Menu that lists levels and lets the player select one.
- Implement robust level load/unload so levels can be switched without leaking entities or corrupting global config.

Scope (MUST be minimal and focused)
- Keyboard-driven level selection (1..N) is sufficient. No full UI required.
- Preserve existing data files in `assets/levels` (use `levels.ron`).
- Make minimal code changes; prefer small, well-scoped edits.

Acceptance criteria (MUST pass)
1. On launch the app is in `MainMenu`.
2. Pressing a numeric key (e.g. `1`) selects the corresponding level from `levels.ron`.
3. The app transitions to a `Loading` state, loads the chosen level, then transitions to `Gameplay`.
4. Level entities are tagged and can be fully despawned when switching levels.
5. After load, `Res<LevelWalls>` and `Res<LevelWidgets>` exist and gameplay systems operate.

Minimal Implementation Plan (imperative)
1) Add states
- You WILL add `src/app/state.rs` defining `AppState` (MainMenu, Loading, Gameplay, Exiting) and `GameplayState` (Playing, Paused, Intermission).

2) Register states early
- You WILL call `app.add_state::<AppState>();` and `app.add_state::<GameplayState>();` in `src/main.rs` before adding plugins that depend on states.

3) Expose registry
- You WILL `Startup`-load `assets/levels/levels.ron` and insert `LevelRegistry` as a resource so the menu can enumerate levels.

4) Menu (keyboard first)
- You WILL add `src/app/menu.rs` with a `MenuPlugin` that:
  - OnEnter(MainMenu): spawns a minimal marker entity (optional) and logs instructions.
  - Update (while in MainMenu): checks `Input<KeyCode>` and maps numeric keys to `registry.list` entries.
  - On selection: atomically `commands.insert_resource(PendingLevel { id })` then set `NextState(AppState::Loading)`.

5) Loader lifecycle
- You WILL update `src/core/level/loader.rs` to:
  - Add `#[derive(Component)] pub struct LevelEntity;` and `pub struct PendingLevel { pub id: String }`.
  - Add `OnEnter(AppState::Loading)` systems:
    - `cleanup_level`: despawn all `LevelEntity` entities.
    - `process_loading`: read `PendingLevel`, parse layout/widgets (and universal walls), spawn colliders and any level entities with the `LevelEntity` tag, insert `LevelWalls`, `LevelWidgets`, and `LevelSelection` resources, then set `NextState(AppState::Gameplay)`. On error, log and set `NextState(AppState::MainMenu)`.
  - Keep parsing/validation logic as-is, but move it behind the new `process_loading` path instead of `Startup`.

6) Tagging & cleanup
- You WILL ensure every spawned level entity includes `LevelEntity` to allow a single bulk cleanup pass using `despawn_recursive()`.

7) Avoid race conditions
- You MUST insert `PendingLevel` before setting `NextState(AppState::Loading)` (do both in the same input-handling system) so `OnEnter(Loading)` sees the pending id.

8) Gate gameplay systems (minimal)
- You WILL ensure systems that require `LevelWalls`/`LevelWidgets` either run only in `AppState::Gameplay` or check `resource_exists::<LevelWidgets>()` before operating.

Testing (manual)
- Run in PowerShell from repo root:

```powershell
cargo run --manifest-path .\Cargo.toml
```

- Test steps:
  1. Verify the app logs/instructions show MainMenu.
  2. Press `1` to select the first level in `assets/levels/levels.ron`.
  3. Watch logs: Loading -> LevelLoader messages -> Gameplay.
  4. Verify `LevelWalls` and `LevelWidgets` resources exist (via debug logs or temporary debug prints).
  5. Trigger another selection: verify previous `LevelEntity` entities are despawned (no orphaned colliders), new level spawns.

Prompt Tester instruction (concise)
- Prompt Tester, simulate the above scenario and report:
  - Files to change (list only).
  - The minimal code or small snippets needed for each change (no full refactors).
  - Any ambiguous points that block implementation.

Deliverable
- A focused developer prompt saved at `menu-prompt.md` (this file). Implement the minimal changes above; ask for permission to apply edits or run the simulated validation.


<!-- <example>
Keyboard selection behavior (1-based):
1. Registry has `list: [ { id: "test_layout", ... } ]`.
2. User presses `Key1`.
3. Menu system: insert `PendingLevel { id: "test_layout" }`; set `NextState(AppState::Loading)`.
4. OnEnter(Loading): cleanup then process loading; spawn `LevelEntity`-tagged entities; set `NextState(AppState::Gameplay)`.
</example> -->
