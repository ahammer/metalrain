mod pipeline;
mod pipeline_normals;
pub mod types; // public for present module

pub use pipeline::{ComputeMetaballsPlugin, GpuMetaballPipeline, MetaballPassLabel};
pub use pipeline_normals::{NormalComputePlugin, NormalsPassLabel};
