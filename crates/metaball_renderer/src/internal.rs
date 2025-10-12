use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;

pub const WORKGROUP_SIZE: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BallGpu {
    pub center: [f32; 2],
    pub radius: f32,
    pub cluster_id: i32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(
    Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource,
)]
pub struct TimeUniform {
    pub time: f32,
    _pad: [f32; 3],
}

#[repr(C, align(16))]
#[derive(
    Resource, Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable, ExtractResource,
)]
pub struct ParamsUniform {
    pub screen_size: [f32; 2],
    pub num_balls: u32,
    pub clustering_enabled: u32,
}

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct FieldTexture(pub Handle<Image>);
#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct AlbedoTexture(pub Handle<Image>);

#[derive(Resource, Clone, Debug, ExtractResource)]
pub struct NormalTexture(pub Handle<Image>);

#[derive(Resource, Clone, Debug, ExtractResource, Default)]
pub struct BallBuffer {
    pub balls: Vec<BallGpu>,
}
