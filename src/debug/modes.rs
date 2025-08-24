#[cfg(feature = "debug")]
use bevy::prelude::*;

#[cfg(feature = "debug")]
use crate::rendering::metaballs::metaballs::MetaballsToggle;

#[cfg(feature = "debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugRenderMode {
    Metaballs,
    RapierWireframe,
    MetaballHeightfield,
    MetaballColorInfo,
}

#[cfg(feature = "debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaballsViewVariant {
    Normal,
    Heightfield,
    ColorInfo,
}

#[cfg(feature = "debug")]
#[derive(Resource)]
pub struct DebugState {
    pub mode: DebugRenderMode,
    pub last_mode: DebugRenderMode,
    pub overlay_visible: bool,
    pub log_interval: f32,
    pub time_accum: f32,
    pub frame_counter: u64,
    pub last_ball_count: usize,
    pub last_cluster_count: usize,
    // Alert cooldown tracking (frame indices of last emitted alerts)
    pub last_frame_time_alert_frame: u64,
    pub last_ball_alert_frame: u64,
    pub last_cluster_alert_frame: u64,
}

#[cfg(feature = "debug")]
impl Default for DebugState {
    fn default() -> Self {
        Self {
            mode: DebugRenderMode::Metaballs,
            last_mode: DebugRenderMode::Metaballs,
            overlay_visible: true,
            log_interval: 1.0,
            time_accum: 0.0,
            frame_counter: 0,
            last_ball_count: 0,
            last_cluster_count: 0,
            last_frame_time_alert_frame: 0,
            last_ball_alert_frame: 0,
            last_cluster_alert_frame: 0,
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Resource, Default, Debug, Clone)]
pub struct DebugStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub ball_count: usize,
    pub cluster_count: usize,
    pub truncated_balls: bool,
    pub metaballs_encoded: usize,
}

#[cfg(feature = "debug")]
#[derive(Resource, Debug, Clone)]
pub struct DebugVisualOverrides {
    pub draw_circles: Option<bool>,
    pub draw_cluster_bounds: Option<bool>,
    pub rapier_debug_enabled: Option<bool>,
    pub metaballs_enabled: Option<bool>,
    pub metaballs_view_variant: MetaballsViewVariant,
}

#[cfg(feature = "debug")]
impl Default for DebugVisualOverrides {
    fn default() -> Self {
        Self {
            draw_circles: None,
            draw_cluster_bounds: None,
            rapier_debug_enabled: None,
            metaballs_enabled: None,
            metaballs_view_variant: MetaballsViewVariant::Normal,
        }
    }
}

#[cfg(feature = "debug")]
pub fn apply_mode_visual_overrides_system(
    mut overrides: ResMut<DebugVisualOverrides>,
    state: Res<DebugState>,
    mut metaballs_toggle: ResMut<MetaballsToggle>,
) {
    use DebugRenderMode::*;
    let variant = match state.mode {
        Metaballs => MetaballsViewVariant::Normal,
        RapierWireframe => MetaballsViewVariant::Normal,
        MetaballHeightfield => MetaballsViewVariant::Heightfield,
        MetaballColorInfo => MetaballsViewVariant::ColorInfo,
    };
    overrides.metaballs_view_variant = variant;
    metaballs_toggle.0 = matches!(
        state.mode,
        Metaballs | MetaballHeightfield | MetaballColorInfo
    );
}

#[derive(Resource, Default)]
pub struct LastAppliedMetaballsView(pub u32);

pub fn propagate_metaballs_view_system(
    overrides: Res<DebugVisualOverrides>,
    mut last: ResMut<LastAppliedMetaballsView>,
    mut materials: ResMut<Assets<crate::rendering::metaballs::metaballs::MetaballsUnifiedMaterial>>,
    q_mat: Query<
        &bevy::sprite::MeshMaterial2d<
            crate::rendering::metaballs::metaballs::MetaballsUnifiedMaterial,
        >,
        With<crate::rendering::metaballs::metaballs::MetaballsUnifiedQuad>,
    >,
) {
    // Only run if changed
    let view_id = match overrides.metaballs_view_variant {
        MetaballsViewVariant::Normal => 0u32,
        MetaballsViewVariant::Heightfield => 1u32,
        MetaballsViewVariant::ColorInfo => 2u32,
    };
    if view_id == last.0 {
        return;
    }
    if let Ok(handle_comp) = q_mat.single() {
        if let Some(mat) = materials.get_mut(&handle_comp.0) {
            mat.set_debug_view(view_id);
            last.0 = view_id;
        }
    }
}
