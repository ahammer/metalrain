mod pipeline;
mod pipeline_normals;
pub mod types; // public for present module

// Re-export only the plugin & pass labels currently required externally.
pub use pipeline::{ComputeMetaballsPlugin, MetaballPassLabel};
pub use pipeline_normals::NormalComputePlugin;
