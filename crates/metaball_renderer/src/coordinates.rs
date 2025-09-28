use bevy::prelude::*;

/// Defines the mapping between authoritative world space (XY plane on Z=0) and the
/// offscreen metaball texture pixel space. World space is a continuous Rect while the
/// texture is a discrete pixel grid (0..texture_size.x, 0..texture_size.y).
#[derive(Resource, Clone, Debug)]
pub struct MetaballCoordinateMapper {
    pub texture_size: UVec2,
    pub world_min: Vec2,
    pub world_max: Vec2,
    pub world_size: Vec2,
}

impl MetaballCoordinateMapper {
    pub fn new(texture_size: UVec2, world_min: Vec2, world_max: Vec2) -> Self {
        let world_size = world_max - world_min;
        assert!(world_size.x > 0.0 && world_size.y > 0.0, "world bounds must have positive extent");
        Self { texture_size, world_min, world_max, world_size }
    }
    /// Map a world position (Vec3, using XY) to continuous texture pixel coordinates.
    pub fn world_to_metaball(&self, world: Vec3) -> Vec2 {
        let p = world.truncate();
        let norm = (p - self.world_min) / self.world_size; // 0..1 (not clamped)
        norm * self.texture_size.as_vec2()
    }
    /// Map a world radius (in world units along X) to texture pixel radius.
    pub fn world_radius_to_tex(&self, r: f32) -> f32 {
        r * (self.texture_size.x as f32) / self.world_size.x
    }
    /// Map metaball texture pixel coordinates to UV (0..1) range (continuous).
    pub fn metaball_to_uv(&self, tex: Vec2) -> Vec2 { tex / self.texture_size.as_vec2() }
    /// Clamp a world position inside the configured world bounds (helpful for physics keeping inside field).
    pub fn clamp_world(&self, mut p: Vec2) -> Vec2 {
        p.x = p.x.clamp(self.world_min.x, self.world_max.x);
        p.y = p.y.clamp(self.world_min.y, self.world_max.y);
        p
    }
}

// --- Projection helpers (simple wrappers relying on Bevy camera APIs) ---

/// Project a world position onto viewport pixel coordinates using the given camera.
pub fn project_world_to_screen(world: Vec3, camera: &Camera, cam_transform: &GlobalTransform) -> Option<Vec2> {
    camera.world_to_viewport(cam_transform, world).ok()
}

/// Unproject a viewport pixel coordinate into world space on the Z=0 plane.
pub fn screen_to_world(screen: Vec2, camera: &Camera, cam_transform: &GlobalTransform) -> Option<Vec3> {
    camera.viewport_to_world_2d(cam_transform, screen).ok().map(|v| v.extend(0.0))
}

/// Convenience: screen -> metaball UV
pub fn screen_to_metaball_uv(screen: Vec2, camera: &Camera, cam_transform: &GlobalTransform, mapper: &MetaballCoordinateMapper) -> Option<Vec2> {
    let world = screen_to_world(screen, camera, cam_transform)?;
    let tex = mapper.world_to_metaball(world);
    Some(mapper.metaball_to_uv(tex))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corners_map_correctly() {
        let mapper = MetaballCoordinateMapper::new(UVec2::new(512,512), Vec2::new(-256.0,-256.0), Vec2::new(256.0,256.0));
        assert_eq!(mapper.world_to_metaball(Vec3::new(-256.0,-256.0,0.0)), Vec2::new(0.0,0.0));
        assert_eq!(mapper.world_to_metaball(Vec3::new(256.0,-256.0,0.0)).x, 512.0);
        assert_eq!(mapper.world_to_metaball(Vec3::new(-256.0,256.0,0.0)).y, 512.0);
    }
    #[test]
    fn radius_scaling() {
        let mapper = MetaballCoordinateMapper::new(UVec2::new(1000,500), Vec2::new(0.0,0.0), Vec2::new(100.0,50.0));
        // 100 world units span -> 1000 tex pixels => 1 world unit == 10 px
        assert_eq!(mapper.world_radius_to_tex(1.0), 10.0);
    }
    #[test]
    fn uv_in_range() {
        let mapper = MetaballCoordinateMapper::new(UVec2::new(400,200), Vec2::new(-2.0,-1.0), Vec2::new(2.0,1.0));
        for i in 0..10 {
            let x = -2.0 + 0.4 * i as f32;
            for j in 0..10 { let y = -1.0 + 0.2 * j as f32; let uv = mapper.metaball_to_uv(mapper.world_to_metaball(Vec3::new(x,y,0.0))); assert!(uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0, "uv out of range: {uv:?}"); }
        }
    }
}
