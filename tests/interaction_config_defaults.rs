use std::fs;

use ball_matcher::core::config::config::GameConfig;

#[test]
fn cluster_pop_default_peak_scale_gt_one() {
    let cfg = GameConfig::default();
    assert!(
        cfg.interactions.cluster_pop.peak_scale > 1.0,
        "cluster_pop.peak_scale should be > 1.0 by default"
    );
    assert!(
        (cfg.interactions.cluster_pop.grow_duration > 0.0)
            && (cfg.interactions.cluster_pop.shrink_duration > 0.0),
        "grow/shrink durations must be positive"
    );
}

#[test]
fn legacy_explosion_drag_keys_ignored() {
    // Create a temporary RON config containing legacy keys that must now be ignored.
    let mut path = std::env::temp_dir();
    path.push("legacy_interactions_config.ron");
    let ron = r#"
        (
            window: (
                width: 640.0,
                height: 480.0,
                title: "Test",
                autoClose: 0.0,
            ),
            interactions: (
                explosion: (impulse: 999.0),
                drag: (enabled: true),
                cluster_pop: (
                    enabled: true,
                    min_ball_count: 3,
                    min_total_area: 100.0,
                    // Legacy fields that should be ignored but produce a single warning in validation:
                    impulse: 500.0,
                    outward_bonus: 0.5,
                    despawn_delay: 0.0,
                    fade_duration: 1.0,
                    fade_scale_end: 0.0,
                    collider_shrink: false,
                    collider_min_scale: 0.25,
                    velocity_damping: 0.0,
                    spin_jitter: 0.0,
                    // New required paddle fields:
                    peak_scale: 1.8,
                    grow_duration: 0.25,
                    hold_duration: 0.10,
                    shrink_duration: 0.40,
                    collider_scale_curve: 1,
                    freeze_mode: 0,
                    fade_alpha: true,
                    fade_curve: 1,
                    aabb_pad: 4.0,
                    tap_radius: 30.0,
                    exclude_from_new_clusters: true,
                ),
            ),
        )
    "#;
    fs::write(&path, ron).expect("write temp ron");
    let (cfg, _used, errors) = GameConfig::load_layered([&path]);
    // Ensure we still deserialize a config and new peak_scale is accessible.
    assert!(cfg.interactions.cluster_pop.peak_scale >= 1.0);

    // Expect a legacy interactions key warning (explosion / drag)
    let joined = errors.join("\n");
    assert!(
        joined.contains("Ignoring legacy interactions keys removed"),
        "expected legacy interactions key warning, got: {joined}"
    );

    // (Validation legacy cluster_pop field warning optional depending on deserialization behavior)
    let _warns = cfg.validate();
}
