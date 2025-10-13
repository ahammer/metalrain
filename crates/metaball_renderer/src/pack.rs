use crate::coordinates::MetaballCoordinateMapper;
use crate::internal::{BallBuffer, BallGpu, ParamsUniform, SpatialGrid, TimeUniform};
use crate::spatial::build_spatial_grid;
use crate::RuntimeSettings;
use crate::{MetaBall, MetaBallCluster, MetaBallColor};
use bevy::prelude::*;

#[derive(Resource, Default, Deref, DerefMut)]
struct NeedsRepack(bool);

#[derive(Default)]
struct LoggedOnce(bool);

pub(crate) struct PackingPlugin;
impl Plugin for PackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NeedsRepack>();
        app.init_resource::<SpatialGrid>();
        app.add_systems(
            Update,
            (
                advance_time,
                mark_repack,
                gather_metaballs,
                build_spatial_grid_system,
                update_grid_dimensions,
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
    }
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
    }

    // Clear only the active count, not the actual buffer (fixed capacity)
    buffer.free_indices.clear();
    let ball_count = query.iter().len();

    // Rebuild the buffer with new data
    // For now, simple approach: clear and rebuild
    // Future optimization: track entity IDs and update in-place
    buffer.balls.clear();
    buffer.balls.reserve(ball_count);
    buffer.active_count = 0;

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
        buffer.active_count += 1;
    }

    params.num_balls = buffer.balls.len() as u32;
    params.active_ball_count = buffer.active_count;
    **flag = false;

    if !logged.0 {
        info!(target: "metaballs", "initial pack: {} balls", buffer.balls.len());
        logged.0 = true;
    }
}

/// Build spatial grid from current ball data
fn build_spatial_grid_system(
    buffer: Res<BallBuffer>,
    params: Res<ParamsUniform>,
    mut grid: ResMut<SpatialGrid>,
) {
    if buffer.is_changed() || params.is_changed() {
        let screen_size = Vec2::new(params.screen_size[0], params.screen_size[1]);
        *grid = build_spatial_grid(&buffer.balls, screen_size);

        // Update params with grid dimensions
        // Note: This is done in a separate system to avoid mutation issues
    }
}

/// Update params with grid dimensions (runs after grid build)
fn update_grid_dimensions(grid: Res<SpatialGrid>, mut params: ResMut<ParamsUniform>) {
    if grid.is_changed() {
        params.grid_dimensions = [grid.dimensions.x, grid.dimensions.y];
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
        app.insert_resource(BallBuffer::default());
        app.insert_resource(ParamsUniform {
            screen_size: [1024.0, 1024.0],
            num_balls: 0,
            clustering_enabled: 1,
            grid_dimensions: [16, 16],
            active_ball_count: 0,
            _pad: 0,
        });
        app.insert_resource(TimeUniform::default());
        app.init_resource::<NeedsRepack>();
        app.init_resource::<SpatialGrid>();
        app.insert_resource(crate::coordinates::MetaballCoordinateMapper::new(
            UVec2::new(1024, 1024),
            Vec2::new(-512.0, -512.0),
            Vec2::new(512.0, 512.0),
        ));
        app.add_systems(
            Update,
            (
                mark_repack,
                gather_metaballs,
                build_spatial_grid_system,
                update_grid_dimensions,
            )
                .chain(),
        );
        app
    }

    #[test]
    fn initial_pack_counts_entities() {
        let mut app = setup_app();
        for i in 0..10 {
            app.world_mut().spawn((
                Transform::from_translation(Vec3::new(i as f32, i as f32, 0.0)),
                MetaBall { radius_world: 5.0 },
                #[allow(deprecated)]
                MetaBallColor(LinearRgba::new(1.0, 1.0, 1.0, 1.0)),
            ));
        }
        app.update();
        let params = app.world().resource::<ParamsUniform>();
        assert_eq!(params.num_balls, 10);
        assert_eq!(params.active_ball_count, 10);
    }

    #[test]
    fn no_repack_without_changes() {
        let mut app = setup_app();
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            MetaBall { radius_world: 5.0 },
        ));
        app.update();
        let first_ptr = app.world().resource::<BallBuffer>().balls.as_ptr();
        app.update();
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
        app.update();
        {
            let mut tr = app.world_mut().get_mut::<Transform>(e).unwrap();
            tr.translation.x = 42.0;
        }
        app.update();
        let world_ref = app.world();
        let buffer = world_ref.resource::<BallBuffer>();
        let mapper = world_ref.resource::<crate::coordinates::MetaballCoordinateMapper>();
        let expected = mapper.world_to_metaball(Vec3::new(42.0, 0.0, 0.0)).x;
        assert_eq!(buffer.balls[0].center[0], expected);
    }
}
