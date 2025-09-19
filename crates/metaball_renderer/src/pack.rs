use bevy::prelude::*;
use crate::{MetaBall, MetaBallColor, MetaBallCluster};
use crate::internal::{BallBuffer, BallGpu, ParamsUniform, TimeUniform};
use crate::RuntimeSettings;

#[derive(Resource, Default)]
struct PackLogOnce(bool);

pub(crate) struct PackingPlugin;
impl Plugin for PackingPlugin { fn build(&self, app: &mut App) {
    app.init_resource::<PackLogOnce>()
        .add_systems(Update, (advance_time, gather_metaballs, sync_runtime_settings));
}}

fn advance_time(time: Res<Time>, uni: Option<ResMut<TimeUniform>>) { if let Some(mut u) = uni { u.time += time.delta_secs(); } }

fn gather_metaballs(
    mut buffer: ResMut<BallBuffer>,
    mut params: ResMut<ParamsUniform>,
    mut logged: ResMut<PackLogOnce>,
    query: Query<(&MetaBall, Option<&MetaBallColor>, Option<&MetaBallCluster>)>,
) {
    buffer.balls.clear();
    for (mb, color_opt, cluster_opt) in query.iter() {
        let c = color_opt.map(|c| c.0).unwrap_or(LinearRgba::new(1.0,1.0,1.0,1.0));
        buffer.balls.push(BallGpu {
            center:[mb.center.x, mb.center.y],
            radius: mb.radius,
            cluster_id: cluster_opt.map(|c| c.0).unwrap_or(0),
            color:[c.red, c.green, c.blue, c.alpha]
        });
    }
    params.num_balls = buffer.balls.len() as u32;
    if !logged.0 { info!(target: "metaballs", "packed {} balls", buffer.balls.len()); logged.0 = true; }
}

fn sync_runtime_settings(rt: Option<Res<RuntimeSettings>>, params: Option<ResMut<ParamsUniform>>) {
    let (Some(rt), Some(mut params)) = (rt, params) else { return; };
    let desired = if rt.clustering_enabled { 1u32 } else { 0u32 };
    if params.clustering_enabled == desired { return; }
    params.clustering_enabled = desired;
}

#[cfg(test)]
mod tests {
    use super::*; use crate::{MetaBall, MetaBallColor};
    #[test]
    fn packing_truncates_and_sets_sentinel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(BallBuffer { balls: Vec::new() });
    app.insert_resource(ParamsUniform { screen_size:[1024.0,1024.0], num_balls:0, clustering_enabled:1 });
        app.insert_resource(TimeUniform::default());
        app.init_resource::<PackLogOnce>();
        // Spawn > capacity
            for i in 0..105 { app.world_mut().spawn((MetaBall { center: Vec2::new(i as f32, i as f32), radius: 5.0 }, MetaBallColor(LinearRgba::new(1.0,1.0,1.0,1.0)))); }
        app.add_systems(Update, gather_metaballs);
        app.update();
        let params = app.world().resource::<ParamsUniform>();
            assert_eq!(params.num_balls as usize, 105, "Dynamic count should match spawned entities");
        let buffer = app.world().resource::<BallBuffer>();
            assert_eq!(buffer.balls.len(), 105);
    }
}
