use incular_config::TextDirection;
use incular_text::{TextAlign, TextEngine, TextLayoutOptions, TextOverflow, TextStyle};

#[test]
fn unlimited_ellipsis_keeps_hard_breaks_but_cuts_the_first_overwide_hard_line() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const FONT: &str = "Pinned unlimited ellipsis Ahem";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let style = TextStyle::new()
        .font_family(FONT)
        .font_size(10.)
        .height(Some(1.));
    let mut engine = TextEngine::new();
    let options = TextLayoutOptions::new(Some(45.), TextAlign::Start)
        .text_direction(TextDirection::Ltr)
        .overflow(TextOverflow::Ellipsis);

    let fitting = engine.layout_with_options("one\ntwo\nthin", &style, options);
    assert_eq!(fitting.lines.len(), 3);
    assert!(!fitting.overflowed);
    assert_eq!(fitting.retained_source_end(), "one\ntwo\nthin".len());

    let long = engine.layout_with_options("one\nthin enormousword after", &style, options);
    assert_eq!(long.lines.len(), 2);
    assert!(long.overflowed);
    assert_eq!(long.retained_source_end(), 7);

    let words = engine.layout_with_options("one two\nthin", &style, options);
    assert_eq!(words.lines.len(), 1);
    assert!(words.overflowed);
    assert_eq!(words.retained_source_end(), 3);
}

#[test]
fn a_mark_that_does_not_fit_leaves_no_glyph_in_either_ellipsis_mode() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const FONT: &str = "Pinned narrow ellipsis Ahem";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let style = TextStyle::new().font_family(FONT).font_size(10.);
    let mut engine = TextEngine::new();
    for cap in [None, Some(1)] {
        let options = TextLayoutOptions::new(Some(3.), TextAlign::Start)
            .max_lines(cap)
            .overflow(TextOverflow::Ellipsis);
        let layout = engine.layout_with_options("text", &style, options);
        assert_eq!(layout.lines.len(), 1);
        assert!(layout.overflowed);
        assert_eq!(layout.retained_source_end(), 0);
        assert_eq!(layout.glyph_count(), 0);
        assert_eq!(layout.lines[0].width, 0.);
    }
}

#[test]
fn a_long_source_line_shapes_a_bounded_number_of_candidate_prefixes() {
    let Some(path) = std::env::var_os("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    const FONT: &str = "Pinned long ellipsis Ahem";
    incular_text::register_application_font_as(FONT, std::fs::read(path).unwrap());
    let style = TextStyle::new().font_family(FONT).font_size(10.);
    let mut engine = TextEngine::new();
    let options = TextLayoutOptions::new(Some(45.), TextAlign::Start)
        .text_direction(TextDirection::Ltr)
        .overflow(TextOverflow::Ellipsis);
    let layout = engine.layout_with_options(&"W".repeat(4096), &style, options);
    assert_eq!(layout.lines.len(), 1);
    assert!(layout.overflowed);
    assert_eq!(layout.retained_source_end(), 3);
    assert!(engine.diagnostics().ellipsis_candidates_shaped <= 4);
}
