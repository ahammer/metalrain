use crate::coordinates::MetaballCoordinateMapper;
use crate::internal::{BallBuffer, BallGpu, ParamsUniform, TimeUniform};
use crate::RuntimeSettings;
use crate::{MetaBall, MetaBallCluster, MetaBallColor};
use bevy::prelude::*;

/// Flag resource indicating the CPU->GPU ball buffer needs to be repacked this frame.
#[derive(Resource, Default, Deref, DerefMut)]
struct NeedsRepack(bool);

/// Local one-shot log guard (no need for a global resource).
#[derive(Default)]
struct LoggedOnce(bool);

pub(crate) struct PackingPlugin;
impl Plugin for PackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NeedsRepack>();
        app.add_systems(
            Update,
            (
                advance_time,
                mark_repack,
                gather_metaballs,
                sync_runtime_settings,
            )
                .chain(),
        );
    }
}

fn advance_time(time: Res<Time>, uni: Option<ResMut<TimeUniform>>) {
    if let Some(mut u) = uni {
        u.time += time.delta_secs();
    }
}

/// Mark that we need to repack if any relevant component set changed since last frame.
fn mark_repack(
    mut flag: ResMut<NeedsRepack>,
    added_ball: Query<Entity, Added<MetaBall>>,
    changed_ball_any: Query<Entity, Changed<MetaBall>>,
    changed_transform: Query<Entity, (With<MetaBall>, Changed<Transform>)>,
    changed_color: Query<Entity, Changed<MetaBallColor>>,
    changed_cluster: Query<Entity, Changed<MetaBallCluster>>,
    removed_ball: RemovedComponents<MetaBall>,
) {
    if **flag {
        return;
    } // already marked
    let changed = !added_ball.is_empty()
        || !changed_ball_any.is_empty()
        || !changed_transform.is_empty()
        || !changed_color.is_empty()
        || !changed_cluster.is_empty()
        || !removed_ball.is_empty();
    if changed {
        **flag = true;
    }
}

fn gather_metaballs(
    mut buffer: ResMut<BallBuffer>,
    mut params: ResMut<ParamsUniform>,
    mut flag: ResMut<NeedsRepack>,
    mapper: Res<MetaballCoordinateMapper>,
    query: Query<(
        &Transform,
        &MetaBall,
        Option<&MetaBallColor>,
        Option<&MetaBallCluster>,
    )>,
    mut logged: Local<LoggedOnce>,
) {
    if !**flag {
        return;
    } // no changes -> skip work
    buffer.balls.clear();
    buffer.balls.reserve(query.iter().len());
    for (tr, mb, color_opt, cluster_opt) in query.iter() {
        let world = tr.translation;
        let tex = mapper.world_to_metaball(world);
        let radius_tex = mapper.world_radius_to_tex(mb.radius_world);
        let c = color_opt
            .map(|c| c.0)
            .unwrap_or(LinearRgba::new(1.0, 1.0, 1.0, 1.0));
        buffer.balls.push(BallGpu {
            center: [tex.x, tex.y],
            radius: radius_tex,
            cluster_id: cluster_opt.map(|c| c.0).unwrap_or(0),
            color: [c.red, c.green, c.blue, c.alpha],
        });
    }
    params.num_balls = buffer.balls.len() as u32;
    **flag = false;
    if !logged.0 {
        info!(target: "metaballs", "initial pack: {} balls", buffer.balls.len());
        logged.0 = true;
    }
}

fn sync_runtime_settings(
    rt: Option<Res<RuntimeSettings>>,
    mut params: Option<ResMut<ParamsUniform>>,
) {
    let (Some(rt), Some(params)) = (rt, params.as_deref_mut()) else {
        return;
    };
    if !rt.is_changed() {
        return;
    }
    let desired = if rt.clustering_enabled { 1u32 } else { 0u32 };
    if params.clustering_enabled != desired {
        params.clustering_enabled = desired;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MetaBall, MetaBallColor};
    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(BallBuffer { balls: Vec::new() });
        app.insert_resource(ParamsUniform {
            screen_size: [1024.0, 1024.0],
            num_balls: 0,
            clustering_enabled: 1,
        });
        app.insert_resource(TimeUniform::default());
        app.init_resource::<NeedsRepack>();
        // Provide a mapper resource mimicking plugin insertion.
        app.insert_resource(crate::coordinates::MetaballCoordinateMapper::new(
            UVec2::new(1024, 1024),
            Vec2::new(-512.0, -512.0),
            Vec2::new(512.0, 512.0),
        ));
        app.add_systems(Update, (mark_repack, gather_metaballs).chain());
        app
    }

    #[test]
    fn initial_pack_counts_entities() {
        let mut app = setup_app();
        for i in 0..10 {
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(i as f32, i as f32, 0.0)),
                MetaBall { radius_world: 5.0 },
                MetaBallColor(LinearRgba::new(1.0, 1.0, 1.0, 1.0)),
            ));
        }
        app.update();
        let params = app.world().resource::<ParamsUniform>();
        assert_eq!(params.num_balls, 10);
    }

    #[test]
    fn no_repack_without_changes() {
        let mut app = setup_app();
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            MetaBall { radius_world: 5.0 },
        ));
        app.update(); // initial pack
        let first_ptr = app.world().resource::<BallBuffer>().balls.as_ptr();
        app.update(); // should skip
        let second_ptr = app.world().resource::<BallBuffer>().balls.as_ptr();
        assert_eq!(
            first_ptr, second_ptr,
            "Buffer reallocation occurred or repack executed unexpectedly"
        );
    }

    #[test]
    fn repack_after_component_change() {
        let mut app = setup_app();
        let e = app
            .world_mut()
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                MetaBall { radius_world: 5.0 },
            ))
            .id();
        app.update(); // initial pack
                      // mutate component
        {
            let mut tr = app.world_mut().get_mut::<Transform>(e).unwrap();
            tr.translation.x = 42.0;
        }
        app.update(); // should repack
        let world_ref = app.world();
        let buffer = world_ref.resource::<BallBuffer>();
        let mapper = world_ref.resource::<crate::coordinates::MetaballCoordinateMapper>();
        let expected = mapper.world_to_metaball(Vec3::new(42.0, 0.0, 0.0)).x;
        assert_eq!(buffer.balls[0].center[0], expected);
    }
}
