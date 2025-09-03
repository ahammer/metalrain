pub mod registry; // Deprecated soon (replaced by embedded_levels abstraction)
pub mod layout;
pub mod widgets;
pub mod loader;
pub mod wall_timeline;
pub mod embedded_levels;

// Re-export primary plugin & resources for convenience
pub use loader::{LevelLoaderPlugin, LevelSelection, LevelWalls, LevelWidgets};
pub use wall_timeline::{WallTimelinePlugin};
