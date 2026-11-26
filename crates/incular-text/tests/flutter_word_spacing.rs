//! Flutter 3.50.0 word-spacing regression (C28 in the pinned parity audit).

use incular_text::{TextAlign, TextEngine, TextStyle};

fn width(engine: &mut TextEngine, style: &TextStyle) -> f32 {
    engine.layout("a a", style, None, TextAlign::Start).lines[0].width
}

#[test]
fn word_spacing_adds_once_for_one_space_even_on_a_warm_cache() {
    let plain = TextStyle::default();
    let word_spaced = plain.clone().word_spacing(Some(12.0));
    let letter_spaced = plain.clone().letter_spacing(2.0);
    let mut warm = TextEngine::new();

    let base = width(&mut warm, &plain);
    assert!(
        width(&mut warm, &letter_spaced) > base,
        "letter-spacing control"
    );

    let warm_delta = width(&mut warm, &word_spaced) - base;
    let cold_delta = width(&mut TextEngine::new(), &word_spaced) - base;
    assert!(
        (warm_delta - 12.0).abs() < 0.01 && (cold_delta - 12.0).abs() < 0.01,
        "one space should add 12 px; warm delta {warm_delta}, cold delta {cold_delta}"
    );
}

#[test]
fn changing_spacing_does_not_reuse_a_single_line_or_document_from_another_style() {
    let mut engine = TextEngine::new();
    let plain = TextStyle::default();
    let base = width(&mut engine, &plain);
    for spacing in [9., -3., 0., 17.] {
        let adjusted = plain.clone().word_spacing(Some(spacing));
        let single = width(&mut engine, &adjusted);
        assert!((single - base - spacing).abs() < 0.01);
        let layout = engine.layout("a a\na a", &adjusted, Some(240.), TextAlign::Start);
        assert_eq!(layout.lines.len(), 2);
        for line in layout.lines.iter() {
            assert!((line.width - base - spacing).abs() < 0.01);
        }
    }
    assert_eq!(width(&mut engine, &plain), base);
}
