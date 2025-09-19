use bevy::prelude::*;
use crate::internal::{BallGpu as Ball, BallBuffer, ParamsUniform, TimeUniform, FieldTexture, AlbedoTexture, padded_slice};

// (Phase 2) keep internal types private to crate; no public re-export required yet.

#[derive(Resource)]
pub struct GpuMetaballBindGroup(pub bevy::render::render_resource::BindGroup);

#[derive(Resource)]
pub struct GpuBuffers { pub params: bevy::render::render_resource::Buffer, pub time: bevy::render::render_resource::Buffer, pub balls: bevy::render::render_resource::Buffer }

pub(crate) type CpuBallBuffer = BallBuffer;
pub(crate) type CpuParams = ParamsUniform;
pub(crate) type CpuTime = TimeUniform;
pub(crate) type CpuFieldTexture = FieldTexture;
pub(crate) type CpuAlbedoTexture = AlbedoTexture;
// Access padded_slice directly via the existing import.
