//! Placeholder for future visual regression testing.
//! Strategy (Sprint 3+):
//! 1. Spawn deterministic set of metaballs.
//! 2. Run a few frames until GPU buffers filled.
//! 3. Read back field/albedo texture bytes (CPU readback staging buffer).
//! 4. Hash content (e.g., blake3) and compare against stored baseline hash.
//! 5. On hash mismatch, write artifact to `target/vis_diffs/` for manual inspection.
//!
//! This test is `#[ignore]` so CI can optionally enable it when GPU readback is stable.

#[test]
#[ignore]
fn metaball_visual_snapshot_todo() {
    // Intentionally empty. See doc comment above for planned implementation.
    assert!(true);
}
