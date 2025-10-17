use background_renderer::{BackgroundConfig, BackgroundMode};
use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierConfiguration;
use game_physics::PhysicsConfig;
use game_rendering::{
    BlendMode, CameraShakeCommand, CameraZoomCommand, CompositorSettings, GameCamera,
    LayerBlendState, LayerToggleState, RenderLayer,
};
use metaball_renderer::RuntimeSettings;

use crate::resources::{MetaballMode, ScaffoldConfig, ScaffoldHudState, ScaffoldMetaballMode};

const LOG_TARGET: &str = "scaffold";

/// Applies universal demo bindings for rendering, physics, and hud controls.
pub fn handle_universal_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut layer_state: ResMut<LayerToggleState>,
    mut blend_state: ResMut<LayerBlendState>,
    mut settings: ResMut<CompositorSettings>,
    mut hud_state: ResMut<ScaffoldHudState>,
    mut physics_config: ResMut<PhysicsConfig>,
    mut background: ResMut<BackgroundConfig>,
    mut camera_q: Query<&mut GameCamera>,
    mut metaball_runtime: ResMut<RuntimeSettings>,
    mut metaball_mode: ResMut<ScaffoldMetaballMode>,
    mut rapier_config: Query<&mut RapierConfiguration>,
    mut zoom_events: EventWriter<CameraZoomCommand>,
    mut shake_events: EventWriter<CameraShakeCommand>,
    config: Res<ScaffoldConfig>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        log_layer(
            toggle_layer(&mut layer_state, RenderLayer::Background),
            RenderLayer::Background,
        );
    }
    if keys.just_pressed(KeyCode::Digit2) {
        log_layer(
            toggle_layer(&mut layer_state, RenderLayer::GameWorld),
            RenderLayer::GameWorld,
        );
    }
    if keys.just_pressed(KeyCode::Digit3) {
        log_layer(
            toggle_layer(&mut layer_state, RenderLayer::Metaballs),
            RenderLayer::Metaballs,
        );
    }
    // Layers 4 & 5 removed (Effects, Ui)

    if keys.just_pressed(KeyCode::KeyQ) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Normal);
        info!(target: LOG_TARGET, "Metaballs blend -> Normal");
    }
    if keys.just_pressed(KeyCode::KeyW) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Additive);
        info!(target: LOG_TARGET, "Metaballs blend -> Additive");
    }
    if keys.just_pressed(KeyCode::KeyE) {
        blend_state.set_blend_for(RenderLayer::Metaballs, BlendMode::Multiply);
        info!(target: LOG_TARGET, "Metaballs blend -> Multiply");
    }

    if keys.just_pressed(KeyCode::Minus) {
        zoom_events.write(CameraZoomCommand { delta_scale: -0.1 });
    }
    if keys.just_pressed(KeyCode::Equal) {
        zoom_events.write(CameraZoomCommand { delta_scale: 0.1 });
    }

    if keys.just_pressed(KeyCode::Space) {
        shake_events.write(CameraShakeCommand { intensity: 12.0 });
    }

    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.exposure = (settings.exposure - 0.1).clamp(0.1, 3.0);
        info!(target: LOG_TARGET, "Exposure -> {:.2}", settings.exposure);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.exposure = (settings.exposure + 0.1).clamp(0.1, 3.0);
        info!(target: LOG_TARGET, "Exposure -> {:.2}", settings.exposure);
    }

    if keys.just_pressed(KeyCode::F2) {
        settings.debug_layer_boundaries = !settings.debug_layer_boundaries;
        info!(
            target: LOG_TARGET,
            "Layer boundary debug {}",
            if settings.debug_layer_boundaries { "enabled" } else { "disabled" }
        );
    }

    if keys.just_pressed(KeyCode::F1) {
        hud_state.visible = !hud_state.visible;
        info!(target: LOG_TARGET, "HUD {}", if hud_state.visible { "shown" } else { "hidden" });
    }

    if keys.just_pressed(KeyCode::KeyR) {
        if let Some(mut cam) = camera_q.iter_mut().next() {
            cam.viewport_scale = 1.0;
            cam.target_viewport_scale = 1.0;
            cam.shake_intensity = 0.0;
            cam.shake_offset = Vec2::ZERO;
        }
        settings.exposure = 1.0;
        physics_config.gravity = config.default_gravity;
        info!(target: LOG_TARGET, "Camera, exposure, and gravity reset");
    }

    if keys.just_pressed(KeyCode::KeyP) {
        if let Some(mut cfg) = rapier_config.iter_mut().next() {
            cfg.physics_pipeline_active = !cfg.physics_pipeline_active;
            info!(
                target: LOG_TARGET,
                "Physics {}",
                if cfg.physics_pipeline_active { "resumed" } else { "paused" }
            );
        }
    }

    let delta = time.delta().as_secs_f32();
    let gravity_step = 500.0 * delta;
    let mut gravity_changed = false;

    if keys.pressed(KeyCode::ArrowLeft) {
        physics_config.gravity.x -= gravity_step;
        gravity_changed = true;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        physics_config.gravity.x += gravity_step;
        gravity_changed = true;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        physics_config.gravity.y += gravity_step;
        gravity_changed = true;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        physics_config.gravity.y -= gravity_step;
        gravity_changed = true;
    }
    if gravity_changed {
        let g = physics_config.gravity;
        info!(target: LOG_TARGET, "Gravity -> ({:.0}, {:.0})", g.x, g.y);
    }

    if keys.just_pressed(KeyCode::KeyB) {
        background.mode = background.mode.next();
        info!(target: LOG_TARGET, "Background mode -> {:?}", background.mode);
    }
    if background.mode == BackgroundMode::RadialGradient {
        let mut changed = false;
        if keys.pressed(KeyCode::KeyA) {
            background.angle += 0.9 * delta;
            changed = true;
        }
        if keys.pressed(KeyCode::KeyD) {
            background.angle -= 0.9 * delta;
            changed = true;
        }
        if changed {
            info!(target: LOG_TARGET, "Background angle -> {:.2}", background.angle);
        }
    }

    if keys.just_pressed(KeyCode::KeyM) {
        metaball_mode.mode = metaball_mode.mode.next();
        match metaball_mode.mode {
            MetaballMode::Clustered => {
                metaball_runtime.clustering_enabled = true;
                if let Some(cfg) = layer_state.config_mut(RenderLayer::Metaballs) {
                    cfg.enabled = true;
                }
                info!(target: LOG_TARGET, "Metaballs mode -> Clustered");
            }
            MetaballMode::NoClustering => {
                metaball_runtime.clustering_enabled = false;
                if let Some(cfg) = layer_state.config_mut(RenderLayer::Metaballs) {
                    cfg.enabled = true;
                }
                info!(target: LOG_TARGET, "Metaballs mode -> No Clustering");
            }
            MetaballMode::Hidden => {
                metaball_runtime.clustering_enabled = false;
                if let Some(cfg) = layer_state.config_mut(RenderLayer::Metaballs) {
                    cfg.enabled = false;
                }
                info!(target: LOG_TARGET, "Metaballs mode -> Hidden");
            }
        }
    }
}

fn log_layer(enabled: bool, layer: RenderLayer) {
    info!(
        target: LOG_TARGET,
        "{} layer {}",
        layer,
        if enabled { "enabled" } else { "disabled" }
    );
}

fn toggle_layer(state: &mut LayerToggleState, layer: RenderLayer) -> bool {
    if let Some(cfg) = state.config_mut(layer) {
        cfg.enabled = !cfg.enabled;
        cfg.enabled
    } else {
        true
    }
}

/// Sends an AppExit event when Escape is pressed.
pub fn exit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}
