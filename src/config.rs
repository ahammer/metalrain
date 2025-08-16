use bevy::prelude::*;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Resource, Clone)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub title: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GravityConfig {
    pub y: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BounceConfig {
    pub restitution: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpawnRange<T> {
    pub min: T,
    pub max: T,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BallSpawnConfig {
    pub count: usize,
    pub radius_range: SpawnRange<f32>,
    pub x_range: SpawnRange<f32>,
    pub y_range: SpawnRange<f32>,
    pub vel_x_range: SpawnRange<f32>,
    pub vel_y_range: SpawnRange<f32>,
}

#[derive(Debug, Deserialize, Resource, Clone)]
pub struct GameConfig {
    pub window: WindowConfig,
    pub gravity: GravityConfig,
    pub bounce: BounceConfig,
    pub balls: BallSpawnConfig,
    pub separation: CollisionSeparationConfig,
    pub rapier_debug: bool,
    pub draw_circles: bool,
    pub metaballs_enabled: bool,
    pub draw_cluster_bounds: bool,
    pub interactions: InteractionConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CollisionSeparationConfig {
    pub enabled: bool,
    pub overlap_slop: f32,      // multiply radii sum by this to decide early push
    pub push_strength: f32,     // scalar for position correction amount
    pub max_push: f32,          // clamp for stability
    pub velocity_dampen: f32,   // how much to damp relative velocity along normal (0..1)
}

#[derive(Debug, Deserialize, Clone)]
pub struct InteractionConfig {
    pub explosion: ExplosionConfig,
    pub drag: DragConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExplosionConfig {
    pub enabled: bool,
    pub impulse: f32,     // base impulse magnitude applied at center
    pub radius: f32,      // effect radius (world units)
    pub falloff_exp: f32, // force scaled by (1 - d/radius)^falloff_exp (clamped)
}

#[derive(Debug, Deserialize, Clone)]
pub struct DragConfig {
    pub enabled: bool,
    pub grab_radius: f32,   // max distance from pointer to grab a ball at press
    pub pull_strength: f32, // acceleration magnitude toward pointer (units/sec^2)
    pub max_speed: f32,     // optional cap on speed while dragged (0 = no cap)
}

impl GameConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let data = fs::read_to_string(&path).map_err(|e| format!("read config: {e}"))?;
        ron::from_str(&data).map_err(|e| format!("parse RON: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_sample_config() {
        let sample = r#"(
            window: (width: 800.0, height: 600.0, title: "Test"),
            gravity: (y: -9.8),
            bounce: (restitution: 0.5),
            balls: (
                count: 10,
                radius_range: (min: 1.0, max: 2.0),
                x_range: (min: -10.0, max: 10.0),
                y_range: (min: -5.0, max: 5.0),
                vel_x_range: (min: -1.0, max: 1.0),
                vel_y_range: (min: 0.0, max: 2.0),
            ),
            separation: (
                enabled: true,
                overlap_slop: 0.98,
                push_strength: 0.5,
                max_push: 10.0,
                velocity_dampen: 0.2,
            ),
            rapier_debug: false,
            draw_circles: true,
            metaballs_enabled: true,
            draw_cluster_bounds: false,
            interactions: (
                explosion: (enabled: true, impulse: 500.0, radius: 200.0, falloff_exp: 1.0),
                drag: (enabled: true, grab_radius: 30.0, pull_strength: 800.0, max_speed: 1200.0),
            ),
        )"#;
        let mut file = tempfile::NamedTempFile::new().expect("tmp file");
        file.write_all(sample.as_bytes()).unwrap();
        let cfg = GameConfig::load_from_file(file.path()).expect("parse config");
        assert_eq!(cfg.window.width, 800.0);
        assert_eq!(cfg.balls.count, 10);
        assert_eq!(cfg.bounce.restitution, 0.5);
    }
}
