//! HUD and performance tracking systems.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use game_rendering::{
    BlendMode, CompositorSettings, GameCamera, LayerBlendState, LayerToggleState, RenderLayer,
};

use crate::components::HudText;
use crate::resources::{FrameCounter, LayerHudCache, PerformanceOverlayState, PerformanceStats};

/// Accumulates performance statistics over time.
pub fn accumulate_performance_stats(
    time: Res<Time>,
    mut stats: ResMut<PerformanceStats>,
    mut frame_counter: ResMut<FrameCounter>,
) {
    frame_counter.frame += 1;
    stats.frames = frame_counter.frame;
    let now = time.elapsed().as_secs_f32();
    let dt = time.delta().as_secs_f32();
    stats.last_sample_time = now;
    stats.recent.push_back((now, dt));
    while let Some((t, _)) = stats.recent.front() {
        if now - *t > 6.0 {
            stats.recent.pop_front();
        } else {
            break;
        }
    }
}

/// Computes FPS for 1-second and 5-second windows.
fn compute_fps_windows(stats: &PerformanceStats) -> (f32, f32) {
    let now = stats.last_sample_time;
    let mut count_1s = 0u32;
    let mut time_1s = 0.0;
    let mut count_5s = 0u32;
    let mut time_5s = 0.0;
    for (t, dt) in stats.recent.iter().rev() {
        let age = now - *t;
        if age <= 1.0 {
            count_1s += 1;
            time_1s += *dt;
        }
        if age <= 5.0 {
            count_5s += 1;
            time_5s += *dt;
        } else {
            break;
        }
    }
    let fps_1s = if time_1s > 0.0 {
        count_1s as f32 / time_1s
    } else {
        0.0
    };
    let fps_5s = if time_5s > 0.0 {
        count_5s as f32 / time_5s
    } else {
        0.0
    };
    (fps_1s, fps_5s)
}

/// Updates the HUD text with current performance and layer state.
pub fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    layer_toggles: Res<LayerToggleState>,
    blend_state: Res<LayerBlendState>,
    settings: Res<CompositorSettings>,
    overlay_state: Res<PerformanceOverlayState>,
    mut cache: ResMut<LayerHudCache>,
    stats: Res<PerformanceStats>,
    cam_q: Query<&GameCamera>,
    mut text_q: Query<&mut Text2d, With<HudText>>,
    entities_layers: Query<&RenderLayers>,
) {
    let mut text = if let Some(t) = text_q.iter_mut().next() {
        t
    } else {
        return;
    };
    if !overlay_state.visible {
        text.0 = "(HUD hidden - F1)".to_string();
        return;
    }

    let mut enabled = [true; 5];
    for cfg in &layer_toggles.configs {
        enabled[cfg.layer.order()] = cfg.enabled;
    }
    let mut blends = [BlendMode::Normal; 5];
    for layer in RenderLayer::ALL {
        blends[layer.order()] = blend_state.blend_for(layer);
    }
    let camera_scale = cam_q.iter().next().map(|c| c.viewport_scale).unwrap_or(1.0);
    let (fps_1s, fps_5s) = compute_fps_windows(&stats);
    let fps_instant = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(fps_1s as f64) as f32;
    let frame_time_ms = (diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
        * 1000.0) as f32;
    let mut counts = [0usize; 5];
    for rl in &entities_layers {
        for (i, _) in RenderLayer::ALL.iter().enumerate() {
            if rl.intersects(&RenderLayers::layer(i)) {
                counts[i] += 1;
            }
        }
    }
    let mut needs_rebuild = false;
    if enabled != cache.last_enabled {
        needs_rebuild = true;
        cache.last_enabled = enabled;
    }
    if blends != cache.last_blends {
        needs_rebuild = true;
        cache.last_blends = blends;
    }
    if (settings.exposure - cache.last_exposure).abs() > f32::EPSILON {
        needs_rebuild = true;
        cache.last_exposure = settings.exposure;
    }
    if settings.debug_layer_boundaries != cache.last_boundary_debug {
        needs_rebuild = true;
        cache.last_boundary_debug = settings.debug_layer_boundaries;
    }
    if (camera_scale - cache.last_camera_scale).abs() > 1e-4 {
        needs_rebuild = true;
        cache.last_camera_scale = camera_scale;
    }
    if !needs_rebuild {
        return;
    }
    let layer_lines: String = RenderLayer::ALL
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let en = if enabled[i] { "ON " } else { "OFF" };
            let blend = match blends[i] {
                BlendMode::Normal => "N",
                BlendMode::Additive => "A",
                BlendMode::Multiply => "M",
            };
            format!(
                "[{}] {:10} {:3} | Ent:{:4} | B:{}\n",
                i + 1,
                l.label(),
                en,
                counts[i],
                blend
            )
        })
        .collect();
    let exposure = settings.exposure;
    let boundaries = if settings.debug_layer_boundaries {
        "ON"
    } else {
        "OFF"
    };
    let hud = format!(
        "Layers:\n{layer_lines}FPS: {fps_instant:.1} (1s:{fps_1s:.1} 5s:{fps_5s:.1})\nFrame: {}  {:.2} ms\nExposure: {exposure:.2}  Boundaries:{boundaries}\nZoom: {camera_scale:.2}x  (F1 HUD 1-5 Layers Q/W/E Blend +/- Zoom [ ] Exposure F2 Bounds R Reset)",
        stats.frames, frame_time_ms
    );
    cache.last_text = hud.clone();
    text.0 = hud;
}

/// Logs a performance snapshot every 600 frames.
pub fn log_periodic_performance_snapshot(
    diagnostics: Res<DiagnosticsStore>,
    frame_counter: Res<FrameCounter>,
) {
    if frame_counter.frame == 0 || frame_counter.frame % 600 != 0 {
        return;
    }
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0)
        * 1000.0;
    info!(target: "perf_snapshot", "Frame {} | FPS {:.2} | Frame {:.2} ms", frame_counter.frame, fps, frame_time_ms);
}
