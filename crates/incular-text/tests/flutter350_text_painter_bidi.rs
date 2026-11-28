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

#[test]
fn point_hits_follow_the_visual_neighbor_at_a_bidi_join_and_the_glyph_midpoint() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_BIDI_HIT_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    let font = "Flutter exact bidi hit testing";
    assert!(engine.register_font_bytes_as(font, std::fs::read(path).unwrap()));
    let points = [
        -100., 0., 4., 5., 8., 10., 12., 28., 29.9, 30., 30.1, 32., 35., 38., 42., 58., 59.9, 60.,
        60.1, 62., 65., 88., 90., 100.,
    ];
    for (text, direction, reference) in [
        (
            "ABCאבגDEF",
            TextDirection::Ltr,
            "0d 0d 0d 1u 1u 1d 1d 3u 3u 6u 6u 6u 5d 5d 5u 3d 3d 6d 6d 6d 7u 9u 9u 9u",
        ),
        (
            "אבגABCדהו",
            TextDirection::Rtl,
            "9u 9u 9u 8d 8d 8u 8u 6d 6d 3d 3d 3d 4u 4u 4d 6u 6u 3u 3u 3u 2d 0d 0d 0d",
        ),
    ] {
        let span = TextSpan::new(text).style(TextStyle::new().font_family(font).font_size(10.));
        let mut painter = TextPainter::with_span(span, direction);
        painter
            .layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        assert_eq!(painter.width(), Ok(90.));
        let hits: Vec<_> = points
            .iter()
            .map(|&x| {
                let pos = painter.position_for_offset(Offset::new(x, 5.)).unwrap();
                let utf16 = text[..pos.offset].encode_utf16().count();
                let affinity = if pos.affinity == TextAffinity::Upstream {
                    "u"
                } else {
                    "d"
                };
                format!("{utf16}{affinity}")
            })
            .collect();
        assert_eq!(hits.join(" "), reference, "{direction:?}: {points:?}");
    }
}
