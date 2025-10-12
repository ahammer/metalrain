use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_rapier2d::render::RapierDebugRenderPlugin;

use background_renderer::{BackgroundConfig, BackgroundRendererPlugin};
use event_core::EventCorePlugin;
use game_assets::configure_demo;
use game_core::{ArenaConfig, GameCorePlugin};
use game_physics::GamePhysicsPlugin;
use game_rendering::{GameRenderingPlugin, RenderLayer, RenderSurfaceSettings};
use metaball_renderer::{MetaballRenderSettings, MetaballRendererPlugin};
use widget_renderer::WidgetRendererPlugin;

pub mod resources;
mod systems;

pub use resources::{
    MetaballMode, ScaffoldConfig, ScaffoldHudState, ScaffoldMetaballMode, ScaffoldMetadata,
    ScaffoldPerformanceStats,
};
pub use systems::{
    arena::spawn_physics_arena,
    camera::align_game_camera,
    hud::{accumulate_performance_stats, spawn_performance_hud, update_performance_hud},
    input::{exit_on_escape, handle_universal_inputs},
};

/// Label sets for systems exposed by the scaffold plugin.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ScaffoldSystemSet {
    /// Handles input bindings such as layer toggles and camera controls.
    Input,
    /// Updates performance overlays and diagnostics-driven UI.
    Hud,
}

/// High-level plugin wiring physics, rendering, assets, input, and diagnostics for demos.
#[derive(Debug, Clone)]
pub struct ScaffoldIntegrationPlugin {
    demo_name: String,
}

impl Default for ScaffoldIntegrationPlugin {
    fn default() -> Self {
        Self {
            demo_name: "Unnamed Demo".to_string(),
        }
    }
}

impl ScaffoldIntegrationPlugin {
    /// Assigns a demo name presented in the HUD diagnostics overlay.
    pub fn with_demo_name(name: impl Into<String>) -> Self {
        Self {
            demo_name: name.into(),
        }
    }
}

impl Plugin for ScaffoldIntegrationPlugin {
    fn build(&self, app: &mut App) {
        configure_demo(app);

        let config = app
            .world_mut()
            .get_resource_or_insert_with::<ScaffoldConfig>(ScaffoldConfig::default)
            .clone();

        {
            let mut arena = app
                .world_mut()
                .get_resource_or_insert_with::<ArenaConfig>(ArenaConfig::default);
            arena.width = config.world_half_extent * 2.0;
            arena.height = config.world_half_extent * 2.0;
        }

        if app.world().contains_resource::<RenderSurfaceSettings>() {
            let mut surface = app.world_mut().resource_mut::<RenderSurfaceSettings>();
            surface.base_resolution = config.base_resolution;
        } else {
            let mut surface = RenderSurfaceSettings::default();
            surface.base_resolution = config.base_resolution;
            app.insert_resource(surface);
        }

        if app
            .world()
            .contains_resource::<game_physics::PhysicsConfig>()
        {
            let mut physics = app
                .world_mut()
                .resource_mut::<game_physics::PhysicsConfig>();
            physics.gravity = config.default_gravity;
        } else {
            let mut physics_cfg = game_physics::PhysicsConfig::default();
            physics_cfg.gravity = config.default_gravity;
            app.insert_resource(physics_cfg);
        }

        if !app.world().contains_resource::<BackgroundConfig>() {
            app.insert_resource(BackgroundConfig::default());
        }

        app.insert_resource(ScaffoldMetadata::new(self.demo_name.clone()));
        app.init_resource::<ScaffoldHudState>();
        app.init_resource::<ScaffoldPerformanceStats>();
        app.init_resource::<ScaffoldMetaballMode>();

        let metaball_settings = MetaballRenderSettings::default()
            .with_texture_size(config.metaball_texture_size)
            .with_world_bounds(Rect::from_center_size(
                Vec2::ZERO,
                Vec2::splat(config.world_half_extent * 2.0),
            ))
            .clustering_enabled(true)
            .with_presentation(true)
            .with_presentation_layer(RenderLayer::Metaballs.order() as u8);

        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EventCorePlugin::default(),
            GameCorePlugin,
            GamePhysicsPlugin,
            GameRenderingPlugin,
            BackgroundRendererPlugin,
            WidgetRendererPlugin,
            MetaballRendererPlugin::with(metaball_settings),
        ));

        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(RapierDebugRenderPlugin::default());

        app.configure_sets(
            Update,
            (ScaffoldSystemSet::Input, ScaffoldSystemSet::Hud).chain(),
        );

        app.add_systems(Startup, (spawn_physics_arena, spawn_performance_hud));
        app.add_systems(PostStartup, align_game_camera);
        app.add_systems(
            Update,
            (
                handle_universal_inputs.in_set(ScaffoldSystemSet::Input),
                exit_on_escape
                    .after(handle_universal_inputs)
                    .in_set(ScaffoldSystemSet::Input),
                accumulate_performance_stats.in_set(ScaffoldSystemSet::Hud),
                update_performance_hud
                    .after(accumulate_performance_stats)
                    .in_set(ScaffoldSystemSet::Hud),
            ),
        );
    }
}
