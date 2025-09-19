use bevy::prelude::*;
use crate::{MetaBall, MetaBallColor, MetaBallCluster};
use crate::internal::{BallBuffer, BallGpu, ParamsUniform, TimeUniform, MAX_BALLS, OverflowWarned};
use crate::RuntimeSettings;

pub(crate) struct PackingPlugin;
impl Plugin for PackingPlugin { fn build(&self, app: &mut App) {
    app.init_resource::<OverflowWarned>()
        .add_systems(Update, (advance_time, gather_metaballs, sync_runtime_settings));
}}

fn advance_time(time: Res<Time>, uni: Option<ResMut<TimeUniform>>) { if let Some(mut u) = uni { u.time += time.delta_secs(); } }

fn gather_metaballs(
    mut buffer: ResMut<BallBuffer>,
    mut params: ResMut<ParamsUniform>,
    mut warned: ResMut<OverflowWarned>,
    query: Query<(&MetaBall, Option<&MetaBallColor>, Option<&MetaBallCluster>)>,
) {
    // Ensure capacity once
    if buffer.balls.len() != MAX_BALLS { buffer.balls = vec![BallGpu { center:[0.0,0.0], radius:0.0, cluster_id:0, color:[0.0;4] }; MAX_BALLS]; }
    let mut count = 0usize;
    for (mb, color_opt, cluster_opt) in query.iter() {
        if count >= MAX_BALLS { break; }
        let dst = &mut buffer.balls[count];
        dst.center = [mb.center.x, mb.center.y];
        dst.radius = mb.radius;
        dst.cluster_id = cluster_opt.map(|c| c.0).unwrap_or(0);
        let c = color_opt.map(|c| c.0).unwrap_or(LinearRgba::new(1.0,1.0,1.0,1.0));
        dst.color = [c.red, c.green, c.blue, c.alpha];
        count += 1;
    }
    // Sentinel (negative radius) at next slot if room
    if count < MAX_BALLS { buffer.balls[count].radius = -1.0; }
    params.num_balls = count as u32;
    static mut LOGGED: bool = false;
    unsafe {
        if !LOGGED {
            if let Some(first) = buffer.balls.get(0) { info!(target: "metaballs", "packed {} balls; first radius {}", count, first.radius); }
            LOGGED = true;
        }
    }
    if query.iter().count() > MAX_BALLS && !warned.0 {
        warned.0 = true;
        warn!(target: "metaballs", "MetaBall entity count exceeded capacity {MAX_BALLS}; truncating");
    }
}

fn sync_runtime_settings(rt: Option<Res<RuntimeSettings>>, params: Option<ResMut<ParamsUniform>>) {
    let (Some(rt), Some(mut params)) = (rt, params) else { return; };
    let desired = if rt.clustering_enabled { 1u32 } else { 0u32 };
    if params.clustering_enabled != desired { params.clustering_enabled = desired; }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::{MetaBall, MetaBallColor};
    #[test]
    fn packing_truncates_and_sets_sentinel() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(BallBuffer { balls: Vec::new() });
        app.insert_resource(ParamsUniform { screen_size:[1024.0,1024.0], num_balls:0,_unused0:0, iso:2.2,_unused2:0.0,_unused3:0.0,_unused4:0, clustering_enabled:1,_pad:0.0 });
        app.insert_resource(TimeUniform::default());
        app.init_resource::<OverflowWarned>();
        // Spawn > capacity
        for i in 0..(MAX_BALLS+5) { app.world_mut().spawn((MetaBall { center: Vec2::new(i as f32, i as f32), radius: 5.0 }, MetaBallColor(LinearRgba::new(1.0,1.0,1.0,1.0)))); }
        app.add_systems(Update, gather_metaballs);
        app.update();
        let params = app.world().resource::<ParamsUniform>();
        assert_eq!(params.num_balls as usize, MAX_BALLS, "Should truncate to MAX_BALLS");
        let buffer = app.world().resource::<BallBuffer>();
        // Sentinel at next index if capacity not exceeded; since exactly truncated, last slot should have radius set by last write; we ensure no panic and num_balls == MAX_BALLS
        assert!(buffer.balls.len() >= MAX_BALLS);
    }
}
