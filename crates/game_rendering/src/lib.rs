//! Game rendering pipeline crate (Sprint 3)
//!
//! Provides a multi-layer rendering orchestrator that manages per-layer render
//! targets, a compositor material, and camera utilities used across the game
//! and associated demos.

pub mod camera;
pub mod compositor;
pub mod layers;
pub mod plugin;
pub mod targets;

pub use camera::{CameraShakeCommand, CameraZoomCommand, GameCamera, GameCameraSettings};
pub use compositor::{
    CompositorMaterial, CompositorMaterialPlugin, CompositorPresentation, CompositorSettings,
    LayerBlendState,
};
pub use layers::{BlendMode, LayerConfig, LayerToggleState, RenderLayer};
pub use plugin::GameRenderingPlugin;
pub use targets::{LayerRenderTarget, RenderSurfaceSettings, RenderTargetHandles, RenderTargets};
