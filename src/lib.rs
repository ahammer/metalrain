#![allow(clippy::module_inception, clippy::too_many_arguments)]

pub mod app;
pub mod core;
pub mod debug;
pub mod gameplay;
pub mod interaction;
pub mod physics;
pub mod rendering;

// Curated re-exports
pub use app::game::GamePlugin;
pub use core::components::{Ball, BallCircleVisual, BallRadius};
pub use core::config::{config::GameConfig, config::WindowConfig};
