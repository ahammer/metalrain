use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use game_core::{Hazard, HazardType, Paddle, Selected, SpawnPoint, Target, TargetState, Wall};

pub struct WidgetRendererPlugin;

impl Plugin for WidgetRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_wall_visuals,
                spawn_target_visuals,
                spawn_hazard_visuals,
                spawn_paddle_visuals,
                spawn_spawnpoint_visuals,
                update_target_animations,
                update_hazard_pulse,
                update_active_spawnpoint_pulse,
                update_selected_highlight,
                cleanup_destroyed_targets,
            ),
        );
    }
}

fn spawn_wall_visuals(mut commands: Commands, walls: Query<(Entity, &Wall), Added<Wall>>) {
    for (entity, wall) in &walls {
        let direction = (wall.end - wall.start).normalize_or_zero();
        let length = wall.length();
        let angle = direction.y.atan2(direction.x);
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(wall.color, Vec2::new(length, wall.thickness)),
            Transform::from_translation(wall.center().extend(0.0))
                .with_rotation(Quat::from_rotation_z(angle)),
            GlobalTransform::IDENTITY,
            Name::new("WallVisual"),
        ));
    }
}

fn spawn_target_visuals(mut commands: Commands, targets: Query<(Entity, &Target), Added<Target>>) {
    for (entity, target) in &targets {
        let size = Vec2::splat(target.radius * 2.0);
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(target.color, size),
            Transform::from_translation(Vec3::Z),
            GlobalTransform::IDENTITY,
            Name::new("TargetVisual"),
        ));
    }
}

fn spawn_hazard_visuals(mut commands: Commands, hazards: Query<(Entity, &Hazard), Added<Hazard>>) {
    for (entity, hazard) in &hazards {
        let size = hazard.size();
        let base_color = match hazard.hazard_type {
            HazardType::Pit => Color::srgba(0.8, 0.1, 0.1, 0.35),
        };
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(base_color, size),
            Transform::from_translation(hazard.center().extend(-0.1)),
            GlobalTransform::IDENTITY,
            Name::new("HazardVisual"),
        ));
    }
}

fn update_target_animations(
    time: Res<Time>,
    mut query: Query<(&mut Target, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (mut target, mut transform, mut sprite) in &mut query {
        match target.state {
            TargetState::Idle => {
                let health_ratio = if target.max_health > 0 {
                    target.health as f32 / target.max_health as f32
                } else {
                    0.0
                };
                let alpha = 0.5 + 0.5 * health_ratio;
                let lin = target.color.to_linear();
                sprite.color = Color::linear_rgba(lin.red, lin.green, lin.blue, alpha);
                transform.scale = Vec3::ONE;
            }
            TargetState::Hit(ref mut t) => {
                let new_t = (*t + dt * 4.0).min(1.0);
                let scale = 1.0 + 0.2 * (1.0 - new_t);
                transform.scale = Vec3::splat(scale);
                let tcol = target.color;
                let amt = (1.0 - new_t) * 0.5;
                let lin = tcol.to_linear();
                let nr = lin.red + (1.0 - lin.red) * amt;
                let ng = lin.green + (1.0 - lin.green) * amt;
                let nb = lin.blue + (1.0 - lin.blue) * amt;
                sprite.color = Color::linear_rgba(nr, ng, nb, lin.alpha);
                if new_t >= 1.0 {
                    target.state = TargetState::Idle;
                } else if let TargetState::Hit(ref mut inner) = target.state {
                    *inner = new_t;
                }
            }
            TargetState::Destroying(ref mut t) => {
                let new_t = (*t + dt * 2.0).min(1.0);
                transform.scale = Vec3::splat(1.0 + new_t * 0.4);
                let lin = target.color.to_linear();
                sprite.color = Color::linear_rgba(lin.red, lin.green, lin.blue, 1.0 - new_t);
                if new_t >= 1.0 { /* removal handled later */
                } else if let TargetState::Destroying(ref mut inner) = target.state {
                    *inner = new_t;
                }
            }
        }
    }
}

fn cleanup_destroyed_targets(mut commands: Commands, targets: Query<(Entity, &Target)>) {
    for (e, t) in &targets {
        if matches!(t.state, TargetState::Destroying(progress) if progress >= 1.0) {
            commands.entity(e).despawn();
        }
    }
}

fn update_hazard_pulse(time: Res<Time>, mut hazards: Query<&mut Sprite, With<Hazard>>) {
    let t = time.elapsed_secs_wrapped();
    for mut sprite in &mut hazards {
        let base = sprite.color.with_alpha(0.25);
        let pulse = (t * 2.0).sin() * 0.15 + 0.25;
        sprite.color = base.with_alpha(pulse.clamp(0.1, 0.5));
    }
}

fn spawn_paddle_visuals(mut commands: Commands, paddles: Query<(Entity, &Paddle), Added<Paddle>>) {
    for (entity, paddle) in &paddles {
        let size = paddle.half_extents * 2.0;
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(Color::srgb(0.1, 0.85, 0.95), size),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.2)),
            GlobalTransform::IDENTITY,
            Name::new("PaddleVisual"),
        ));
    }
}

fn spawn_spawnpoint_visuals(
    mut commands: Commands,
    spawns: Query<(Entity, &SpawnPoint), Added<SpawnPoint>>,
) {
    for (entity, sp) in &spawns {
        let r = sp.radius;
        commands.entity(entity).insert((
            RenderLayers::layer(1),
            Sprite::from_color(Color::srgb(0.9, 0.9, 0.25), Vec2::splat(r * 1.6)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.05)),
            GlobalTransform::IDENTITY,
            Name::new("SpawnPointVisual"),
        ));
        commands.entity(entity).with_children(|c| {
            c.spawn((
                RenderLayers::layer(1),
                Sprite::from_color(Color::srgb(1.0, 1.0, 0.5), Vec2::splat(r * 2.4)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.06)),
                GlobalTransform::IDENTITY,
                Name::new("SpawnPointRing"),
            ));
        });
    }
}

fn update_active_spawnpoint_pulse(
    time: Res<Time>,
    mut q: Query<(&SpawnPoint, &mut Transform, &mut Sprite)>,
) {
    let t = time.elapsed_secs_wrapped();
    for (sp, mut tf, mut sprite) in &mut q {
        if sp.active {
            let pulse = (t * 3.5).sin() * 0.15 + 1.0;
            tf.scale = Vec3::splat(pulse);
            sprite.color = sprite.color.with_alpha(0.9);
        } else {
            tf.scale = Vec3::ONE;
            sprite.color = sprite.color.with_alpha(0.4);
        }
    }
}

fn update_selected_highlight(mut q: Query<&mut Sprite, Added<Selected>>) {
    for mut sprite in &mut q {
        let lin = sprite.color.to_linear();
        sprite.color = Color::linear_rgba((lin.red + 0.3).min(1.0), lin.green, lin.blue, lin.alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plugin_builds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(WidgetRendererPlugin);
    }
}
