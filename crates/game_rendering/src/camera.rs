use bevy::prelude::*;

/// Component attached to the primary gameplay camera controlling shake/zoom state.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct GameCamera {
    pub base_resolution: Vec2,
    pub viewport_scale: f32,
    pub target_viewport_scale: f32,
    pub shake_intensity: f32,
    pub shake_decay_rate: f32,
    pub shake_offset: Vec2,
    pub zoom_bounds: Vec2,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            base_resolution: Vec2::new(1280.0, 720.0),
            viewport_scale: 1.0,
            target_viewport_scale: 1.0,
            shake_intensity: 0.0,
            shake_decay_rate: 2.5,
            shake_offset: Vec2::ZERO,
            zoom_bounds: Vec2::new(0.5, 2.0),
        }
    }
}

/// Configuration resource for camera behavior.
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct GameCameraSettings {
    pub shake_decay: f32,
    pub zoom_speed: f32,
}

impl Default for GameCameraSettings {
    fn default() -> Self {
        Self {
            shake_decay: 2.5,
            zoom_speed: 1.25,
        }
    }
}

/// Event to trigger camera shake with a starting intensity.
#[derive(Event, Debug, Clone, Copy)]
pub struct CameraShakeCommand {
    pub intensity: f32,
}

/// Event to request a zoom change (positive zooms in, negative zooms out).
#[derive(Event, Debug, Clone, Copy)]
pub struct CameraZoomCommand {
    pub delta_scale: f32,
}

/// Internal system: apply queued shake and zoom commands to the camera state.
pub fn apply_camera_commands(
    mut shake_events: EventReader<CameraShakeCommand>,
    mut zoom_events: EventReader<CameraZoomCommand>,
    mut query: Query<&mut GameCamera>,
) {
    for mut cam in &mut query {
        for ev in shake_events.read() {
            cam.shake_intensity = cam.shake_intensity.max(ev.intensity);
        }
        for ev in zoom_events.read() {
            // Interpret delta_scale as additive target scale delta (linear zoom)
            cam.target_viewport_scale = (cam.target_viewport_scale + ev.delta_scale)
                .clamp(cam.zoom_bounds.x, cam.zoom_bounds.y);
        }
    }
}

/// System: update camera shake decay, generate offset, and smooth zoom.
pub fn update_game_camera(time: Res<Time>, mut query: Query<&mut GameCamera>) {
    let dt = time.delta().as_secs_f32();
    let t = time.elapsed().as_secs_f32();
    for mut cam in &mut query {
        // Smooth zoom toward target.
        let smoothing_speed = 8.0; // higher = faster convergence
        let lerp_factor = (smoothing_speed * dt).min(1.0);
        cam.viewport_scale = cam
            .viewport_scale
            .lerp(cam.target_viewport_scale, lerp_factor);

        // Apply shake decay.
        if cam.shake_intensity > 0.0001 {
            cam.shake_intensity = (cam.shake_intensity - cam.shake_decay_rate * dt)
                .max(0.0);
            // Simple procedural offset using different frequencies.
            let freq1 = 17.0;
            let freq2 = 23.0;
            let x = (t * freq1).sin();
            let y = (t * freq2).cos();
            cam.shake_offset = Vec2::new(x, y) * cam.shake_intensity;
        } else {
            cam.shake_offset = Vec2::ZERO;
        }
    }
}

/// System: propagate GameCamera state to all per-layer cameras (orthographic) by adjusting
/// projection scale and transform translation (for shake offset). This keeps world-space
/// coordinates consistent while zooming and shaking.
pub fn apply_camera_to_layer_cameras(
    game_cam_q: Query<&GameCamera>,
    mut proj_q: Query<&mut Projection>,
    mut transform_q: Query<&mut Transform>,
    targets: Res<crate::targets::RenderTargets>,
) {
    let Some(game_cam) = game_cam_q.iter().next() else { return; };
    for layer in targets.layers.values() {
        if let Ok(mut projection) = proj_q.get_mut(layer.camera) {
            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scale = game_cam.viewport_scale;
            }
        }
        if let Ok(mut tr) = transform_q.get_mut(layer.camera) {
            tr.translation.x = game_cam.shake_offset.x;
            tr.translation.y = game_cam.shake_offset.y;
        }
    }
}

/// Helper to reset camera (used by demo reset key binding).
pub fn reset_camera(mut query: Query<&mut GameCamera>) {
    if let Some(mut cam) = query.iter_mut().next() {
        cam.viewport_scale = 1.0;
        cam.target_viewport_scale = 1.0;
        cam.shake_intensity = 0.0;
        cam.shake_offset = Vec2::ZERO;
    }
}
