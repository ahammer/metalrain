use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::Material2d,
};

use crate::config::BackgroundConfig;

#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct BackgroundMaterial {
    #[uniform(0)]
    pub mode: u32,
    #[uniform(0)]
    pub primary_color: Vec4,
    #[uniform(0)]
    pub secondary_color: Vec4,
    #[uniform(0)]
    pub params: Vec4,
    #[uniform(0)]
    pub radial_center: Vec2,
}

impl BackgroundMaterial {
    pub fn from_config(config: &BackgroundConfig, time: f32) -> Self {
        Self {
            mode: config.mode.as_u32(),
            primary_color: Vec4::from_array(config.primary_color.to_f32_array()),
            secondary_color: Vec4::from_array(config.secondary_color.to_f32_array()),
            params: Vec4::new(
                config.angle,
                time,
                config.animation_speed,
                config.radial_radius,
            ),
            radial_center: config.radial_center,
        }
    }
    pub fn update_from_config(&mut self, config: &BackgroundConfig, time: f32) {
        self.mode = config.mode.as_u32();
        self.primary_color = Vec4::from_array(config.primary_color.to_f32_array());
        self.secondary_color = Vec4::from_array(config.secondary_color.to_f32_array());
        self.params = Vec4::new(
            config.angle,
            time,
            config.animation_speed,
            config.radial_radius,
        );
        self.radial_center = config.radial_center;
    }
}

impl Material2d for BackgroundMaterial {
    fn fragment_shader() -> ShaderRef { ShaderRef::Path("shaders/background.wgsl".into()) }
}
