pub mod gpu;
pub mod material;
pub mod resources;
pub mod startup;
pub mod systems;
pub mod palette;
pub mod metaballs; // legacy plugin wrapper retained
pub mod compute_noop; // newly added no-op compute prepass

// Re-export primary types commonly used elsewhere for minimal churn
pub use gpu::{MetaballsUniform, GpuBall, NoiseParamsUniform, SurfaceNoiseParamsUniform, TileHeaderGpu, MAX_BALLS, MAX_CLUSTERS, map_signed_distance};
pub use material::MetaballsUnifiedMaterial;
pub use resources::*;
pub use metaballs::{MetaballsPlugin, MetaballsUpdateSet};
