# Bevy Source Submodule

This project vendors the Bevy game engine source as a git submodule for convenient offline browsing of:

- Engine and crate source code
- Examples (`external/bevy/examples`)
- Documentation comments / generated docs
- CHANGELOG / upgrade notes

Pinned version: **v0.16.0** (tag `v0.16.0`). The dependency in `Cargo.toml` still uses crates.io `bevy = "0.16"`.

## Typical Tasks

### Initialize / Update After Clone
If you freshly cloned this repository:
```pwsh
git submodule update --init --recursive
```

### Pull Latest Changes (keeping same tag)
No action needed unless you intentionally move to a newer tag. To update to a newer release (e.g. `v0.16.1` once published):
```pwsh
cd external/bevy
git fetch --tags origin
git checkout v0.16.1
cd ../..
git add external/bevy
git commit -m "Update Bevy submodule to v0.16.1"
```

### Regenerate Local Docs
Build docs (uses your current toolchain):
```pwsh
cargo doc -p bevy --open
```
If using the crates.io dependency (current setup), this renders the published crate sources, not necessarily the submodule copies. To force docs for the submodule commit specifically without altering dependencies, you can temporarily patch:
```toml
# (Optional) in Cargo.toml for a one-off doc build
[patch.crates-io]
bevy = { path = "external/bevy/crates/bevy" }
```
Then run `cargo doc`, and afterwards revert the patch.

### Running Examples
Unless you add a path patch, run examples directly from the submodule workspace:
```pwsh
cd external/bevy
cargo run --example sprite --features bevy_sprite,bevy_winit
```
Add any feature flags required by that example (inspect `examples/*` for guidance). Some examples rely on default features; since `ball_matcher` disables Bevy default features, running from the submodule avoids affecting your main crate.

### Keeping Submodule Clean
Avoid committing local experimental changes inside `external/bevy`; if you need to patch Bevy, create a branch inside the submodule and commit there, then commit the updated submodule pointer in the parent repo.

### Removing the Submodule
If you ever decide to remove it:
```pwsh
git rm external/bevy
rm .git/modules/external/bevy -Recurse -Force
```
(Verify paths on Windows / PowerShell). Commit the removal.

## Rationale
Having the engine source locally simplifies debugging and code navigation (IDE symbol lookup) without forcing the project to build against an unpublished fork.

---
