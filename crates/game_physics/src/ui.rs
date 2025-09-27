//! UI systems for adjusting `PhysicsConfig` at runtime (feature `ui`).
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::PhysicsConfig;

pub fn physics_config_panel(mut egui_ctx: EguiContexts, mut config: ResMut<PhysicsConfig>) {
    let ctx = egui_ctx.ctx_mut();
    egui::Window::new("Physics").show(ctx, |ui| {
        ui.heading("Forces");
        ui.horizontal(|ui| { ui.label("Gravity X"); ui.add(egui::Slider::new(&mut config.gravity.x, -1000.0..=1000.0)); });
        ui.horizontal(|ui| { ui.label("Gravity Y"); ui.add(egui::Slider::new(&mut config.gravity.y, -1000.0..=1000.0)); });
        ui.add_space(4.0);
        ui.heading("Clustering");
        ui.add(egui::Slider::new(&mut config.clustering_strength, 0.0..=500.0).text("Strength"));
        ui.add(egui::Slider::new(&mut config.clustering_radius, 10.0..=400.0).text("Radius"));
        ui.checkbox(&mut config.optimize_clustering, "Optimize");
        ui.add_space(4.0);
        ui.heading("Motion Limits");
        ui.add(egui::Slider::new(&mut config.max_ball_speed, 50.0..=1500.0).text("Max Speed"));
        ui.add(egui::Slider::new(&mut config.min_ball_speed, 0.0..=500.0).text("Min Speed"));
        ui.add_space(4.0);
        ui.heading("Material");
        ui.add(egui::Slider::new(&mut config.ball_restitution, 0.0..=1.0).text("Restitution"));
        ui.add(egui::Slider::new(&mut config.ball_friction, 0.0..=1.0).text("Friction"));
    });
}
