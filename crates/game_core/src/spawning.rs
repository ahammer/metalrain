use crate::{
    ActiveSpawnRotation, ArenaConfig, BallSpawnPolicy, BallSpawnPolicyMode, Paddle, PaddleControl,
    SpawnBallEvent, SpawnMetrics, SpawnPoint,
};
use bevy::prelude::*;
pub fn paddle_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    arena: Option<Res<ArenaConfig>>,
    mut q: Query<(&mut Transform, &Paddle)>,
) {
    let dt = time.delta_secs();
    for (mut transform, paddle) in &mut q {
        if !matches!(paddle.control, PaddleControl::Player) {
            continue;
        }
        let mut dir = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            dir.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            dir.y -= 1.0;
        }
        if dir != Vec2::ZERO {
            dir = dir.normalize();
        }
        let delta = dir * paddle.move_speed * dt;
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
        if let Some(arena) = arena.as_ref() {
            let half_w = arena.width * 0.5 - paddle.half_extents.x;
            let half_h = arena.height * 0.5 - paddle.half_extents.y;
            transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
            transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
        }
    }
}
pub fn process_spawn_points(
    time: Res<Time>,
    policy: Res<BallSpawnPolicy>,
    mut q: Query<(Entity, &mut SpawnPoint)>,
    mut writer: EventWriter<SpawnBallEvent>,
) {
    let BallSpawnPolicyMode::Auto(interval) = policy.mode else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut sp) in &mut q {
        if !sp.active {
            continue;
        }
        sp.timer += dt;
        if sp.timer >= interval && sp.cooldown <= 0.0 {
            sp.timer = 0.0;
            writer.write(SpawnBallEvent {
                spawn_entity: entity,
                override_position: None,
            });
        }
    }
}
pub fn maintain_spawn_rotation(
    mut rotation: ResMut<ActiveSpawnRotation>,
    changed: Query<(Entity, &SpawnPoint), Changed<SpawnPoint>>,
    all: Query<(Entity, &SpawnPoint)>,
) {
    let mut dirty = false;
    for (e, sp) in &changed {
        let present = rotation.indices.contains(&e);
        if sp.active && !present {
            rotation.indices.push(e);
            dirty = true;
        } else if !sp.active && present {
            rotation.indices.retain(|&x| x != e);
            dirty = true;
        }
    }
    rotation.indices.retain(|e| all.get(*e).is_ok());
    if dirty && rotation.current >= rotation.indices.len() {
        rotation.current = 0;
    }
}

pub struct PaddlePlugin;
impl Plugin for PaddlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, paddle_input_system);
    }
}

pub struct SpawningPlugin;
impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnBallEvent>()
            .init_resource::<BallSpawnPolicy>()
            .init_resource::<ActiveSpawnRotation>()
            .init_resource::<SpawnMetrics>()
            .add_systems(Update, (process_spawn_points, maintain_spawn_rotation));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_wraps() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(SpawningPlugin);
        let e1 = app.world_mut().spawn(SpawnPoint::default()).id();
        let e2 = app.world_mut().spawn(SpawnPoint::default()).id();
        app.update();
        let mut rot = app.world_mut().resource_mut::<ActiveSpawnRotation>();
        assert_eq!(rot.indices.len(), 2);
        assert_eq!(rot.current_entity(), Some(e1));
        rot.advance();
        assert_eq!(rot.current_entity(), Some(e2));
        rot.advance();
        assert_eq!(rot.current_entity(), Some(e1));
        {
            let mut sp = app.world_mut().get_mut::<SpawnPoint>(e1).unwrap();
            sp.active = false;
        }
        app.update();
        let rot2 = app.world().resource::<ActiveSpawnRotation>();
        assert_eq!(rot2.indices, vec![e2]);
    }
}
