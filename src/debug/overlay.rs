#[cfg(feature = "debug")]
use bevy::prelude::*;
// Bevy 0.16 text API uses components: Text, TextFont, TextColor, Node for UI text.
#[cfg(feature = "debug")]
use super::modes::{DebugStats, DebugState, MetaballsViewVariant};

#[cfg(feature = "debug")]
#[derive(Component)]
pub(crate) struct DebugOverlayText;

#[cfg(feature = "debug")]
pub fn debug_overlay_spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Top-left anchored UI text node.
    let font_path = "fonts/FiraSans-Bold.ttf";
    let custom_path = format!("assets/{font_path}");
    let (font_handle, initial_text) = if std::path::Path::new(&custom_path).exists() {
        (asset_server.load(font_path), String::from("(loading debug font...)"))
    } else {
        warn!("Debug overlay font missing at {custom_path}. Using fallback font. To improve clarity, add a TTF at {custom_path}.");
        (Handle::default(), String::from("(fallback font)"))
    };
    commands.spawn((
        Text::new(initial_text),
        TextFont { font: font_handle, font_size: 14.0, ..Default::default() },
        TextColor(Color::WHITE),
        // Absolute positioned node in top-left.
        bevy::ui::Node { position_type: bevy::ui::PositionType::Absolute, top: Val::Px(4.0), left: Val::Px(6.0), ..Default::default() },
        DebugOverlayText,
    ));
}

#[cfg(feature = "debug")]
pub(crate) fn debug_overlay_update(state: Res<DebugState>, stats: Res<DebugStats>, mut q_text: Query<&mut Text, With<DebugOverlayText>>) {
    if let Ok(mut text) = q_text.single_mut() {
        if !state.overlay_visible { text.0.clear(); return; }
        if !(state.is_changed() || stats.is_changed()) { return; }
        let metaballs_variant = match state.mode {
            super::modes::DebugRenderMode::Metaballs => MetaballsViewVariant::Normal,
            super::modes::DebugRenderMode::MetaballHeightfield => MetaballsViewVariant::Heightfield,
            super::modes::DebugRenderMode::MetaballColorInfo => MetaballsViewVariant::ColorInfo,
            _ => MetaballsViewVariant::Normal,
        };
        text.0 = format!(
            "FPS {:.1} ft {:.1}ms balls {} enc {}/{} trunc {} clusters {} mode {:?} view {:?}",
            stats.fps,
            stats.frame_time_ms,
            stats.ball_count,
            stats.metaballs_encoded,
            stats.ball_count,
            stats.truncated_balls,
            stats.cluster_count,
            state.mode,
            metaballs_variant
        );
    }
}
