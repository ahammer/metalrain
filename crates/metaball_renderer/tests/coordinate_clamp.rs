use bevy::prelude::*;
use metaball_renderer::MetaballCoordinateMapper;

#[test]
fn clamp_world_edges() {
    let mapper = MetaballCoordinateMapper::new(UVec2::new(400, 300), Vec2::new(-100.0, -50.0), Vec2::new(100.0, 50.0));
    // Points beyond bounds should clamp to edges
    let p = mapper.clamp_world(Vec2::new(150.0, 80.0));
    assert_eq!(p, Vec2::new(100.0, 50.0));
    let p2 = mapper.clamp_world(Vec2::new(-150.0, -80.0));
    assert_eq!(p2, Vec2::new(-100.0, -50.0));
}
