use criterion::{criterion_group, criterion_main, Criterion};
use bevy::prelude::*;
use metaball_renderer::MetaballCoordinateMapper;

fn bench_world_to_metaball(c: &mut Criterion) {
    let mapper = MetaballCoordinateMapper::new(
        UVec2::new(1024, 1024),
        Vec2::new(-512.0, -512.0),
        Vec2::new(512.0, 512.0),
    );
    c.bench_function("world_to_metaball", |b| {
        let mut x: f32 = 0.0;
        b.iter(|| {
            x += 1.0;
            let _ = mapper.world_to_metaball(Vec3::new(x % 512.0, (x * 0.5) % 512.0, 0.0));
        });
    });
}

criterion_group!(benches, bench_world_to_metaball);
criterion_main!(benches);
