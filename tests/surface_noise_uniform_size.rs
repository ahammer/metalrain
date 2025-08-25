use ball_matcher::rendering::metaballs::metaballs::SurfaceNoiseParamsUniform;

// Ensure future edits keep uniform buffer size 16-byte multiple (std140-style downlevel requirement)
#[test]
fn surface_noise_uniform_size_is_64_bytes() {
    assert_eq!(::std::mem::size_of::<SurfaceNoiseParamsUniform>(), 64, "SurfaceNoiseParamsUniform must remain 64 bytes (16 * 4-byte scalars) so that the WGSL uniform size is a multiple of 16 and passes wgpu validation on downlevel/WebGL backends.");
}
