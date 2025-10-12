//! Input handling system for compositor controls.

use bevy::prelude::*;

use background_renderer::{BackgroundConfig, BackgroundMode};
use game_rendering::{
    BlendMode, CameraShakeCommand, CameraZoomCommand, CompositorSettings, GameCamera,
    LayerBlendState, LayerToggleState, RenderLayer,
};

use crate::resources::PerformanceOverlayState;

/// Handles keyboard input for controlling compositor features.
pub fn handle_compositor_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mut layer_state: ResMut<LayerToggleState>,
    mut blend_state: ResMut<LayerBlendState>,
    mut settings: ResMut<CompositorSettings>,
    mut shake_ev: EventWriter<CameraShakeCommand>,
    mut zoom_ev: EventWriter<CameraZoomCommand>,
    mut game_cam_q: Query<&mut GameCamera>,
    mut overlay_state: ResMut<PerformanceOverlayState>,
    mut bg_cfg: ResMut<BackgroundConfig>,
) {
    // Layer toggles (1-5)
    if keys.just_pressed(KeyCode::Digit1) {
        toggle_layer(&mut layer_state, RenderLayer::Background);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        toggle_layer(&mut layer_state, RenderLayer::GameWorld);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        toggle_layer(&mut layer_state, RenderLayer::Metaballs);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        toggle_layer(&mut layer_state, RenderLayer::Effects);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        toggle_layer(&mut layer_state, RenderLayer::Ui);
    }

    // Blend modes (Q/W/E)
    if keys.just_pressed(KeyCode::KeyQ) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Normal);
        info!(target: "compositor_demo", "Metaballs blend mode: Normal");
    }
    if keys.just_pressed(KeyCode::KeyW) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Additive);
        info!(target: "compositor_demo", "Metaballs blend mode: Additive");
    }
    if keys.just_pressed(KeyCode::KeyE) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Multiply);
        info!(target: "compositor_demo", "Metaballs blend mode: Multiply");
    }

    // Camera zoom (-/=)
    if keys.just_pressed(KeyCode::Minus) {
        zoom_ev.write(CameraZoomCommand { delta_scale: -0.1 });
    }
    if keys.just_pressed(KeyCode::Equal) {
        zoom_ev.write(CameraZoomCommand { delta_scale: 0.1 });
    }

    // Camera shake (Space)
    if keys.just_pressed(KeyCode::Space) {
        shake_ev.write(CameraShakeCommand { intensity: 12.0 });
    }

    // Exposure adjustment ([/])
    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.exposure = (settings.exposure - 0.1).clamp(0.1, 3.0);
        info!(
            target: "compositor_demo",
            "Exposure set to {:.2}",
            settings.exposure
        );
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.exposure = (settings.exposure + 0.1).clamp(0.1, 3.0);
        info!(
            target: "compositor_demo",
            "Exposure set to {:.2}",
            settings.exposure
        );
    }

    // Layer boundary debug (F2)
    if keys.just_pressed(KeyCode::F2) {
        settings.debug_layer_boundaries = !settings.debug_layer_boundaries;
        info!(
            target: "compositor_demo",
            "Layer boundary debug {}",
            if settings.debug_layer_boundaries {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    // HUD toggle (F1)
    if keys.just_pressed(KeyCode::F1) {
        overlay_state.visible = !overlay_state.visible;
        info!(target: "compositor_demo", "HUD {}", if overlay_state.visible { "shown" } else { "hidden" });
    }

    // Reset camera and exposure (R)
    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(mut cam) = game_cam_q.iter_mut().next() {
            cam.viewport_scale = 1.0;
            cam.target_viewport_scale = 1.0;
            cam.shake_intensity = 0.0;
            cam.shake_offset = Vec2::ZERO;
        }
        settings.exposure = 1.0;
        info!(target: "compositor_demo", "Camera & exposure reset");
    }

    // Background mode cycling (B)
    if keys.just_pressed(KeyCode::KeyB) {
        bg_cfg.mode = bg_cfg.mode.next();
        info!(target: "compositor_demo", "Background mode -> {:?}", bg_cfg.mode);
    }

    // Background angle adjustment (A/D)
    if keys.pressed(KeyCode::KeyA) {
        bg_cfg.angle += 0.9 * 0.016;
    }
    if keys.pressed(KeyCode::KeyD) {
        bg_cfg.angle -= 0.9 * 0.016;
    }

    // Radial gradient center adjustment (Arrow keys)
    if matches!(bg_cfg.mode, BackgroundMode::RadialGradient) {
        let mut changed = false;
        if keys.pressed(KeyCode::ArrowLeft) {
            bg_cfg.radial_center.x -= 0.25 * 0.016;
            changed = true;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            bg_cfg.radial_center.x += 0.25 * 0.016;
            changed = true;
        }
        if keys.pressed(KeyCode::ArrowUp) {
            bg_cfg.radial_center.y += 0.25 * 0.016;
            changed = true;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            bg_cfg.radial_center.y -= 0.25 * 0.016;
            changed = true;
        }
        if changed {
            bg_cfg.radial_center = bg_cfg.radial_center.clamp(Vec2::ZERO, Vec2::splat(1.0));
        }
    }
}

/// Helper function to toggle a layer's enabled state.
fn toggle_layer(state: &mut LayerToggleState, layer: RenderLayer) {
    if let Some(config) = state.config_mut(layer) {
        config.enabled = !config.enabled;
        info!(
            target: "compositor_demo",
            "{} layer {}",
            layer,
            if config.enabled { "enabled" } else { "disabled" }
        );
    }
}
