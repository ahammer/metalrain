#[cfg(feature = "debug")]
use bevy::prelude::*;
// Bevy 0.16 text API uses components: Text, TextFont, TextColor, Node for UI text.
#[cfg(feature = "debug")]
use super::modes::{DebugStats, DebugState, MetaballsViewVariant};
#[cfg(feature = "debug")]
use crate::config::GameConfig;

#[cfg(feature = "debug")]
#[derive(Component)]
pub(crate) struct DebugOverlayText;

#[cfg(feature = "debug")]
#[derive(Component)]
pub(crate) struct DebugConfigOverlayText;

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
    // Main (top-left) stats overlay
    commands.spawn((
        Text::new(initial_text),
        TextFont { font: font_handle.clone(), font_size: 14.0, ..Default::default() },
        TextColor(Color::WHITE),
        // Absolute positioned node in top-left.
        bevy::ui::Node { position_type: bevy::ui::PositionType::Absolute, top: Val::Px(4.0), left: Val::Px(6.0), ..Default::default() },
        DebugOverlayText,
    ));

    // Bottom-left config overlay (starts blank until first update)
    commands.spawn((
        Text::new(String::new()),
        TextFont { font: font_handle.clone(), font_size: 13.0, ..Default::default() },
        TextColor(Color::srgb(0.75, 0.85, 0.95)),
        bevy::ui::Node { position_type: bevy::ui::PositionType::Absolute, bottom: Val::Px(4.0), left: Val::Px(6.0), ..Default::default() },
        DebugConfigOverlayText,
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

#[cfg(feature = "debug")]
pub(crate) fn debug_config_overlay_update(
    state: Res<DebugState>,
    cfg: Res<GameConfig>,
    mut q_text: Query<&mut Text, With<DebugConfigOverlayText>>,
) {
    if let Ok(mut text) = q_text.single_mut() {
        if !state.overlay_visible { text.0.clear(); return; }
    // Regenerate when state visibility toggled OR config changed OR first run.
    if !(state.is_changed() || cfg.is_changed()) && !text.0.is_empty() { return; }
        // Compact multi-line representation; keep within ~120 cols.
        let b = &cfg.balls;
        let sep = &cfg.separation;
        let ex = &cfg.interactions.explosion;
        let dr = &cfg.interactions.drag;
        let mb = &cfg.metaballs;
        text.0 = format!(
            "CFG window {w:.0}x{h:.0} gravY {gy} rest {rest:.2}\n \
balls n={bc} r[{rmin:.0}-{rmax:.0}] vx[{vxmin:.0},{vxmax:.0}] vy[{vymin:.0},{vymax:.0}]\n \
sep {sepen} slop {slop:.2} push {push:.2} max {maxp:.1} damp {damp:.2}\n \
expl {exen} imp {imp:.0} r {er:.0} fall {fall:.2} | drag {dren} grab {grab:.0} pull {pull:.0} vmax {vmax:.0}\n \
metab all={mben} iso {iso:.2} nz {nz:.1} mtl {metal:.2} rough {rough:.2} env {env:.2} spec {spec:.2} hard {hard} cbexp {cbexp:.2}",
            w = cfg.window.width, h = cfg.window.height,
            gy = cfg.gravity.y,
            rest = cfg.bounce.restitution,
            bc = b.count,
            rmin = b.radius_range.min, rmax = b.radius_range.max,
            vxmin = b.vel_x_range.min, vxmax = b.vel_x_range.max,
            vymin = b.vel_y_range.min, vymax = b.vel_y_range.max,
            sepen = if sep.enabled {"on"} else {"off"},
            slop = sep.overlap_slop, push = sep.push_strength, maxp = sep.max_push, damp = sep.velocity_dampen,
            exen = if ex.enabled {"on"} else {"off"}, imp = ex.impulse, er = ex.radius, fall = ex.falloff_exp,
            dren = if dr.enabled {"on"} else {"off"}, grab = dr.grab_radius, pull = dr.pull_strength, vmax = dr.max_speed,
            mben = if cfg.metaballs_enabled {"on"} else {"off"}, iso = mb.iso, nz = mb.normal_z_scale, metal = mb.metallic,
            rough = mb.roughness, env = mb.env_intensity, spec = mb.spec_intensity, hard = mb.hard_cluster_boundaries,
            cbexp = mb.color_blend_exponent,
        );
    }
}
