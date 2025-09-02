use bevy::prelude::*;

/// High-level app lifecycle state.
/// MainMenu -> Loading -> Gameplay -> (Exiting TBD)
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    /// Player chooses a level.
    #[default]
    MainMenu,
    /// Transitional state while (re)loading a level.
    Loading,
    /// Active gameplay (sub-states refine play mode).
    Gameplay,
    /// Reserved for future graceful shutdown sequence.
    Exiting,
}

/// Gameplay sub-state (future expansion; minimal for now).
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameplayState {
    #[default]
    Playing,
    Paused,
    Intermission,
}
