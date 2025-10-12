use bevy::prelude::*;

#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBall {
    pub radius_world: f32,
}
#[deprecated(
    note = "Use Transform + MetaBall { radius_world } instead; centers now derived from Transform"
)]

#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallColor(pub LinearRgba);
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallCluster(pub i32);
