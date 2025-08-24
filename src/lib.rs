pub mod app;
pub mod core;
pub mod debug;
pub mod gameplay;
pub mod interaction;
pub mod physics;
pub mod rendering;

// Curated re-exports
pub use core::config::{config::GameConfig, config::WindowConfig};
pub use core::components::{Ball, BallRadius, BallCircleVisual};
pub use app::game::GamePlugin;
