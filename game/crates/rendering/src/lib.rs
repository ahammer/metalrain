// Phase 3 (in progress): Rendering crate basics.
// Adds: RenderingPlugin spawning a 2D camera (no clear), background grid (Material2d full‑screen quad), palette module.
// Future: circle mesh/material pipeline for balls, golden frame hash harness, camera controls & resizing logic refactor.

use bevy::prelude::*;

mod palette;
pub use palette::{Palette, BASE_COLORS, color_for_index};

pub mod background;
pub use background::{BackgroundPlugin, BgMaterial, BackgroundQuad};

mod circles;
pub use circles::CirclesPlugin;

pub struct RenderingPlugin;

#[derive(Component)]
pub struct GameCamera;

fn setup_camera(mut commands: Commands) {
    // Primary 2D camera. We disable automatic clear so background material draws first.
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::None,
            ..default()
        },
        GameCamera,
    ));
}

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BackgroundPlugin,                                // background grid first
            CirclesPlugin,                                   // circle visuals for Balls
        ))
            .add_systems(Startup, setup_camera)
            .insert_resource(ClearColor(Palette::BG)); // Fallback if background disabled later
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_spawns_camera_and_background() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(RenderingPlugin);
        app.update();

        // Camera present
        let world = app.world_mut();
        let mut q_cam = world.query::<&GameCamera>();
        assert_eq!(q_cam.iter(world).count(), 1, "expected exactly one GameCamera");

        // Background present
        let mut q_bg = world.query::<&background::BackgroundQuad>();
        assert_eq!(q_bg.iter(world).count(), 1, "expected background quad");
    }

    #[test]
    fn palette_wrap_and_constants() {
        assert_eq!(color_for_index(4), BASE_COLORS[0]);
        assert_eq!(Palette::BG, Color::srgb(0.02, 0.02, 0.05));
    }

    #[test]
    fn rendering_plugin_spawns_circle_for_ball() {
        use bm_core::{Ball, BallRadius, BallCircleVisual, CorePlugin};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(RenderingPlugin);
        app.add_plugins(CorePlugin);
        // Spawn a ball AFTER plugins so Added<Ball> triggers circle spawn
        app.world_mut().spawn((Ball, BallRadius(3.0)));
        app.update();

        let world = app.world_mut();
        let mut q = world.query::<&BallCircleVisual>();
        assert_eq!(q.iter(world).count(), 1, "expected one BallCircleVisual via RenderingPlugin");
    }
}
