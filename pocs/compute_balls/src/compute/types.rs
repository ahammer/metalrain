use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use crate::constants::MAX_BALLS;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ball {
    pub center: [f32; 2],
    pub radius: f32,
    pub _pad: f32,
}

#[derive(Resource, Clone, ExtractResource)]
pub struct BallBuffer {
    pub balls: Vec<Ball>,
}

#[repr(C)]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct TimeUniform {
    pub time: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Resource, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, ExtractResource)]
pub struct ParamsUniform {
    pub screen_size: [f32; 2],
    pub num_balls: u32,
    pub debug_mode: u32,
    pub iso: f32,
    pub ambient: f32,
    pub rim_power: f32,
    pub show_centers: u32,
}

#[derive(Resource, Clone, ExtractResource)]
pub struct MetaballTarget {
    pub texture: Handle<Image>,
}

#[derive(Resource)]
pub struct GpuMetaballBindGroup(pub bevy::render::render_resource::BindGroup);

#[derive(Resource)]
pub struct GpuBuffers {
    pub params: bevy::render::render_resource::Buffer,
    pub time: bevy::render::render_resource::Buffer,
    pub balls: bevy::render::render_resource::Buffer,
}

impl Default for TimeUniform {
    fn default() -> Self { Self { time: 0.0, _pad: [0.0; 3] } }
}

pub fn padded_balls_slice(src: &[Ball]) -> [Ball; MAX_BALLS] {
    let mut fixed = [Ball { center: [0.0, 0.0], radius: 0.0, _pad: 0.0 }; MAX_BALLS];
    for (i, b) in src.iter().take(MAX_BALLS).enumerate() { fixed[i] = *b; }
    fixed
}
