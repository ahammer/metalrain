# Sub-Sprint: Background Renderer Crate Implementation (UPDATED)

## Goal

Deliver a dedicated `background_renderer` crate that supplies a reusable, configurable background rendering system (solid / linear / radial / animated gradients) integrated with the existing asset pipeline (`game_assets`) and renderer layering, using a single draw call and zero per‑frame allocations after setup.

## Current State (Post-Scaffolding)

| Item | Status | Notes |
|------|--------|-------|
| Crate added to workspace | ✅ | `crates/background_renderer` registered in root `Cargo.toml` |
| Core source files | ✅ | `config.rs`, `material.rs`, `systems.rs`, `lib.rs` created |
| Shader asset | ✅ | `assets/shaders/background.wgsl` implemented |
| Asset loader integration | ✅ | `background` handle added to `ShaderAssets` in `game_assets` |
| Material plugin wiring | ✅ | `BackgroundRendererPlugin` registers material + systems |
| Runtime config resource | ✅ | `BackgroundConfig` with four modes + parameters |
| Update system | ✅ | Only mutates GPU uniforms when config changes or animated |
| Cleanup system | ⏳ (Optional) | Simple despawn helper present but not yet scheduled for state exit |
| Demo / example | 🔜 | To be added (either new `demos/background_test` or integrate into existing demo) |

## Mode Definitions

1. `Solid` – single color fill
2. `LinearGradient` – two-color gradient with angle (radians)
3. `RadialGradient` – center + radius falloff
4. `Animated` – vertical wave blend over time (time + speed params)

## Public API Snapshot

```rust
pub use background_renderer::{
    BackgroundRendererPlugin,
    BackgroundConfig, BackgroundMode,
};
```

Add to any Bevy app after calling one of the `configure_*` helpers from `game_assets` so the shader path resolves.

## Crate Structure

```
crates/background_renderer/
├── Cargo.toml
└── src/
    ├── lib.rs          # Plugin + type registration
    ├── config.rs       # Modes & BackgroundConfig resource
    ├── material.rs     # GPU material (AsBindGroup + Material2d)
    └── systems.rs      # Setup & update systems
assets/shaders/background.wgsl  # WGSL fragment logic (group(2) material uniforms)
```

## Key Implementation Details

### Config

`BackgroundConfig` (Resource, Reflect) holds colors, angle, radial parameters, animation speed & mode. Default = linear gradient subtle dark purple.

### Material

`BackgroundMaterial` packs all uniforms into a single bind group (mode, two colors, params vec4, radial center). Uses `Material2d` fragment shader path `shaders/background.wgsl` so reuse is automatic once the asset server loads the file (loaded up-front by `game_assets`).

### Systems

`setup_background`: spawns a large quad scaled to cover view (RenderLayers layer 0).
`update_background`: updates material only when config changed OR animated mode active to minimize uniform writes.
Optional `cleanup_background` retained for future state-driven lifecycle.

### Shader (`background.wgsl`)

Uniform layout mirrors `AsBindGroup` ordering. Supports the four modes via a `switch` on `material.mode`. Linear gradient uses rotation of centered UV; radial uses distance; animated applies sine wave modulation.

## Performance Considerations

* Single draw call (full-screen quad) – negligible CPU overhead.
* No per-frame allocations after spawn.
* Update system early-outs on unchanged, non-animated modes.

## Integration Steps (Consumer Crate)

```rust
use bevy::prelude::*;
use game_assets::configure_demo; // or configure_game_crate / configure_workspace_root
use background_renderer::{BackgroundRendererPlugin, BackgroundConfig, BackgroundMode};

fn main() {
    let mut app = App::new();
    configure_demo(&mut app); // sets AssetPlugin root + GameAssetsPlugin
    app.add_plugins(DefaultPlugins)
        .add_plugins(BackgroundRendererPlugin)
        .insert_resource(BackgroundConfig { mode: BackgroundMode::RadialGradient, ..default() })
        .add_systems(Startup, |mut commands: Commands| { commands.spawn(Camera2d); })
        .run();
}
```

## Remaining Work (Next Tasks)

| Task | Priority | Notes |
|------|----------|-------|
| Add demo crate `demos/background_test` | High | Validate hot-reload & mode cycling |
| Input system to cycle modes & tweak params | High | Space to cycle, arrow keys adjust radial center, A/D adjust angle |
| Optional cleanup system scheduling | Medium | Hook into a future state machine if introduced |
| Documentation / README for crate | Medium | Quick usage + mode table |
| Add unit / light integration test (panic check) | Low | Ensure plugin registers without panic |
| Integrate into compositor test demo | Low | Add plugin + config to existing demo for visual validation |

### Progress Note (Integration)

Integrated into `demos/compositor_test`: replaced manual solid sprite background with `BackgroundRendererPlugin` + interactive controls (B cycle modes, A/D rotate angle, arrows move radial center in radial mode). Build verified.

## Testing Checklist

* Renders behind all other layers (verify compositor layering precedence)
* Mode cycling produces expected visual transitions
* No uniform updates when idle (Solid/Linear/Radial)
* Animated mode exhibits smooth wave; speed param respected
* Hot reloading shader (if file_watcher feature active) updates fragment logic without restart
* No warnings about missing shader or bind groups

## Future Enhancements (Deferred)

* Multi-stop gradients / gradient textures
* Procedural noise overlays (fbm / simplex)
* Parallax multi-layer backgrounds
* Day/Night color curve blending
* Theme-driven presets (level biome integration)
* Post-process bloom hook (if pipeline added)

## Risk & Mitigation

| Risk | Mitigation |
|------|------------|
| Shader bind group mismatch | Mirrored layout, single uniform struct, early compile test |
| Asset path resolution issues | Always configure assets before adding plugin |
| Layer conflicts | Hard-coded to `RenderLayers::layer(0)`; document contract |
| Overdraw / performance regression | Single quad -> negligible; profiler validation |

## Metrics Targets

* < 0.05 ms CPU per frame (avg)
* 0 allocations/frame after setup
* 0 shader recompilation warnings

## Code Snapshot (Representative)

```rust
// systems.rs (excerpt)
pub fn update_background(
    time: Res<Time>,
    config: Res<BackgroundConfig>,
    mut materials: ResMut<Assets<BackgroundMaterial>>,
    q: Query<&MeshMaterial2d<BackgroundMaterial>, With<BackgroundEntity>>,
) {
    let animated = matches!(config.mode, BackgroundMode::Animated);
    if !config.is_changed() && !animated { return; }
    let t = time.elapsed_secs();
    for h in &q { if let Some(m) = materials.get_mut(&h.0) { m.update_from_config(&config, t); } }
}
```

## Done Definition

This subsprint is considered complete once: crate builds, shader loads without warnings, all four modes function, documentation in this file updated, and a consumer demo exists (or explicit acceptance to defer demo).

---
Updated: Implementation scaffolding committed; follow-up tasks listed above.
