# Sub‑Sprint: Demo Launcher Tool

## Goal

Provide a single unified launcher (binary + optional cargo alias) to list and run any demo (`compositor_test`, `physics_playground`, `metaballs_test`, `architecture_test`, etc.) with an interactive picker or CLI flags. Simplify iteration & QA.

## Current State

* Multiple standalone demo crates under `demos/*`.
* Developers must remember each crate name and type full `cargo run -p <demo>`.
* Repeated rebuilds; no consistent entrypoint for showcasing features.

## Objectives

1. New crate `demos/demo_launcher` producing `demo_launcher` binary.
2. Refactor each existing demo so its startup logic lives in a `run_<demo>()` function (or common trait) callable from launcher.
3. CLI flags:
   * `--list` – list demos.
   * `--demo <name>` – run a specific demo.
   * `--release` passthrough (optionally rely on cargo invocation instead) – maybe omit first iteration.
   * `--help` – usage text.
4. Interactive TTY mode (if no `--demo`): present numbered menu, accept numeric selection.
5. Graceful error for unknown demo (non‑zero exit code).

## Out of Scope

* GUI selection menu (future improvement).
* Dynamic discovery via filesystem scanning (initially hard‑coded registry is fine).
* Headless benchmarking mode (follow‑up).

## Architecture & Design

Refactor each demo crate:

```rust
// in demos/compositor_test/src/lib.rs
pub fn run_compositor_test() { bevy_app().run(); }

// in demos/compositor_test/src/main.rs
fn main() { run_compositor_test(); }
```

Launcher crate maintains registry:

```rust
struct Demo { name: &'static str, run: fn(); description: &'static str }
static DEMOS: &[Demo] = &[
   Demo { name: "compositor_test", run: run_compositor_test, description: "Layer compositing reference" },
   Demo { name: "physics_playground", run: run_physics_playground, description: "Physics sandbox with compositor" },
   // ...
];
```

CLI parsing can be hand‑rolled (simple) or use `clap` if already a workspace dependency (add only if beneficial).

Interactive mode:

* Print numbered list; read line; map to DEMOS index.
* Non‑TTY (e.g., CI) -> print usage and exit 1 if no explicit `--demo`.

## Tasks

1. Create `demos/demo_launcher/Cargo.toml` (depends on all demo crates by name).
2. For each demo crate, extract logic into `lib.rs` with `pub fn run_<demo>()` and keep minimal `main.rs` calling it.
3. Implement DEMOS registry in launcher.
4. Implement CLI arg parsing (basic): iterate args; match flags.
5. Implement `print_list()` showing name + description.
6. Implement interactive picker (only if `atty::is(Stream::Stdin)` true).
7. Error handling: unknown demo -> print list + exit code 2.
8. Add optional cargo alias: in root `Cargo.toml` `[alias] demos = "run -p demo_launcher --"` (if using aliases).
9. Update root / docs (`north-star-structure.md` or new README section) describing usage examples.
10. Smoke test script (optional): run `--list` in CI to ensure binary builds.

## Acceptance Criteria

* `cargo run -p demo_launcher -- --list` prints all demos with names and descriptions.
* `cargo run -p demo_launcher -- --demo compositor_test` launches same content as original binary.
* Running with no args in interactive terminal prompts selection; selecting valid number launches that demo.
* Unknown demo returns non‑zero exit code and prints usage.
* Refactors do not break direct `cargo run -p <demo>` usage.

## Edge Cases

* No demos registered (should not happen) – list prints "No demos available" and exits 1.
* Duplicate demo names (compile‑time duplication detection by manual review; optionally debug assert uniqueness at startup).
* Non‑TTY environment with no args – prints usage & exits 1.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Refactor churn breaks a demo | Keep extraction minimal; run each after change. |
| Added dependency graph increases compile times | Only small wrapper crate; no heavy libs unless justified. |
| Name drift (registry vs crate) | Use constants exported from each demo crate (e.g., `pub const DEMO_NAME`). |

## Definition of Done

All tasks complete; launcher runs each demo equivalently to standalone binaries; documentation updated; CI (if present) validates `--list`.

## Follow‑Ups

* Colored terminal UI / fuzzy search.
* GUI menu built in Bevy (enables hot reloading into demo scenes).
* Headless benchmarking (`--headless --frames 300`).
* Feature flag filtering (e.g., show only rendering / only physics demos).
