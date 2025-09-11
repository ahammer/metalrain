// ============================================================================
// Metaballs Precompute No-Op Pass
// Dispatched before metaball rendering. Future use: field reductions, SDF normal prep.
// ============================================================================
@compute @workgroup_size(1)
fn cs_main() { /* intentionally empty */ }
