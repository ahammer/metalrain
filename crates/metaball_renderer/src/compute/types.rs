use bevy::prelude::*;

// (Phase 2) keep internal types private to crate; no public re-export required yet.

#[derive(Resource)]
pub struct GpuMetaballBindGroup(pub bevy::render::render_resource::BindGroup);

#[derive(Resource)]
pub struct GpuBuffers { pub params: bevy::render::render_resource::Buffer, pub time: bevy::render::render_resource::Buffer, pub balls: bevy::render::render_resource::Buffer }

// Removed unused CPU-side alias types to reduce warnings (previously: BallBuffer, ParamsUniform, TimeUniform, Field/Albedo textures).
