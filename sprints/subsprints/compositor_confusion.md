## Issue: Metaballs Rendering to Wrong Layer

### Current Symptoms

1) Metaballs rendered in the background layer (layer 0)
2) Metaball layer (layer 2) not receiving any content
3) Appears to be rendering directly to screen instead of offscreen texture

### Root Cause

The metaball presentation quad entity is not properly assigned to the Metaballs RenderLayer. The `MetaballDisplayPlugin` creates a presentation quad but doesn't:

- Name it correctly for the configuration system to find
- Or directly assign it to the appropriate render layer

### Fix Implementation Plan

#### Step 1: Verify Entity Naming

Check if presentation quad has `Name::new("MetaballPresentationQuad")` component in `metaball_renderer/src/present/mod.rs`

#### Step 2: Add Debug Logging

In `compositor_test/src/lib.rs::configure_metaball_presentation`:

- Log all entities with Name components
- Log when "MetaballPresentationQuad" is found
- Verify RenderLayers assignment

#### Step 3: Fix Presentation Quad Spawning

In `metaball_renderer/src/present/mod.rs`, update the presentation quad spawn:

```rust
commands.spawn((
    Mesh2d(quad_handle),
    MeshMaterial2d(material_handle),
    Name::new("MetaballPresentationQuad"), // Add this
    RenderLayers::layer(0), // Default, will be overridden
    // ... other components
));
```

#### Step 4: Ensure System Ordering

In `compositor_test/src/lib.rs`:

```rust
.add_systems(
    Startup,
    configure_metaball_presentation
        .after(MetaballDisplaySet) // Run after metaball systems
)
```

#### Step 5: Alternative - Direct Layer Configuration

Add layer configuration to `MetaballRenderSettings`:

```rust
pub struct MetaballRenderSettings {
    pub presentation_layer: Option<u8>,
    // ... existing fields
}
```

Then in compositor_test, configure directly:

```rust
MetaballRendererPlugin::with(
    MetaballRenderSettings::default()
        .with_presentation_layer(2) // Metaballs layer
)
```

### Verification Checklist

- [ ] Presentation quad entity has Name component
- [ ] RenderLayers component shows layer 2 (use bevy inspector)
- [ ] Compositor shader samples correct layer texture
- [ ] Background layer (0) no longer shows metaballs
- [ ] Metaball layer (2) shows metaball content
- [ ] Blend mode is Normal for metaball layer composition
