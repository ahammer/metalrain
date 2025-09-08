//! Tests for adaptive vs legacy mask monotonicity and distance proxy mapping.
//! These are CPU-side reimplementations of small pure helper logic from WGSL shader
//! to provide minimal regression coverage without altering shader layout.

fn compute_adaptive_mask(field: f32, iso: f32, grad_len: f32) -> f32 {
    let grad_l = grad_len.max(1e-5);
    let aa = (iso / grad_l * 0.5).clamp(0.75, 4.0);
    ((field - (iso - aa)) / (2.0 * aa)).clamp(0.0, 1.0) // approximate smoothstep for monotonicity check
}

fn compute_legacy_mask(field: f32, iso: f32) -> f32 { ((field - iso * 0.6) / (iso * 0.4)).clamp(0.0, 1.0) }

fn map_signed_distance(signed_d: f32, d_scale: f32) -> f32 { (0.5 - 0.5 * signed_d / d_scale).clamp(0.0, 1.0) }

#[test]
fn legacy_mask_monotonic() {
    let iso = 1.0;
    let mut last = 0.0;
    for step in 0..50 { // sample field values across reasonable range
        let f = step as f32 / 40.0 * iso * 1.4; // go a bit beyond iso
        let m = compute_legacy_mask(f, iso);
        assert!(m + 1e-6 >= last, "legacy mask not monotonic: f={f} m={m} last={last}");
        last = m;
    }
}

#[test]
fn adaptive_mask_monotonic_with_grad() {
    let iso = 1.0;
    for g in [0.25, 0.5, 1.0, 2.0] { // different gradient magnitudes
        let mut last = 0.0;
        for step in 0..50 {
            let f = step as f32 / 40.0 * iso * 1.4;
            let m = compute_adaptive_mask(f, iso, g);
            assert!(m + 1e-6 >= last, "adaptive mask not monotonic: grad={g} f={f} m={m} last={last}");
            last = m;
        }
    }
}

#[test]
fn distance_mapping_properties() {
    let scale = 8.0;
    // Negative (inside) -> >0.5
    assert!(map_signed_distance(-2.0, scale) > 0.5);
    // Zero -> 0.5
    assert!((map_signed_distance(0.0, scale) - 0.5).abs() < 1e-6);
    // Positive -> <0.5
    assert!(map_signed_distance(2.0, scale) < 0.5);
    // Far positive clamps to 0
    assert!((map_signed_distance(1e6, scale)).abs() < 1e-6);
}
