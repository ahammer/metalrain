//! Debug module: feature gated runtime visualization & stats/logging.
//! Built only when compiled with `--features debug`.

#[cfg(feature = "debug")]
pub mod keys; // pub for testing
#[cfg(feature = "debug")]
mod logging;
#[cfg(feature = "debug")]
mod modes;
#[cfg(feature = "debug")]
mod overlay;
#[cfg(feature = "debug")]
mod stats;

#[cfg(feature = "debug")]
pub use modes::*;

#[cfg(feature = "debug")]
use crate::core::system::system_order::PostPhysicsAdjustSet;
#[cfg(feature = "debug")]
use crate::interaction::inputmap::types::InputMap;
#[cfg(feature = "debug")]
use bevy::prelude::*;
#[cfg(feature = "debug")]
use crate::gameplay::spawn::spawn::{spawn_ball_entity, CircleMesh};
#[cfg(feature = "debug")]
use crate::rendering::materials::materials::{BallDisplayMaterials, BallPhysicsMaterials};
#[cfg(feature = "debug")]
use crate::core::config::GameConfig;
#[cfg(feature = "debug")]
use rand::Rng;

#[cfg(feature = "debug")]
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DebugPreRenderSet;

#[cfg(feature = "debug")]
pub struct DebugPlugin;
#[cfg(feature = "debug")]
impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        use crate::core::components::BallCircleVisual;
        #[cfg(feature = "debug")]
        use bevy_rapier2d::render::DebugRenderContext;
        use keys::debug_key_input_system;
        use logging::debug_logging_system;
        use modes::apply_mode_visual_overrides_system;
        use modes::propagate_metaballs_view_system;
        #[cfg(not(test))]
        use overlay::{debug_config_overlay_update, debug_overlay_spawn, debug_overlay_update};
        use stats::debug_stats_collect_system;

        fn toggle_circle_visibility(
            state: Res<modes::DebugState>,
            mut q_circles: Query<
                &mut Visibility,
                (
                    With<BallCircleVisual>,
                    Without<crate::rendering::metaballs::metaballs::MetaballsUnifiedQuad>,
                ),
            >,
            mut q_metaballs_quad: Query<
                &mut Visibility,
                With<crate::rendering::metaballs::metaballs::MetaballsUnifiedQuad>,
            >,
        ) {
            use modes::DebugRenderMode::*;
            // Circles only shown for rapier wireframe mode now
            let show_circles = matches!(state.mode, RapierWireframe);
            for mut vis in q_circles.iter_mut() {
                vis.set_if_neq(if show_circles {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                });
            }
            // Metaballs quad only visible for metaball-based modes
            let show_metaballs = matches!(
                state.mode,
                Metaballs | MetaballHeightfield | MetaballColorInfo
            );
            if let Ok(mut vis) = q_metaballs_quad.single_mut() {
                vis.set_if_neq(if show_metaballs {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                });
            }
        }

        #[cfg(feature = "debug")]
        fn toggle_rapier_debug(
            state: Res<modes::DebugState>,
            ctx: Option<ResMut<DebugRenderContext>>,
        ) {
            if let Some(mut c) = ctx {
                use modes::DebugRenderMode::*;
                let enable = matches!(state.mode, RapierWireframe);
                if c.enabled != enable {
                    c.enabled = enable;
                }
            }
        }

        #[cfg(feature = "debug")]
        fn debug_input_gizmos(input_map: Res<InputMap>, mut gizmos: Gizmos) {
            let rt = &input_map.gesture_rt;
            if rt.pointer_down {
                let p = rt.pointer_last;
                gizmos.circle_2d(p, 8.0, Color::srgb(1.0, 1.0, 0.2));
                if rt.dragging {
                    gizmos.line_2d(rt.pointer_start, p, Color::srgb(1.0, 0.5, 0.0));
                }
            }
        }

        // Number of balls spawned per debug button click
        const DEBUG_EXTRA_SPAWN_COUNT: usize = 100;

        fn debug_spawn_button_interact(
            mut commands: Commands,
            mut q_btn: Query<
                (&Interaction, &mut BackgroundColor),
                (Changed<Interaction>, With<overlay::DebugSpawnButton>),
            >,
            circle_mesh: Option<Res<CircleMesh>>,
            mut materials: ResMut<Assets<ColorMaterial>>,
            cfg: Res<GameConfig>,
            display_palette: Option<Res<BallDisplayMaterials>>,
            physics_palette: Option<Res<BallPhysicsMaterials>>,
        ) {
            for (interaction, mut bg) in q_btn.iter_mut() {
                match *interaction {
                    Interaction::Pressed => {
                        // Click -> spawn extra balls (if mesh ready)
                        *bg = BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.8));
                        if let Some(circle) = &circle_mesh {
                            spawn_debug_extra_balls(
                                &mut commands,
                                &circle.0,
                                &cfg,
                                display_palette.as_ref().map(|r| r.as_ref()),
                                physics_palette.as_ref().map(|r| r.as_ref()),
                                &mut materials,
                            );
                        } else {
                            // Mesh not yet ready; silently skip
                            info!("debug spawn button clicked before CircleMesh resource ready (skipping)");
                        }
                    }
                    Interaction::Hovered => {
                        *bg = BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.7));
                    }
                    Interaction::None => {
                        *bg = BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.6));
                    }
                }
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn spawn_debug_extra_balls(
            commands: &mut Commands,
            circle_mesh: &Handle<Mesh>,
            cfg: &GameConfig,
            display_palette: Option<&BallDisplayMaterials>,
            physics_palette: Option<&BallPhysicsMaterials>,
            materials: &mut ResMut<Assets<ColorMaterial>>,
        ) {
            let mut rng = rand::thread_rng();
            let c = &cfg.balls;
            let half_w = cfg.window.width * 0.5;
            let half_h = cfg.window.height * 0.5;

            for _ in 0..DEBUG_EXTRA_SPAWN_COUNT {
                let radius = if c.radius_range.min < c.radius_range.max {
                    rng.gen_range(c.radius_range.min..c.radius_range.max)
                } else {
                    c.radius_range.min
                };
                let x = rng.gen_range(-half_w..half_w);
                let y = rng.gen_range(-half_h..half_h);

                let vx = if c.vel_x_range.min < c.vel_x_range.max {
                    rng.gen_range(c.vel_x_range.min..c.vel_x_range.max)
                } else {
                    c.vel_x_range.min
                };
                let vy = if c.vel_y_range.min < c.vel_y_range.max {
                    rng.gen_range(c.vel_y_range.min..c.vel_y_range.max)
                } else {
                    c.vel_y_range.min
                };
                let vel = Vec2::new(vx, vy);

                let (material, restitution, variant_idx) =
                    if let (Some(disp), Some(phys)) = (display_palette, physics_palette) {
                        if !disp.0.is_empty() && !phys.0.is_empty() {
                            let idx_range_end = disp.0.len().min(phys.0.len());
                            let idx = if idx_range_end > 1 {
                                rng.gen_range(0..idx_range_end)
                            } else {
                                0
                            };
                            (disp.0[idx].clone(), phys.0[idx].restitution, idx)
                        } else {
                            let color = Color::srgb(
                                rng.gen::<f32>() * 0.9 + 0.1,
                                rng.gen::<f32>() * 0.9 + 0.1,
                                rng.gen::<f32>() * 0.9 + 0.1,
                            );
                            (materials.add(color), cfg.bounce.restitution, 0)
                        }
                    } else {
                        let color = Color::srgb(
                            rng.gen::<f32>() * 0.9 + 0.1,
                            rng.gen::<f32>() * 0.9 + 0.1,
                            rng.gen::<f32>() * 0.9 + 0.1,
                        );
                        (materials.add(color), cfg.bounce.restitution, 0)
                    };

                spawn_ball_entity(
                    commands,
                    circle_mesh,
                    Vec3::new(x, y, 0.0),
                    vel,
                    radius,
                    material,
                    restitution,
                    cfg.bounce.friction,
                    cfg.bounce.linear_damping,
                    cfg.bounce.angular_damping,
                    variant_idx,
                    cfg.draw_circles,
                );
            }

            info!(target: "spawn", "debug button spawned {} extra balls", DEBUG_EXTRA_SPAWN_COUNT);
        }

        app.init_resource::<modes::DebugState>()
            .init_resource::<modes::DebugStats>()
            .init_resource::<modes::DebugVisualOverrides>()
            .init_resource::<modes::LastAppliedMetaballsView>()
            .configure_sets(Update, DebugPreRenderSet.after(PostPhysicsAdjustSet));
        // In tests, skip overlay spawn (AssetServer not present with MinimalPlugins)
        #[cfg(not(test))]
        app.add_systems(Startup, debug_overlay_spawn);
        app.add_systems(
            Update,
            (
                debug_key_input_system,
                debug_stats_collect_system,
                apply_mode_visual_overrides_system,
                propagate_metaballs_view_system,
                toggle_circle_visibility,
                toggle_rapier_debug,
                debug_logging_system,
                debug_input_gizmos,
                #[cfg(not(test))]
                debug_config_overlay_update,
                #[cfg(not(test))]
                debug_overlay_update,
                debug_spawn_button_interact,
            )
                .in_set(DebugPreRenderSet),
        );
    }
}

#[cfg(not(feature = "debug"))]
pub struct DebugPlugin;
#[cfg(not(feature = "debug"))]
impl bevy::prelude::Plugin for DebugPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}
