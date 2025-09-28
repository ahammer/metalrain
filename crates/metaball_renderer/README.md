# Metaball Renderer (Sprint 2.1 Refactor)

Offscreen GPU compute + normals pass producing field + albedo textures for metaball blobs. Presentation & compositing are intentionally external: this crate now supplies only the offscreen data and coordinate mapping utilities.

## Key Changes (Sprint 2.1)

- Removed internal camera & on-screen quad (no automatic `Camera2d`).
- Added `MetaballCoordinateMapper` resource (world <-> texture mapping).
- Simplified `MetaBall` component: `Transform` holds world position; `radius_world` is the only field.
- Packing system maps `(Transform, MetaBall, Color/Cluster)` → GPU buffer each time data/transform changes.
- Public helpers for projection & picking: `project_world_to_screen`, `screen_to_world`, `screen_to_metaball_uv`.
- Configurable world bounds via `MetaballRenderSettings.world_bounds`.

## Usage

```rust
use bevy::prelude::*;
use metaball_renderer::{MetaballRendererPlugin, MetaballRenderSettings, MetaBall};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MetaballRendererPlugin::with(
            MetaballRenderSettings::default()
                .with_world_bounds(Rect::from_corners(vec2(-256.0,-256.0), vec2(256.0,256.0)))
        ))
        .add_systems(Startup, spawn_some)
        .run();
}

fn spawn_some(mut commands: Commands) {
    for i in 0..50 {
        let x = (i as f32 * 10.0) - 250.0;
        commands.spawn((
            Transform::from_translation(Vec3::new(x, 0.0, 0.0)),
            GlobalTransform::default(),
            MetaBall { radius_world: 12.0 },
        ));
    }
}
```

Fetch textures for custom compositing:

```rust
if let Some((field, albedo)) = metaball_renderer::metaball_textures(world) {
    // sample or blit in a custom render pass
}
```

## Migration (Pre-Refactor → Sprint 2.1)

Old:

```rust
commands.spawn(MetaBall { center: world_to_tex(pos), radius: r });
```

New:

```rust
commands.spawn((Transform::from_translation(pos.extend(0.0)), MetaBall { radius_world: r }));
```

Mapping is automatic during packing; remove any manual sync systems updating `MetaBall.center`.

Optional legacy adapter kept (deprecated) as `LegacyMetaBall` for temporary transitional code.

## Coordinate Mapping

`MetaballCoordinateMapper` provides:

- `world_to_metaball(world: Vec3) -> Vec2`
- `metaball_to_uv(tex: Vec2) -> Vec2`
- `world_radius_to_tex(r: f32) -> f32`

World bounds (`Rect`) define the min/max XY that map to the texture region `[0..W, 0..H]`.

## Testing

Unit tests assert:

- Corner world points map to (0,0)/(W,H)
- UV of mapped world points stays inside [0,1]
- Radius scaling matches expected pixel span

## Roadmap (Sprint 3 Preview)

- External compositor / layered pipeline (UI, background, metaballs, effects)
- Dynamic resolution scaling
- Post-effects (glow/bloom) applied after composition

## License

Dual-licensed under MIT or Apache-2.0 at your option.
