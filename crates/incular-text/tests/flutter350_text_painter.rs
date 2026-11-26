use incular_config::TextDirection;
use incular_core::Size;
use incular_text::{
    InlineSpan, TextAlign, TextEngine, TextPainter, TextPainterError, TextRange, TextSpan,
    TextStyle, TextWidthBasis, WidgetSpan,
};
use std::sync::Arc;

fn painter(span: TextSpan, direction: TextDirection) -> TextPainter {
    let mut painter = TextPainter::new();
    painter.set_text(Some(span));
    painter.set_text_direction(Some(direction));
    painter
}

#[test]
fn layout_requires_explicit_text_direction_even_for_empty_text() {
    let mut missing = TextPainter::new();
    missing.set_text(Some(TextSpan::empty()));
    assert_eq!(
        missing.layout(0., f32::INFINITY),
        Err(TextPainterError::MissingTextDirection)
    );
    assert_eq!(missing.size(), Err(TextPainterError::NeedsLayout));
    missing.set_text_direction(Some(TextDirection::Rtl));
    missing.layout(0., f32::INFINITY).unwrap();
    assert_eq!(missing.size().unwrap().width, 0.);
}

#[test]
fn disposal_releases_a_measurement_and_cannot_be_repeated() {
    let mut painter = painter(TextSpan::new("text"), TextDirection::Ltr);
    assert!(!painter.debug_disposed());
    painter.layout(0., f32::INFINITY).unwrap();
    painter.dispose().unwrap();
    assert!(painter.debug_disposed());
    assert_eq!(painter.size(), Err(TextPainterError::Disposed));
    assert_eq!(painter.dispose(), Err(TextPainterError::Disposed));
}

#[test]
fn plain_text_keeps_inline_placeholders_before_and_after_layout() {
    fn mixed(before: &str, after: &str) -> TextSpan {
        TextSpan::empty().children([
            InlineSpan::from(TextSpan::new(before)),
            InlineSpan::from(WidgetSpan::sized(1, Size::new(50., 30.))),
            InlineSpan::from(TextSpan::new(after)),
        ])
    }

    let mut painter = TextPainter::new();
    painter.set_text_direction(Some(TextDirection::Ltr));
    assert_eq!(painter.plain_text(), "");
    for (before, after, flattened) in [
        ("before\n", "after", "before\n\u{fffc}after"),
        ("be\nfo\nre\n", "af\nter", "be\nfo\nre\n\u{fffc}af\nter"),
    ] {
        painter.set_text(Some(mixed(before, after)));
        assert_eq!(painter.plain_text(), flattened);
        assert_eq!(painter.size(), Err(TextPainterError::NeedsLayout));
        painter.layout(0., f32::INFINITY).unwrap();
        assert_eq!(painter.plain_text(), flattened);
        assert_eq!(painter.shaped_layout().unwrap().placeholders.len(), 1);
    }
}

#[test]
fn consecutive_family_emoji_are_independent_word_boundaries() {
    let emoji = "👨‍👩‍👦";
    let text = emoji.repeat(3);
    let mut painter = painter(TextSpan::new(&text), TextDirection::Ltr);
    painter.layout(0., f32::INFINITY).unwrap();
    let range = painter.word_boundary(emoji.len()).unwrap();
    assert_eq!((range.start, range.end), (emoji.len(), emoji.len() * 2));
    assert_eq!(text[..range.start].encode_utf16().count(), 8);
    assert_eq!(text[..range.end].encode_utf16().count(), 16);
    assert_eq!(
        painter.word_boundary(text.len()).unwrap(),
        TextRange::collapsed(text.len())
    );
}

fn inter() -> (TextEngine, TextStyle) {
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as(
        "Incular Painter Test",
        include_bytes!("../../incular-web/fonts/Inter-Regular.ttf").to_vec(),
    ));
    let style = TextStyle::default()
        .font_family("Incular Painter Test")
        .font_size(10.);
    (engine, style)
}

#[test]
fn longest_line_geometry_distinguishes_width_and_alignment_offsets() {
    let (mut engine, style) = inter();
    let mut p = painter(
        TextSpan::new("aaaaaa\naaaaaa").style(style),
        TextDirection::Ltr,
    );
    p.set_text_align(TextAlign::Center);
    p.set_text_width_basis(TextWidthBasis::LongestLine);
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    let metrics = p.line_metrics().unwrap();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].width, p.size().unwrap().width);
    assert!(p.paint_offset().unwrap().x < 0.);
    assert!(
        metrics.iter().all(|line| line.left.abs() < 0.0001),
        "{metrics:?}"
    );
    assert!(Arc::ptr_eq(metrics, p.line_metrics().unwrap()));
    let previously = metrics.clone();
    let requests = engine.diagnostics().layouts_requested;
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    assert_eq!(engine.diagnostics().layouts_requested, requests);
    assert!(Arc::ptr_eq(&previously, p.line_metrics().unwrap()));
    let (mut separate, _) = inter();
    p.layout_with_engine(&mut separate, 0., 100.).unwrap();
    assert!(!Arc::ptr_eq(&previously, p.line_metrics().unwrap()));
}

#[test]
fn reflow_replaces_cached_metrics_instead_of_leaving_the_old_lines() {
    let (mut engine, style) = inter();
    let mut p = painter(
        TextSpan::new("word1 word2 word3").style(style),
        TextDirection::Ltr,
    );
    for align in [
        TextAlign::Start,
        TextAlign::End,
        TextAlign::Center,
        TextAlign::Justify,
        TextAlign::Left,
        TextAlign::Right,
    ] {
        p.set_text_align(align);
        p.set_text_width_basis(TextWidthBasis::LongestLine);
        p.layout_with_engine(&mut engine, 0., 44.).unwrap();
        let wrapped = p.line_metrics().unwrap().clone();
        assert!(wrapped.len() > 1);
        assert!(Arc::ptr_eq(&wrapped, p.line_metrics().unwrap()));
        p.layout_with_engine(&mut engine, 0., 1_000.).unwrap();
        assert_eq!(p.line_metrics().unwrap().len(), 1);
        assert!(!Arc::ptr_eq(&wrapped, p.line_metrics().unwrap()));
    }
}

#[test]
fn flutter_ahem_line_metrics_when_the_pinned_font_is_available() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let font = std::fs::read(path).expect("pinned Ahem font");
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Pinned Ahem", font));
    let make = |text: &str, size: f32| {
        TextSpan::new(text).style(
            TextStyle::default()
                .font_family("Pinned Ahem")
                .font_size(size),
        )
    };
    let mut p = painter(make("aaaaaa\naaaaaa", 10.), TextDirection::Ltr);
    p.set_text_align(TextAlign::Center);
    p.set_text_width_basis(TextWidthBasis::LongestLine);
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    assert_eq!(p.size().unwrap().width, 60.);
    assert!(p.line_metrics().unwrap().iter().all(|line| line.left == 0.));

    p.set_text(Some(make("word1 word2 word3", 14.)));
    p.layout_with_engine(&mut engine, 0., 80.).unwrap();
    assert_eq!(p.line_metrics().unwrap().len(), 3);
    p.layout_with_engine(&mut engine, 0., 1000.).unwrap();
    assert_eq!(p.line_metrics().unwrap().len(), 1);

    p.set_text(Some(make("word1 word2 word3", 10.)));
    for align in [
        TextAlign::Start,
        TextAlign::End,
        TextAlign::Center,
        TextAlign::Justify,
        TextAlign::Left,
        TextAlign::Right,
    ] {
        p.set_text_align(align);
        p.layout_with_engine(&mut engine, 0., 130.).unwrap();
        let lines = p.line_metrics().unwrap().clone();
        assert_eq!(lines.len(), 2);
        assert!(Arc::ptr_eq(&lines, p.line_metrics().unwrap()));
        p.layout_with_engine(&mut engine, 0., 1000.).unwrap();
        assert_eq!(p.line_metrics().unwrap().len(), 1);
    }

    p.set_text_align(TextAlign::Justify);
    p.set_text(Some(make(&"A".repeat(100), 10.)));
    for (max, expected) in [(1000., 1000.), (100., 100.), (1000., 1000.)] {
        p.layout_with_engine(&mut engine, 0., max).unwrap();
        assert_eq!(p.size().unwrap().width, expected);
    }

    p.set_text(Some(make("12345", 1.25)));
    p.layout_with_engine(&mut engine, 0., 6.25).unwrap();
    assert_eq!(p.max_intrinsic_width().unwrap(), 6.25);
    assert_eq!(p.line_metrics().unwrap().len(), 1);
    assert_eq!(p.line_metrics().unwrap()[0].width, 6.25);
}

#[test]
fn the_same_line_ltr_caret_has_the_same_offset_for_both_affinities() {
    use incular_text::TextAffinity::{Downstream, Upstream};
    let (mut engine, style) = inter();
    let mut p = painter(TextSpan::new("aa").style(style), TextDirection::Ltr);
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(
        p.caret_offset(1, Downstream, 5.),
        p.caret_offset(1, Upstream, 5.)
    );
}

#[test]
fn decomposed_korean_remains_queryable_at_a_degenerate_wrap_width() {
    use incular_text::TextAffinity::Downstream;
    let mut p = painter(
        TextSpan::new("각").style(TextStyle::default().font_size(10.)),
        TextDirection::Ltr,
    );
    p.layout(0., 1.).unwrap();
    let caret = p.caret_offset(0, Downstream, 0.).unwrap();
    assert!(caret.x.is_finite() && caret.y.is_finite());
}

#[test]
fn hard_line_breaks_do_not_use_affinity_but_soft_wraps_do() {
    use incular_core::Offset;
    use incular_text::TextAffinity::{Downstream, Upstream};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Ahem Caret", std::fs::read(path).unwrap()));
    let make = |text: &str| {
        TextSpan::new(text).style(
            TextStyle::default()
                .font_family("Ahem Caret")
                .font_size(14.),
        )
    };
    let mut p = painter(make("aaa"), TextDirection::Ltr);
    for (source, samples) in [
        (
            "aaa",
            &[(0, 0., 0.), (1, 14., 0.), (2, 28., 0.), (3, 42., 0.)][..],
        ),
        ("\n\n", &[(0, 0., 0.), (1, 0., 14.), (2, 0., 28.)][..]),
        ("\naaa", &[(0, 0., 0.), (1, 0., 14.)][..]),
        ("aaa\n", &[(3, 42., 0.), (4, 0., 14.)][..]),
        ("aaa\naaa", &[(4, 0., 14.)][..]),
        (
            "aaa\n\n\n",
            &[(3, 42., 0.), (4, 0., 14.), (5, 0., 28.), (6, 0., 42.)][..],
        ),
        (
            "\n\n\naaa",
            &[(0, 0., 0.), (1, 0., 14.), (2, 0., 28.), (3, 0., 42.)][..],
        ),
    ] {
        p.set_text(Some(make(source)));
        p.layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        for &(byte, x, y) in samples {
            for affinity in [Downstream, Upstream] {
                assert_eq!(
                    p.caret_offset(byte, affinity, 0.).unwrap(),
                    Offset::new(x, y),
                    "{source:?} at {byte} {affinity:?}"
                );
            }
        }
    }
    p.set_text(Some(make("aaaaaaaa")));
    p.layout_with_engine(&mut engine, 0., 100.).unwrap();
    assert_eq!(
        p.caret_offset(7, Downstream, 0.).unwrap(),
        Offset::new(0., 14.)
    );
    assert_eq!(
        p.caret_offset(7, Upstream, 0.).unwrap(),
        Offset::new(98., 0.)
    );

    p.set_text_align(TextAlign::Right);
    p.set_text(Some(make("aaa")));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.caret_offset(0, Downstream, 0.).unwrap(), Offset::ZERO);
}

#[test]
fn grapheme_internal_utf16_positions_share_a_real_cluster_end() {
    use incular_text::TextAffinity::{Downstream, Upstream};
    let (mut engine, style) = inter();
    for emoji in [
        "👩‍🚀",
        "👩‍❤️‍💋‍👩",
        "👨‍👩‍👦‍👦",
        "👨🏾‍🤝‍👨🏻",
        "👨‍👦",
        "👩‍👦",
        "🏌🏿‍♀️",
        "🏊‍♀️",
        "🏄🏻‍♂️",
        "🇺🇳",
        "👩‍❤️‍👨",
    ] {
        for prefix in ["", "a", "ab", "abc", "abcd"] {
            let source = format!("{prefix}{emoji}");
            let n = source.encode_utf16().count();
            let mut p = painter(
                TextSpan::new(source).style(style.clone()),
                TextDirection::Ltr,
            );
            p.layout_with_engine(&mut engine, 0., f32::INFINITY)
                .unwrap();
            let end = p.caret_offset_utf16(n, Downstream, 0.).unwrap();
            let start = p.caret_offset_utf16(prefix.len(), Downstream, 0.).unwrap();
            let mut first = 0.;
            for byte in 0..=prefix.len() {
                let mut before = painter(
                    TextSpan::new(&prefix[..byte]).style(style.clone()),
                    TextDirection::Ltr,
                );
                before
                    .layout_with_engine(&mut engine, 0., f32::INFINITY)
                    .unwrap();
                let x = p.caret_offset_utf16(byte, Downstream, 0.).unwrap().x;
                assert!(
                    (x - before.size().unwrap().width).abs() < 0.001,
                    "{prefix}{emoji} at {byte}"
                );
                assert!(x >= first - 0.001);
                first = x;
            }
            assert!((end.x - p.size().unwrap().width).abs() < 0.001);
            assert!(
                end.x > start.x,
                "{prefix}{emoji} must give the cluster some advance"
            );
            for affinity in [Downstream, Upstream] {
                for index in prefix.len() + 1..n {
                    assert_eq!(
                        p.caret_offset_utf16(index, affinity, 0.).unwrap(),
                        end,
                        "{prefix}{emoji} at {index} {affinity:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_span_with_only_children_can_be_measured_and_then_cleared() {
    use incular_core::Offset;
    use incular_text::TextAffinity::Downstream;
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Ahem Caret", std::fs::read(path).unwrap()));
    let root = |children: Vec<TextSpan>| {
        TextSpan::empty()
            .style(
                TextStyle::default()
                    .font_family("Ahem Caret")
                    .font_size(14.),
            )
            .children(children)
    };
    let mut p = painter(
        root(vec![TextSpan::new("B"), TextSpan::new("C")]),
        TextDirection::Ltr,
    );
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    let width = p.size().unwrap().width;
    assert_eq!(p.caret_offset(0, Downstream, 0.).unwrap().x, 0.);
    assert_eq!(p.caret_offset(1, Downstream, 0.).unwrap().x, width / 2.);
    assert_eq!(p.caret_offset(2, Downstream, 0.).unwrap().x, width);
    p.set_text(Some(root(Vec::new())));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    for offset in 0..=1 {
        assert_eq!(
            p.caret_offset(offset, Downstream, 0.).unwrap(),
            Offset::ZERO
        );
    }
}

#[test]
fn bidi_run_leading_edge_accounts_for_caret_width_without_moving_terminal_caret() {
    use incular_text::TextAffinity::{Downstream, Upstream};
    let (mut engine, style) = inter();
    let mut p = painter(TextSpan::new("aل").style(style), TextDirection::Ltr);
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    let visual_end = p.line_metrics().unwrap()[0].width;
    assert_eq!(p.caret_offset_utf16(0, Downstream, 5.).unwrap().x, 0.);
    let at_run_boundary = p.caret_offset_utf16(1, Downstream, 5.).unwrap().x;
    assert!((at_run_boundary - (visual_end - 5.)).abs() < 0.001);
    assert!(p.caret_offset_utf16(1, Upstream, 5.).unwrap().x < visual_end);
    assert_eq!(
        p.caret_offset_utf16(2, Downstream, 5.).unwrap().x,
        visual_end
    );
}

#[test]
fn ahem_sizes_and_scaler_follow_the_root_or_inherited_style() {
    use incular_text::TextScaler;
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Ahem Painter", std::fs::read(path).unwrap()));
    let default = TextStyle::default().font_family("Ahem Painter");
    let mut p = painter(TextSpan::new("x"), TextDirection::Ltr);
    p.set_default_style(default.clone());
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.preferred_line_height(), Ok(14.));
    assert_eq!(p.size(), Ok(Size::new(14., 14.)));

    p.set_text_scaler(TextScaler::linear(2.));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size(), Ok(Size::new(28., 28.)));

    p.set_text(Some(
        TextSpan::new("X").style(default.clone().font_size(10.)),
    ));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size(), Ok(Size::new(20., 20.)));

    p.set_text_scaler(TextScaler::NO_SCALING);
    p.set_text(Some(TextSpan::new("X").style(default.font_size(123.))));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size(), Ok(Size::new(123., 123.)));
    p.set_text(Some(
        TextSpan::new("x").style(
            TextStyle::default()
                .font_family("Ahem Painter")
                .font_size(100.),
        ),
    ));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.preferred_line_height(), Ok(100.));
    assert_eq!(p.size(), Ok(Size::new(100., 100.)));
}

#[test]
fn adjacent_run_heights_depend_on_affinity_and_terminal_caret_uses_the_template() {
    use incular_text::TextAffinity::{Downstream, Upstream};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Ahem Height", std::fs::read(path).unwrap()));
    let span = |size: f32| {
        TextSpan::new("M").style(
            TextStyle::default()
                .font_family("Ahem Height")
                .font_size(size),
        )
    };
    let mut p = painter(
        span(128.).children([span(32.), span(64.)]),
        TextDirection::Ltr,
    );
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    for (byte, upstream, downstream) in [
        (0, 128., 128.),
        (1, 128., 32.),
        (2, 32., 64.),
        (3, 64., 128.),
    ] {
        assert_eq!(p.full_height_for_caret(byte, Upstream), Ok(upstream));
        assert_eq!(p.full_height_for_caret(byte, Downstream), Ok(downstream));
        // Alternate affinity at the same position so a cache cannot reuse
        // the other run's measurement due to a byte-only cache key.
        assert_eq!(p.full_height_for_caret(byte, Upstream), Ok(upstream));
    }
}

#[test]
fn strut_sets_caret_height_even_when_the_glyph_uses_a_smaller_font() {
    use incular_text::{StrutStyle, TextAffinity::Downstream};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Ahem Strut", std::fs::read(path).unwrap()));
    let mut p = painter(
        TextSpan::new("A").style(
            TextStyle::default()
                .font_family("Ahem Strut")
                .height(Some(1.)),
        ),
        TextDirection::Ltr,
    );
    p.set_strut_style(Some(
        StrutStyle::new().font_family("Ahem Strut").font_size(50.),
    ));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.full_height_for_caret(0, Downstream), Ok(50.));
}

#[test]
fn empty_zero_size_text_returns_the_preferred_height() {
    use incular_text::TextAffinity::Downstream;
    let mut p = painter(
        TextSpan::empty().style(TextStyle::default().font_size(0.).height(Some(1.))),
        TextDirection::Ltr,
    );
    p.layout(0., f32::INFINITY).unwrap();
    assert_eq!(
        p.full_height_for_caret(0, Downstream),
        p.preferred_line_height()
    );
    assert!(p.preferred_line_height().unwrap().is_finite());
}

#[test]
fn static_measurement_matches_the_live_width_and_unconstrained_intrinsic() {
    let span = TextSpan::new("foobar");
    let mut p = TextPainter::with_span(span.clone(), TextDirection::Ltr);
    p.layout(0., f32::INFINITY).unwrap();
    let intrinsic = p.max_intrinsic_width().unwrap();
    for minimum in [0., 500.] {
        p.layout(minimum, f32::INFINITY).unwrap();
        assert_eq!(
            p.size().unwrap().width,
            TextPainter::compute_width(span.clone(), TextDirection::Ltr, minimum, f32::INFINITY)
                .unwrap()
        );
        assert_eq!(
            p.max_intrinsic_width(),
            TextPainter::compute_max_intrinsic_width(
                span.clone(),
                TextDirection::Ltr,
                minimum,
                f32::INFINITY
            )
        );
        assert_eq!(p.max_intrinsic_width(), Ok(intrinsic));
    }
}

#[test]
fn flutter_test_font_reports_exact_line_metrics_across_hard_and_soft_breaks() {
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let span = TextSpan::new("test1\nhello line two really long for soft break\nfinal line 4");
    let mut p = painter(span.clone(), TextDirection::Ltr);
    p.set_default_style(TextStyle::default().font_family("FlutterTest"));
    p.layout_with_engine(&mut engine, 0., 300.).unwrap();
    assert_eq!(p.text(), Some(&span));
    assert_eq!(p.preferred_line_height(), Ok(14.));
    let lines = p.line_metrics().unwrap();
    assert_eq!(lines.len(), 4);
    for (i, (&width, &hard_break)) in [70., 294., 266., 168.]
        .iter()
        .zip(&[true, false, true, true])
        .enumerate()
    {
        let line = lines[i];
        assert_eq!(line.line_number, i);
        assert_eq!(line.left, 0.);
        assert_eq!(line.top, 14. * i as f32);
        assert_eq!(line.baseline, 10.5 + 14. * i as f32);
        assert_eq!(line.ascent, 10.5);
        assert_eq!(line.descent, 3.5);
        assert_eq!(line.unscaled_ascent, 10.5);
        assert_eq!(line.height, 14.);
        assert_eq!(line.width, width);
        assert_eq!(line.hard_break, hard_break);
    }
}

#[test]
fn text_height_behavior_can_be_cleared_after_layout() {
    use incular_text::{TextHeightBehavior, TextLeadingDistribution};
    let mut p = painter(
        TextSpan::new("X").style(TextStyle::default().line_height_multiplier(3.)),
        TextDirection::Ltr,
    );
    p.set_text_height_behavior(Some(TextHeightBehavior::default()));
    p.layout(0., f32::INFINITY).unwrap();
    let original = p.size().unwrap();
    p.set_text_height_behavior(Some(TextHeightBehavior::new(
        false,
        false,
        TextLeadingDistribution::Proportional,
    )));
    assert_eq!(p.size(), Err(TextPainterError::NeedsLayout));
    p.layout(0., f32::INFINITY).unwrap();
    assert!(p.size().unwrap().height < original.height);
    p.set_text_height_behavior(None);
    p.layout(0., f32::INFINITY).unwrap();
    assert_eq!(p.size(), Ok(original));
}

#[test]
fn clipping_mixed_line_heights_keeps_the_retained_lines_actual_height() {
    use incular_text::{TextLayoutOptions, TextOverflow};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_AHEM_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Clip Ahem", std::fs::read(path).unwrap()));
    let style = TextStyle::default().font_family("Clip Ahem").font_size(10.);
    let span = TextSpan::new("small\n").children([
        TextSpan::new("big")
            .style(style.clone().font_size(30.))
            .into(),
        TextSpan::new("\nsmall"),
    ]);
    let shape = engine.layout_rich_with_options(
        &span.plain_text(),
        &span.flatten(&style),
        &[],
        TextLayoutOptions::new(None, TextAlign::Start)
            .max_lines(Some(2))
            .overflow(TextOverflow::Clip),
    );
    assert!(shape.overflowed);
    assert_eq!(shape.lines.len(), 2);
    assert_eq!(shape.lines[0].height, 10.);
    assert_eq!(shape.lines[1].height, 30.);
    assert_eq!(shape.metrics.size.height, 40.);
}
