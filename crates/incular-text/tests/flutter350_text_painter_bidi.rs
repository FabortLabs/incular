use incular_config::TextDirection;
use incular_core::{Offset, Rect, Size};
use incular_text::{
    TextAffinity, TextEngine, TextPainter, TextPainterBox, TextRange, TextSpan, TextStyle,
};

#[test]
fn ordinary_words_keep_their_boundaries_on_both_visual_lines() {
    let mut painter = TextPainter::with_span(TextSpan::new("ABC DEF\nGHI"), TextDirection::Ltr);
    painter.layout(0., f32::INFINITY).unwrap();
    for (offset, start, end) in [(1, 0, 3), (5, 4, 7), (9, 8, 11)] {
        assert_eq!(
            painter.word_boundary(offset).unwrap(),
            TextRange::new(start, end)
        );
    }
}

#[test]
fn forced_wrap_between_ltr_and_rtl_preserves_distinct_caret_affinities() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_EMOJI_CARET_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Flutter Bidi", std::fs::read(path).unwrap()));
    let style = TextStyle::default()
        .font_family("Flutter Bidi")
        .font_size(10.);
    let source = "Aא";
    let mut painter =
        TextPainter::with_span(TextSpan::new(source).style(style), TextDirection::Ltr);
    assert_eq!(source.encode_utf16().count(), 2);
    painter.layout_with_engine(&mut engine, 0., 10.).unwrap();
    assert_eq!(
        painter.word_boundary(0).unwrap(),
        TextRange::new(0, source.len())
    );
    for (offset, affinity, x, y) in [
        (0, TextAffinity::Upstream, 0., 0.),
        (0, TextAffinity::Downstream, 0., 0.),
        (1, TextAffinity::Upstream, 10., 0.),
        (1, TextAffinity::Downstream, 10., 10.),
        (2, TextAffinity::Upstream, 0., 10.),
        (2, TextAffinity::Downstream, 10., 10.),
    ] {
        assert_eq!(
            painter.caret_offset_utf16(offset, affinity, 0.).unwrap(),
            Offset::new(x, y),
            "offset {offset} {affinity:?}"
        );
    }
    let boxes = [
        TextPainterBox {
            rect: Rect::from_origin_size(Offset::ZERO, Size::new(10., 10.)),
            direction: TextDirection::Ltr,
        },
        TextPainterBox {
            rect: Rect::from_origin_size(Offset::new(0., 10.), Size::new(10., 10.)),
            direction: TextDirection::Rtl,
        },
    ];
    for (start, end, expected) in [
        (0, source.len(), &boxes[..]),
        (0, 1, &boxes[..1]),
        (1, source.len(), &boxes[1..]),
    ] {
        assert_eq!(
            painter
                .tight_boxes_for_selection(TextRange::new(start, end))
                .unwrap(),
            expected
        );
    }
}

#[test]
fn even_an_empty_paragraph_has_the_root_fonts_alphabetic_baseline() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Flutter Empty Baseline", std::fs::read(path).unwrap()));
    let style = TextStyle::default()
        .font_family("Flutter Empty Baseline")
        .font_size(100.)
        .height(Some(1.));
    let mut painter = TextPainter::with_span(TextSpan::empty().style(style), TextDirection::Ltr);
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(painter.alphabetic_baseline(), Ok(75.));
}
