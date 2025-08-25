use std::fs;

use ball_matcher::core::config::config::GameConfig;

#[test]
fn cluster_pop_default_impulse_positive() {
    let cfg = GameConfig::default();
    assert!(cfg.interactions.cluster_pop.impulse > 0.0, "cluster_pop.impulse should be > 0 by default");
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
                    impulse: 500.0,
                    outward_bonus: 0.5,
                    despawn_delay: 0.0,
                    aabb_pad: 4.0,
                    tap_radius: 30.0,
                    fade_enabled: true,
                    fade_duration: 1.0,
                    fade_scale_end: 0.0,
                    fade_alpha: true,
                    exclude_from_new_clusters: true,
                    collider_shrink: false,
                    collider_min_scale: 0.25,
                    velocity_damping: 0.0,
                    spin_jitter: 0.0,
                ),
            ),
        )
    "#;
    fs::write(&path, ron).expect("write temp ron");
    let (cfg, _used, errors) = GameConfig::load_layered([&path]);
    // Ensure we still deserialize a config and cluster_pop defaults are accessible.
    assert!(cfg.interactions.cluster_pop.impulse > 0.0);

    // Expect a legacy key warning mentioning at least one removed key.
    let joined = errors.join("\n");
    assert!(
        joined.contains("Ignoring legacy interactions keys removed"),
        "expected legacy key warning, got: {joined}"
    );
}
