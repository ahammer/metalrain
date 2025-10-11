# Asset Test Demo

Demonstrates standardized `game_assets` usage across native and (future) wasm builds.

Key points:

* Centralized loading via `configure_demo` helper (no hardcoded relative paths in demo code).
* Prints a single readiness line once all startup fonts + shaders are loaded.
* No build-time asset copying; runtime loads (native) or future compile-time embedding (wasm) only.

## Run (Native)

```pwsh
cargo run -p asset_test
```

Expected log (once):

```text
Asset_Test: All startup assets loaded (7 assets).
```

## WASM (Future Embedded Path)

When embedding & processed features are added, this demo should run unchanged under the wasm script with `-Embed`.

## Notes

This demo intentionally polls load state until a future `AssetsReady` resource is introduced in `game_assets`.
