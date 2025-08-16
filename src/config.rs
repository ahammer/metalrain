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
        )"#;
        let mut file = tempfile::NamedTempFile::new().expect("tmp file");
        file.write_all(sample.as_bytes()).unwrap();
        let cfg = GameConfig::load_from_file(file.path()).expect("parse config");
        assert_eq!(cfg.window.width, 800.0);
        assert_eq!(cfg.balls.count, 10);
        assert_eq!(cfg.bounce.restitution, 0.5);
    }
}
