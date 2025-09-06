use ball_matcher::core::level::widgets::{WidgetsFile, extract_widgets, TextColorMode};

#[test]
fn text_spawn_extraction_defaults_and_validation() {
    // RON snippet with minimal TextSpawn relying on defaults plus some invalid values to trigger warnings.
    let ron_src = r#"(
        version: 1,
        widgets: [
            (
                type: "TextSpawn",
                id: 42,
                pos: (x: 10.0, y: -5.0),
                text: "Hello  World", // double space to create empty word, will be ignored by glyph logic; still tests splitting
                font_px: 0,            // invalid -> clamped to >=1
                cell: 0.5,             // invalid -> adjusted to 8.0 with warning
                jitter: 5.0,
                radius: { min: 12.0, max: 7.0 }, // swapped
                speed: { min: 30.0, max: 10.0 }, // swapped
                attraction_strength: 55.0,
                attraction_damping: -2.0, // clamped to >=0
                snap_distance: -1.0,      // clamped to >=0
                color_mode: "UnknownMode", // fallback to RandomPerBall warning
                word_colors: [1,2,3],
            ),
        ],
    )"#;

    let wf: WidgetsFile = ron::from_str(ron_src).expect("parse RON");
    let extracted = extract_widgets(&wf);
    assert_eq!(extracted.text_spawns.len(), 1);
    let spec = &extracted.text_spawns[0];
    assert_eq!(spec.id, 42);
    assert_eq!(spec.pos.x, 10.0);
    assert_eq!(spec.font_px, 1); // clamped
    assert_eq!(spec.cell, 8.0); // adjusted
    assert!(spec.radius_min <= spec.radius_max);
    assert!(spec.speed_min <= spec.speed_max);
    assert_eq!(spec.color_mode, TextColorMode::RandomPerBall);
    assert_eq!(spec.word_palette_indices, vec![1,2,3]);
    // Ensure warnings captured (not exhaustive count to remain resilient to future changes)
    assert!(!extracted.warnings.is_empty());
}
