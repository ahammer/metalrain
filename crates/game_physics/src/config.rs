use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct PhysicsConfig {
    pub pixels_per_meter: f32,
    pub gravity: Vec2,
    pub ball_restitution: f32,
    pub ball_friction: f32,
    pub clustering_strength: f32,
    pub clustering_radius: f32,
    pub max_ball_speed: f32,
    pub min_ball_speed: f32,
    pub optimize_clustering: bool,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            pixels_per_meter: 50.0,
            gravity: Vec2::new(0.0, -500.0),
            ball_restitution: 0.95,
            ball_friction: 0.1,
            clustering_strength: 100.0,
            clustering_radius: 150.0,
            max_ball_speed: 500.0,
            min_ball_speed: 100.0,
            optimize_clustering: true,
        }
    }
}
