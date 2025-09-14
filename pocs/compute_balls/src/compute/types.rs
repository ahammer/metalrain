use bevy::prelude::*;

// REPLACED: moved core types to metaball.rs
// Keeping this module temporarily to avoid large path churn; re-export new names.

pub use crate::metaball::{
    Ball,
    MetaballBuffer as BallBuffer,
    MetaballParams as ParamsUniform,
    MetaballTime as TimeUniform,
    MetaballOutputTexture as MetaballTarget,
    padded_balls_slice,
};

// GPU-side resources remain here (bind group + buffers wrappers)
#[derive(Resource)]
pub struct GpuMetaballBindGroup(pub bevy::render::render_resource::BindGroup);

#[derive(Resource)]
pub struct GpuBuffers {
    pub params: bevy::render::render_resource::Buffer,
    pub time: bevy::render::render_resource::Buffer,
    pub balls: bevy::render::render_resource::Buffer,
}
