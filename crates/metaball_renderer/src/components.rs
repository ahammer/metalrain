use bevy::prelude::*;

#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBall { pub center: Vec2, pub radius: f32 }
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallColor(pub LinearRgba);
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallCluster(pub i32);
