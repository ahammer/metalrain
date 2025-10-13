use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy_rapier2d::prelude::RapierConfiguration;
use game_assets::GameAssets;
use game_physics::PhysicsConfig;
use game_rendering::{
    BlendMode, CompositorSettings, GameCamera, LayerBlendState, LayerToggleState, RenderLayer,
};
use metaball_renderer::RuntimeSettings;

use crate::resources::{
    ScaffoldConfig, ScaffoldHudState, ScaffoldMetaballMode, ScaffoldMetadata,
    ScaffoldPerformanceStats,
};

/// Marker component for the scaffold HUD node.
#[derive(Component)]
pub struct ScaffoldHud;

/// Spawns the scaffold HUD using loaded UI assets.
pub fn spawn_performance_hud(
    mut commands: Commands,
    assets: Res<GameAssets>,
    config: Res<ScaffoldConfig>,
) {
    let font = assets.fonts.ui_bold.clone();
    let position = Vec3::new(
        -config.world_half_extent + 24.0,
        config.world_half_extent - 24.0,
        500.0,
    );

    commands.spawn((
        Name::new("ScaffoldHUD"),
        Text2d::new("HUD initializing..."),
        TextFont {
            font,
            font_size: 16.0,
            ..Default::default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(position),
        RenderLayers::layer(RenderLayer::Ui.order()),
        ScaffoldHud,
    ));
}

/// Records rolling window stats for FPS display.
pub fn accumulate_performance_stats(time: Res<Time>, mut stats: ResMut<ScaffoldPerformanceStats>) {
    let timestamp = time.elapsed().as_secs_f32();
    let delta = time.delta().as_secs_f32();
    stats.record_sample(timestamp, delta);
}

fn fps_windows(stats: &ScaffoldPerformanceStats) -> (f32, f32) {
    let now = stats.last_sample_time;
    let mut count_one = 0u32;
    let mut time_one = 0.0;
    let mut count_five = 0u32;
    let mut time_five = 0.0;

    for (sample_time, dt) in stats.recent.iter().rev() {
        let age = now - *sample_time;
        if age <= 1.0 {
            count_one += 1;
            time_one += *dt;
        }
        if age <= 5.0 {
            count_five += 1;
            time_five += *dt;
        } else {
            break;
        }
    }

    let fps_one = if time_one > 0.0 {
        count_one as f32 / time_one
    } else {
        0.0
    };
    let fps_five = if time_five > 0.0 {
        count_five as f32 / time_five
    } else {
        0.0
    };

    (fps_one, fps_five)
}

/// Updates the HUD with diagnostics, layer status, and keybinding help.
pub fn update_performance_hud(
    diagnostics: Res<DiagnosticsStore>,
    layer_state: Res<LayerToggleState>,
    blend_state: Res<LayerBlendState>,
    settings: Res<CompositorSettings>,
    hud_state: Res<ScaffoldHudState>,
    metadata: Res<ScaffoldMetadata>,
    stats: Res<ScaffoldPerformanceStats>,
    runtime_settings: Res<RuntimeSettings>,
    metaball_mode: Res<ScaffoldMetaballMode>,
    physics_config: Res<PhysicsConfig>,
    rapier_config: Query<&RapierConfiguration>,
    camera_q: Query<&GameCamera>,
    entities_layers: Query<&RenderLayers>,
    mut text_q: Query<&mut Text2d, With<ScaffoldHud>>,
) {
    let Some(mut text) = text_q.iter_mut().next() else {
        return;
    };

    if !hud_state.visible {
        text.0 = "HUD hidden (F1 to show)".to_string();
        return;
    }

    let fps_instant = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) as f32;
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) as f32
        * 1000.0;

    let (fps_one, fps_five) = fps_windows(&stats);
    let camera_scale = camera_q
        .iter()
        .next()
        .map(|cam| cam.viewport_scale)
        .unwrap_or(1.0);
    let physics_paused = rapier_config
        .iter()
        .next()
        .map(|cfg| !cfg.physics_pipeline_active)
        .unwrap_or(false);

    let mut entity_counts = [0usize; 5];
    for layers in &entities_layers {
        for (index, layer) in RenderLayer::ALL.iter().enumerate() {
            if layers.intersects(&RenderLayers::layer(index)) {
                entity_counts[layer.order()] += 1;
            }
        }
    }

    let mut layer_lines = String::new();
    for layer in RenderLayer::ALL {
        let enabled = layer_state
            .config(layer)
            .map(|cfg| cfg.enabled)
            .unwrap_or(true);
        let blend = blend_state.blend_for(layer);
        let blend_label = match blend {
            BlendMode::Normal => "N",
            BlendMode::Additive => "A",
            BlendMode::Multiply => "M",
        };
        let status = if enabled { "ON " } else { "OFF" };
        let count = entity_counts[layer.order()];
        layer_lines.push_str(&format!(
            "[{}] {:10} {:3} | Ent:{:4} | Blend:{}\n",
            layer.order() + 1,
            layer.label(),
            status,
            count,
            blend_label
        ));
    }

    let gravity = physics_config.gravity;
    let metaball_label = metaball_mode.mode.label();
    let clustering = if runtime_settings.clustering_enabled {
        "on"
    } else {
        "off"
    };
    let exposure = settings.exposure;
    let boundaries = if settings.debug_layer_boundaries {
        "ON"
    } else {
        "OFF"
    };

    let hud_text = format!(
        "Demo: {}\nFPS {:.1} (1s {:.1} / 5s {:.1}) | Frame {:.2} ms\nExposure {:.2} | Zoom {:.2}x | Boundaries {}\nPhysics {} | Gravity ({:.0}, {:.0})\nMetaballs: {} (clustering {})\nLayers:\n{}Controls: 1-5 Layers  [-]/[]= Exposure  -/= Zoom  R Reset  Space Shake  P Pause  Arrows Gravity  B Background  M Metaballs  F1 HUD  Esc Exit",
        metadata.demo_name(),
        fps_instant,
        fps_one,
        fps_five,
        frame_time_ms,
        exposure,
        camera_scale,
        boundaries,
        if physics_paused { "Paused" } else { "Running" },
        gravity.x,
        gravity.y,
        metaball_label,
        clustering,
        layer_lines
    );

    text.0 = hud_text;
}
