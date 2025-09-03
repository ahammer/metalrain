pub mod embedded_levels;
pub mod layout;
pub mod loader;
pub mod registry; // Deprecated soon (replaced by embedded_levels abstraction)
pub mod wall_timeline;
pub mod widgets;

// Re-export primary plugin & resources for convenience
pub use loader::{LevelLoaderPlugin, LevelSelection, LevelWalls, LevelWidgets};
pub use wall_timeline::WallTimelinePlugin;
