use bevy::prelude::*;
use crate::components::{MetaBall, MetaBallColor, MetaBallCluster};
use crate::coordinates::MetaballCoordinateMapper;
use crate::internal::{BallBuffer, ParamsUniform, FieldTexture, AlbedoTexture, NormalTexture};

/// User‑tunable diagnostics configuration.
#[derive(Resource, Debug, Clone)]
pub struct MetaballDiagnosticsConfig {
    pub enabled: bool,
    /// How many frames between periodic logs.
    pub log_every_n_frames: u32,
    pub log_coordinates: bool,
    pub log_gpu_buffers: bool,
    pub log_textures: bool,
    /// Stop all periodic logging after this frame (inclusive). 0 = unlimited.
    pub max_frames_logging: u64,
}
impl Default for MetaballDiagnosticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_every_n_frames: 60, // ~1s at 60 FPS
            log_coordinates: true,
            log_gpu_buffers: true,
            log_textures: true,
            max_frames_logging: 2,
        }
    }
}

#[derive(Resource, Default)]
struct FrameCounter(u64);

pub struct MetaballDiagnosticsPlugin;
impl Plugin for MetaballDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MetaballDiagnosticsConfig>()
            .init_resource::<FrameCounter>()
            .add_systems(PostStartup, startup_summary)
            .add_systems(Update, (increment_frame_counter, periodic_diagnostics));
    }
}

fn increment_frame_counter(mut fc: ResMut<FrameCounter>) { fc.0 += 1; }

fn startup_summary(
    config: Res<MetaballDiagnosticsConfig>,
    settings: Option<Res<crate::settings::MetaballRenderSettings>>,
    mapper: Option<Res<MetaballCoordinateMapper>>,
) {
    if !config.enabled { return; }
    if let Some(settings) = settings {
        info!(target: "metaballs::diag", "Startup: texture_size={:?} world_bounds=({}, {}) -> ({}, {}) clustering={} ",
            settings.texture_size,
            settings.world_bounds.min.x, settings.world_bounds.min.y,
            settings.world_bounds.max.x, settings.world_bounds.max.y,
            settings.enable_clustering
        );
    } else {
        warn!(target: "metaballs::diag", "Startup: MetaballRenderSettings not found (plugin order?)");
    }
    if let Some(mapper) = mapper {
        info!(target: "metaballs::diag", "Mapper: world_min=({:.2},{:.2}) world_max=({:.2},{:.2}) tex_size={:?}", mapper.world_min.x, mapper.world_min.y, mapper.world_max.x, mapper.world_max.y, mapper.texture_size);
    }
    warn!(target: "metaballs::diag", "NOTE: Sprint 2.1 renderer supplies offscreen textures only; add a presentation/compositing pass to see metaballs on screen.");
}

fn periodic_diagnostics(
    config: Res<MetaballDiagnosticsConfig>,
    fc: Res<FrameCounter>,
    mapper: Option<Res<MetaballCoordinateMapper>>,
    params: Option<Res<ParamsUniform>>,
    buffer: Option<Res<BallBuffer>>,
    field: Option<Res<FieldTexture>>,
    albedo: Option<Res<AlbedoTexture>>,
    normals: Option<Res<NormalTexture>>,
    images: Option<Res<Assets<Image>>>,
    q_balls: Query<(Entity, &Transform, &MetaBall, Option<&MetaBallColor>, Option<&MetaBallCluster>)>,
) {
    let cfg = &*config;
    if !cfg.enabled { return; }
    if cfg.max_frames_logging > 0 && fc.0 > cfg.max_frames_logging { return; }
    if fc.0 == 1 || (cfg.log_every_n_frames > 0 && fc.0 % cfg.log_every_n_frames as u64 == 0) {
        if cfg.log_gpu_buffers {
            if let (Some(params), Some(buffer)) = (params.as_deref(), buffer.as_deref()) {
                info!(target: "metaballs::diag", "Frame {}: packed {} balls (uniform says {}) clustering={} tex={}x{}", fc.0, buffer.balls.len(), params.num_balls, params.clustering_enabled, params.screen_size[0], params.screen_size[1]);
                for (i, b) in buffer.balls.iter().take(3).enumerate() {
                    info!(target: "metaballs::diag", "  GPU[{}] center=({:.1},{:.1}) r={:.2} cluster={} color=[{:.2},{:.2},{:.2},{:.2}]", i, b.center[0], b.center[1], b.radius, b.cluster_id, b.color[0], b.color[1], b.color[2], b.color[3]);
                }
            }
        }
    if cfg.log_coordinates {
            if let Some(mapper) = mapper.as_deref() {
                for (e, tr, mb, color, cluster) in q_balls.iter().take(3) {
                    let world = tr.translation.truncate();
                    let tex = mapper.world_to_metaball(tr.translation);
                    let uv = mapper.metaball_to_uv(tex);
                    info!(target: "metaballs::diag", "  Ent {:?} world=({:.1},{:.1}) tex=({:.1},{:.1}) uv=({:.3},{:.3}) r_world={:.2} col={} cluster={} ",
                        e, world.x, world.y, tex.x, tex.y, uv.x, uv.y, mb.radius_world,
                        color.map(|c| format!("[{:.2},{:.2},{:.2},{:.2}]", c.0.red, c.0.green, c.0.blue, c.0.alpha)).unwrap_or_else(|| "default".into()),
                        cluster.map(|c| c.0).unwrap_or(0)
                    );
                }
            }
        }
    if cfg.log_textures {
            if let (Some(images), Some(field), Some(albedo)) = (images.as_ref(), field.as_ref(), albedo.as_ref()) {
                let field_info = images.get(&field.0).map(|i| format!("{}x{}", i.width(), i.height())).unwrap_or_else(|| "unloaded".into());
                let albedo_info = images.get(&albedo.0).map(|i| format!("{}x{}", i.width(), i.height())).unwrap_or_else(|| "unloaded".into());
                let normal_info = normals.and_then(|n| images.get(&n.0)).map(|i| format!("{}x{}", i.width(), i.height())).unwrap_or_else(|| "unloaded".into());
                info!(target: "metaballs::diag", "  Textures: field={field_info} albedo={albedo_info} normals={normal_info}");
            }
        }
    }
}
