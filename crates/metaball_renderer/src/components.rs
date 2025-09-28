use bevy::prelude::*;

/// Authoritative metaball component (Sprint 2.1):
/// * Position comes from the entity `Transform` (XY on Z=0 plane).
/// * Radius stored in world units; converted to texture space during packing.
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBall {
    pub radius_world: f32,
}
#[deprecated(
    note = "Use Transform + MetaBall { radius_world } instead; centers now derived from Transform"
)]
#[allow(dead_code)]
#[derive(Component, Copy, Clone, Debug)]
pub struct LegacyMetaBall {
    pub center: Vec2,
    pub radius: f32,
}
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallColor(pub LinearRgba);
#[derive(Component, Copy, Clone, Debug)]
pub struct MetaBallCluster(pub i32);
