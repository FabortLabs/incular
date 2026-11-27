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

#[test]
fn strut_leading_expands_an_empty_unstyled_span_evenly() {
    use incular_text::StrutStyle;
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let mut p = painter(TextSpan::empty(), TextDirection::Ltr);
    p.set_default_style(TextStyle::default().font_family("FlutterTest"));
    let strut = StrutStyle::new().height(10.).font_size(10.);
    p.set_strut_style(Some(strut.clone()));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size(), Ok(Size::new(0., 100.)));
    assert_eq!(p.preferred_line_height(), Ok(100.));
    assert_eq!(p.alphabetic_baseline(), Ok(75.));

    p.set_strut_style(Some(strut.leading(2.)));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size(), Ok(Size::new(0., 120.)));
    assert_eq!(p.preferred_line_height(), Ok(120.));
    assert_eq!(p.alphabetic_baseline(), Ok(85.));
}

#[test]
fn forced_strut_keeps_the_line_height_when_a_widget_is_much_taller() {
    use incular_text::{StrutStyle, WidgetSpanAlignment};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let widget =
        WidgetSpan::sized(1, Size::new(1000., 1000.)).alignment(WidgetSpanAlignment::TextBottom);
    let mut p = painter(TextSpan::empty().children([widget]), TextDirection::Ltr);
    p.set_default_style(TextStyle::default().font_family("FlutterTest"));
    p.set_strut_style(Some(
        StrutStyle::new()
            .height(10.)
            .font_size(10.)
            .force_strut_height(true),
    ));
    p.layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(p.size().unwrap().height, 100.);
    let placement = &p.shaped_layout().unwrap().placeholders[0];
    assert_eq!(placement.rect.size, Size::new(1000., 1000.));
}

#[test]
fn max_lines_hides_text_without_moving_the_caret_into_an_unlaid_line() {
    use incular_core::Offset;
    use incular_text::TextAffinity::{Downstream, Upstream};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let style = TextStyle::default()
        .font_family("FlutterTest")
        .font_size(10.);
    for direction in [TextDirection::Ltr, TextDirection::Rtl] {
        let rtl = direction == TextDirection::Rtl;
        for (text, max, width, intrinsic, exceeded, hard_break, first_hidden, x, y) in [
            (
                "abcd efgh ijkl",
                1,
                40.,
                140.,
                true,
                false,
                6,
                if rtl { 0. } else { 40. },
                0.,
            ),
            (
                "ab\ncd\nefgh",
                2,
                100.,
                40.,
                true,
                true,
                6,
                if rtl { 18. } else { 20. },
                10.,
            ),
            (
                "ab\n",
                1,
                100.,
                20.,
                false,
                true,
                3,
                if rtl { 0. } else { 20. },
                0.,
            ),
        ] {
            let description = format!("{direction:?}, {text:?}");
            let mut p = painter(TextSpan::new(text).style(style.clone()), direction);
            assert_eq!(
                p.set_max_lines(Some(0)),
                Err(TextPainterError::InvalidMaxLines)
            );
            p.set_max_lines(Some(max)).unwrap();
            p.layout_with_engine(&mut engine, 0., width).unwrap();
            assert_eq!(p.plain_text(), text, "{description}");
            assert_eq!(
                p.size(),
                Ok(Size::new(f32::min(width, intrinsic), 10. * max as f32)),
                "{description}"
            );
            assert_eq!(p.max_intrinsic_width(), Ok(intrinsic), "{description}");
            assert_eq!(p.did_exceed_max_lines(), Ok(exceeded), "{description}");
            assert_eq!(p.line_metrics().unwrap().len(), max, "{description}");
            assert_eq!(
                p.line_metrics().unwrap().last().unwrap().hard_break,
                hard_break,
                "{description}"
            );
            for byte in first_hidden..=text.len() {
                for affinity in [Downstream, Upstream] {
                    assert_eq!(
                        p.caret_offset(byte, affinity, 2.),
                        Ok(Offset::new(x, y)),
                        "{description}, byte {byte}, {affinity:?}, shaped {:#?}",
                        p.line_metrics().unwrap()
                    );
                    assert_eq!(
                        p.full_height_for_caret(byte, affinity),
                        Ok(10.),
                        "{description}, byte {byte}, {affinity:?}"
                    );
                }
            }
            p.set_max_lines(None).unwrap();
            assert_eq!(p.did_exceed_max_lines(), Err(TextPainterError::NeedsLayout));
            p.layout_with_engine(&mut engine, 0., width).unwrap();
            assert!(p.line_metrics().unwrap().len() > max, "{description}");
            assert_eq!(p.did_exceed_max_lines(), Ok(false), "{description}");
        }
    }
}

#[test]
fn tight_boxes_keep_natural_font_bounds_under_positive_and_negative_leading() {
    use TextLeadingDistribution::{Even, Proportional};
    use incular_core::{Offset, Rect};
    use incular_text::{TextHeightBehavior, TextLeadingDistribution, TextPainterBox};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let base = TextStyle::default().font_family("FlutterTest");
    let even = TextHeightBehavior::new(true, true, Even);
    let trimmed = TextHeightBehavior::new(false, false, Proportional);

    for (font, multiplier, style_distribution, behavior, height, top) in [
        (1., Some(20.), Some(Even), None, 20., 9.5),
        (10., Some(0.1), Some(Even), None, 1., -4.5),
        (10., Some(0.1), Some(Even), Some(trimmed), 10., 0.),
        (1., Some(20.), None, Some(even), 20., 9.5),
        (1., None, None, Some(even), 1., 0.),
        (1., None, None, Some(TextHeightBehavior::default()), 1., 0.),
    ] {
        let mut style = base.clone().font_size(font);
        if let Some(multiplier) = multiplier {
            style = style.line_height_multiplier(multiplier);
        }
        style.leading_distribution = style_distribution;
        let mut p = painter(TextSpan::new("A").style(style), TextDirection::Ltr);
        p.set_text_height_behavior(behavior);
        p.layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        assert_eq!(p.size(), Ok(Size::new(font, height)));
        assert_eq!(
            p.tight_boxes_for_selection(TextRange::new(0, 1)).unwrap(),
            [TextPainterBox {
                rect: Rect::from_origin_size(Offset::new(0., top), Size::new(font, font)),
                direction: TextDirection::Ltr,
            }],
            "size {font}, height multiplier {multiplier:?}, behavior {behavior:?}",
        );
    }
}

#[test]
fn forced_strut_does_not_stretch_the_selected_glyph_box() {
    use incular_core::{Offset, Rect};
    use incular_text::{StrutStyle, TextLeadingDistribution, TextPainterBox};
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let style = TextStyle::default()
        .font_family("FlutterTest")
        .font_size(20.);
    let mut p = painter(TextSpan::new("A").style(style), TextDirection::Ltr);
    let base_strut = StrutStyle::new()
        .height(10.)
        .font_size(10.)
        .force_strut_height(true);
    for (strut, baseline) in [
        (base_strut.clone(), 75.),
        (
            base_strut.leading_distribution(TextLeadingDistribution::Even),
            52.5,
        ),
    ] {
        p.set_strut_style(Some(strut));
        p.layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        assert_eq!(p.size().unwrap().height, 100.);
        assert_eq!(p.alphabetic_baseline(), Ok(baseline));
        assert_eq!(
            p.tight_boxes_for_selection(TextRange::new(0, 1)).unwrap(),
            [TextPainterBox {
                rect: Rect::from_origin_size(Offset::new(0., baseline - 15.), Size::new(20., 20.)),
                direction: TextDirection::Ltr,
            }],
        );
    }
}

#[test]
fn terminal_caret_includes_trailing_spaces_in_both_base_directions() {
    use incular_text::TextAffinity::Downstream;

    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let style = TextStyle::default().font_family("FlutterTest");
    let mut p = painter(TextSpan::new("a    "), TextDirection::Ltr);
    p.set_default_style(style.clone());
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    assert_eq!(p.caret_offset_utf16(5, Downstream, 5.).unwrap().x, 70.);

    p.set_text(Some(TextSpan::new("ل    ")));
    p.set_text_direction(Some(TextDirection::Rtl));
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    let actual = p.caret_offset_utf16(5, Downstream, 5.).unwrap().x;

    // When the installed system font supplies lam, its measured advance
    // differs from the Flutter tester's fallback. Isolate the geometry
    // from this expected choice of glyph.
    p.set_text(Some(TextSpan::new("ل")));
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    let lam = p.max_intrinsic_width().unwrap();
    assert!((actual - (1000. - lam - 4. * 14. - 5.)).abs() < 0.001);
}

#[test]
fn flutter_test_lam_metrics_match_exact_terminal_and_bidi_boundary_carets() {
    use incular_text::TextAffinity::Downstream;

    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_LAM_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("FlutterTest Lam", std::fs::read(path).unwrap()));
    let mut p = painter(TextSpan::new("ل    "), TextDirection::Rtl);
    p.set_default_style(TextStyle::default().font_family("FlutterTest Lam"));
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    assert_eq!(p.caret_offset_utf16(5, Downstream, 5.).unwrap().x, 925.);

    p.set_text(Some(TextSpan::new("aل")));
    p.set_text_direction(Some(TextDirection::Ltr));
    p.layout_with_engine(&mut engine, 1000., 1000.).unwrap();
    for (offset, x) in [(0, 0.), (1, 23.), (2, 28.)] {
        assert_eq!(p.caret_offset_utf16(offset, Downstream, 5.).unwrap().x, x);
    }
}

fn three_inline_widget_dimensions() -> (TextPainter, [incular_text::PlaceholderDimensions; 3]) {
    use incular_text::{PlaceholderDimensions, WidgetSpanAlignment};
    let span = TextSpan::new("before").children([
        InlineSpan::Widget(WidgetSpan::new(1)),
        InlineSpan::Widget(WidgetSpan::new(2)),
        InlineSpan::Widget(WidgetSpan::new(3)),
        InlineSpan::Text(TextSpan::new("after")),
    ]);
    let dimensions = [30., 40., 50.].map(|width| {
        PlaceholderDimensions::new(Size::new(width, 30.), WidgetSpanAlignment::TextBottom)
    });
    let mut painter = painter(span, TextDirection::Ltr);
    painter
        .set_placeholder_dimensions(Some(&dimensions))
        .unwrap();
    painter.layout(0., f32::INFINITY).unwrap();
    (painter, dimensions)
}

#[test]
fn changed_inline_widget_geometry_invalidates_paint_until_layout() {
    let (mut painter, mut dimensions) = three_inline_widget_dimensions();
    let shapes = painter.shaped_layout().unwrap();
    assert_eq!(
        shapes
            .placeholders
            .iter()
            .map(|item| (item.id, item.rect.size))
            .collect::<Vec<_>>(),
        [
            (1, Size::new(30., 30.)),
            (2, Size::new(40., 30.)),
            (3, Size::new(50., 30.))
        ]
    );

    dimensions[1].size.height = 20.;
    painter
        .set_placeholder_dimensions(Some(&dimensions))
        .unwrap();
    assert_eq!(
        painter.shaped_layout().map(|_| ()),
        Err(TextPainterError::NeedsLayout)
    );
    assert_eq!(
        painter.paint_with(incular_core::Offset::ZERO, |_, _| panic!(
            "stale paint geometry"
        )),
        Err(TextPainterError::PaintNeedsLayout),
    );
    painter.layout(0., f32::INFINITY).unwrap();
    assert_eq!(
        painter.shaped_layout().unwrap().placeholders[1].rect.size,
        Size::new(40., 20.)
    );
}

#[test]
fn unchanged_inline_widget_dimensions_preserve_the_existing_paint_layout() {
    let (mut painter, dimensions) = three_inline_widget_dimensions();
    let first = painter.shaped_layout().unwrap().clone();
    painter
        .set_placeholder_dimensions(Some(&dimensions))
        .unwrap();
    assert!(Arc::ptr_eq(&first, painter.shaped_layout().unwrap()));
    let mut painted = false;
    painter
        .paint_with(incular_core::Offset::ZERO, |layout, _| {
            painted = std::ptr::eq(layout, first.as_ref());
        })
        .unwrap();
    assert!(painted);
    painter.set_placeholder_dimensions(None).unwrap();
    painter.set_placeholder_dimensions(Some(&[])).unwrap();
    assert!(Arc::ptr_eq(&first, painter.shaped_layout().unwrap()));
    assert_eq!(
        painter.set_placeholder_dimensions(Some(&dimensions[1..])),
        Err(TextPainterError::InvalidPlaceholderCount)
    );
    assert!(Arc::ptr_eq(&first, painter.shaped_layout().unwrap()));
}

fn centered_width_ahem() -> Option<(TextEngine, TextStyle)> {
    let path = std::env::var("INCULAR_FLUTTER_AHEM_PATH").ok()?;
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Width Ahem", std::fs::read(path).unwrap()));
    Some((engine, TextStyle::default().font_family("Width Ahem")))
}

#[test]
fn centered_trailing_whitespace_offsets_caret_and_clamps_the_end() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let text = "test text with space at end   ";
    let mut painter = painter(
        TextSpan::new(text).style(style.font_size(14.)),
        TextDirection::Ltr,
    );
    painter.set_text_align(TextAlign::Center);
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(painter.size().unwrap().width, 420.);
    for (position, x) in [(0, 21.), (1, 35.), (2, 49.), (text.len(), 420.)] {
        assert_eq!(
            painter
                .caret_offset(position, incular_text::TextAffinity::Downstream, 0.)
                .unwrap()
                .x,
            x,
            "caret at {position}",
        );
    }
}

#[test]
fn centered_infinite_layout_preserves_width_and_has_no_finite_paint_position() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let mut painter = painter(
        TextSpan::new("A").style(style.font_size(10.)),
        TextDirection::Ltr,
    );
    painter.set_text_align(TextAlign::Center);
    painter
        .layout_with_engine(&mut engine, f32::INFINITY, f32::INFINITY)
        .unwrap();
    assert_eq!(painter.width().unwrap(), f32::INFINITY);
    assert_eq!(painter.size(), Err(TextPainterError::UnboundedSize));
    assert_eq!(painter.height().unwrap(), 10.);
    assert!(!painter.paint_offset().unwrap().x.is_finite());

    painter
        .paint_with(incular_core::Offset::ZERO, |_, _| {
            panic!("infinite centered geometry must not be passed to the renderer");
        })
        .unwrap();
    for (minimum, width, offset) in [(0., 10., 0.), (100., 100., 45.)] {
        painter
            .layout_with_engine(&mut engine, minimum, f32::INFINITY)
            .unwrap();
        assert_eq!(painter.width().unwrap(), width);
        assert_eq!(painter.paint_offset().unwrap().x, offset);
        let mut position = None;
        painter
            .paint_with(incular_core::Offset::new(5., 7.), |layout, at| {
                assert_eq!(layout.lines.len(), 1);
                position = Some(at);
            })
            .unwrap();
        assert_eq!(position, Some(incular_core::Offset::new(offset + 5., 7.)));
    }
}

#[test]
fn leading_justification_stays_paintable_even_with_infinite_layout_width() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let mut painter = painter(
        TextSpan::new("A").style(style.font_size(10.)),
        TextDirection::Ltr,
    );
    painter.set_text_align(TextAlign::Justify);
    for (minimum, width) in [(f32::INFINITY, f32::INFINITY), (0., 10.), (100., 100.)] {
        painter
            .layout_with_engine(&mut engine, minimum, f32::INFINITY)
            .unwrap();
        assert_eq!(painter.width().unwrap(), width);
        assert_eq!(painter.paint_offset().unwrap().x, 0.);
        let mut position = None;
        painter
            .paint_with(incular_core::Offset::ZERO, |_, offset| {
                position = Some(offset)
            })
            .unwrap();
        assert_eq!(position, Some(incular_core::Offset::ZERO));
    }
}

#[test]
fn painting_before_layout_reports_missing_paint_geometry() {
    let mut painter = TextPainter::new();
    painter.set_text_direction(Some(TextDirection::Ltr));
    let failure = painter
        .paint_with(incular_core::Offset::ZERO, |_, _| {
            panic!("an unmeasured painter must not submit a draw")
        })
        .unwrap_err();
    assert_eq!(failure, TextPainterError::PaintNeedsLayout);
    assert!(
        failure
            .to_string()
            .contains("TextPainter.paint called when text geometry was not yet calculated")
    );
}

fn assert_ltr_prefix_carets(
    engine: &mut TextEngine,
    style: &TextStyle,
    text: &str,
    boundaries: impl IntoIterator<Item = usize>,
) {
    let mut whole = painter(TextSpan::new(text).style(style.clone()), TextDirection::Ltr);
    whole.layout_with_engine(engine, 0., f32::INFINITY).unwrap();
    let caret = |unit| {
        whole
            .caret_offset_utf16(unit, incular_text::TextAffinity::Downstream, 0.)
            .unwrap()
            .x
    };
    for byte in boundaries {
        let prefix = &text[..byte];
        let mut prefix_painter = painter(
            TextSpan::new(prefix).style(style.clone()),
            TextDirection::Ltr,
        );
        prefix_painter
            .layout_with_engine(engine, 0., f32::INFINITY)
            .unwrap();
        let width = prefix_painter.width().unwrap();
        let actual = caret(prefix.encode_utf16().count());
        assert!(
            (actual - width).abs() < 0.001,
            "caret at {prefix:?} in {text:?}: {actual} != {width}"
        );
    }
    let positions = (0..=text.encode_utf16().count())
        .map(caret)
        .collect::<Vec<_>>();
    assert!(
        positions
            .windows(2)
            .all(|window| window[0] <= window[1] + 0.001),
        "{text:?}: {positions:?}"
    );
}

#[test]
fn a_combining_mark_stack_preserves_prefix_carets() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let text = "Z͉̳̺ͥͬ̾a̴͕̲̒̒͌̋ͪl̨͎̰̘͉̟ͤ̀̈̚͜g͕͔̤͖̟̒͝ͅo̵̡̡̼͚̐ͯ̅ͪ̆ͣ̚";
    let segmenter = icu_segmenter::GraphemeClusterSegmenter::new();
    assert_ltr_prefix_carets(
        &mut engine,
        &style.font_size(14.),
        text,
        segmenter.segment_str(text),
    );
}

#[test]
fn devanagari_shaping_preserves_the_caret_at_each_supplied_cluster_boundary() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let parts = ["प्रा", "प्त", " ", "व", "र्ण", "न", " ", "प्र", "व्रु", "ति"];
    let text = parts.concat();
    let mut offset = 0;
    let boundaries = std::iter::once(0).chain(parts.into_iter().map(|part| {
        offset += part.len();
        offset
    }));
    assert_ltr_prefix_carets(&mut engine, &style.font_size(14.), &text, boundaries);
}

#[test]
fn the_upstream_caret_in_thai_includes_the_final_combining_mark() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let text = "ฟห้";
    let mut painter = painter(
        TextSpan::new(text).style(style.font_size(14.)),
        TextDirection::Ltr,
    );
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(
        painter
            .caret_offset_utf16(
                text.encode_utf16().count(),
                incular_text::TextAffinity::Upstream,
                0.
            )
            .unwrap()
            .x,
        painter.width().unwrap()
    );
}

#[test]
fn terminal_caret_counts_the_full_inline_widget_advance() {
    use incular_text::{PlaceholderDimensions, TextAffinity, WidgetSpanAlignment};

    let span = TextSpan::new("before").children([
        InlineSpan::from(WidgetSpan::new(1)),
        InlineSpan::from(TextSpan::new("after")),
    ]);
    let mut painter = painter(span, TextDirection::Ltr);
    painter
        .set_placeholder_dimensions(Some(&[PlaceholderDimensions::new(
            Size::new(50., 30.),
            WidgetSpanAlignment::TextBottom,
        )
        .baseline_offset(25.)]))
        .unwrap();
    painter.layout(0., f32::INFINITY).unwrap();
    assert_eq!(painter.plain_text(), "before\u{fffc}after");
    assert_eq!(
        painter
            .caret_offset_utf16(
                painter.plain_text().encode_utf16().count(),
                TextAffinity::Downstream,
                0.
            )
            .unwrap()
            .x,
        painter.width().unwrap()
    );
}

#[test]
fn consecutive_inline_widgets_keep_distinct_carets_and_boxes_across_wrap() {
    use incular_text::{PlaceholderDimensions, TextAffinity, WidgetSpanAlignment};
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let mut children = Vec::new();
    for id in 0..14 {
        children.push(InlineSpan::from(WidgetSpan::new(id)));
        if matches!(id, 0 | 2) {
            children.push(InlineSpan::from(TextSpan::new("test")));
        }
    }
    let mut painter = painter(
        TextSpan::new("test")
            .style(style.font_size(14.))
            .children(children),
        TextDirection::Ltr,
    );
    let dimensions: [_; 14] = std::array::from_fn(|id| {
        PlaceholderDimensions::new(
            Size::new(if id == 12 { 51. } else { 50. }, 30.),
            WidgetSpanAlignment::TextBottom,
        )
        .baseline_offset(25.)
    });
    painter
        .set_placeholder_dimensions(Some(&dimensions))
        .unwrap();
    painter.layout_with_engine(&mut engine, 0., 500.).unwrap();
    for (offset, x) in [
        (1, 14.),
        (4, 56.),
        (5, 106.),
        (6, 120.),
        (10, 212.),
        (11, 262.),
        (12, 276.),
        (13, 290.),
        (14, 304.),
        (15, 318.),
        (16, 368.),
        (17, 418.),
        (18, 0.),
        (19, 50.),
        (23, 250.),
    ] {
        assert_eq!(
            painter
                .caret_offset_utf16(offset, TextAffinity::Downstream, 0.)
                .unwrap()
                .x,
            x,
            "caret {offset}"
        );
    }
    let layout = painter.shaped_layout().unwrap();
    assert_eq!(layout.placeholders.len(), 14);
    for (id, left, top, right) in [
        (0, 56., 0., 106.),
        (2, 212., 0., 262.),
        (3, 318., 0., 368.),
        (4, 368., 0., 418.),
        (5, 418., 0., 468.),
        (6, 0., 30., 50.),
        (7, 50., 30., 100.),
        (10, 200., 30., 250.),
        (11, 250., 30., 300.),
        (12, 300., 30., 351.),
        (13, 351., 30., 401.),
    ] {
        let item = &layout.placeholders[id];
        assert_eq!(item.id, id as u64);
        assert_eq!(
            item.rect,
            incular_core::Rect::from_origin_size(
                incular_core::Offset::new(left, top),
                Size::new(right - left, 30.)
            ),
            "box {id}"
        );
    }
}

fn emoji_caret_fixture() -> Option<(TextEngine, TextStyle)> {
    let path = std::env::var("INCULAR_FLUTTER_EMOJI_CARET_FONT_PATH").ok()?;
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Flutter Emoji Caret", std::fs::read(path).unwrap()));
    Some((
        engine,
        TextStyle::default()
            .font_family("Flutter Emoji Caret")
            .font_size(14.),
    ))
}

#[test]
fn family_and_modified_emoji_snap_every_utf16_caret_to_their_full_advance() {
    let Some((mut engine, style)) = emoji_caret_fixture() else {
        return;
    };
    let text = "👩‍👩‍👦👩‍👩‍👧‍👧👏🏽";
    let segmenter = icu_segmenter::GraphemeClusterSegmenter::new();
    assert_ltr_prefix_carets(&mut engine, &style, text, segmenter.segment_str(text));
    let mut painter = painter(TextSpan::new(text).style(style), TextDirection::Ltr);
    painter.layout_with_engine(&mut engine, 0., 10000.).unwrap();
    assert_eq!(text.encode_utf16().count(), 23);
    assert_eq!(painter.width(), Ok(126.));
    for offset in 0..=23 {
        let x = match offset {
            0 => 0.,
            1..=8 => 42.,
            9..=19 => 98.,
            _ => 126.,
        };
        assert_eq!(
            painter
                .caret_offset_utf16(offset, incular_text::TextAffinity::Downstream, 0.)
                .unwrap()
                .x,
            x,
            "caret {offset}"
        );
    }
}

fn assert_emoji_carets_across_style_boundary(
    direction: TextDirection,
    words: &str,
    first: &[f32],
    second: &[f32],
) {
    let Some((mut engine, style)) = emoji_caret_fixture() else {
        return;
    };
    let words_style = style.clone().font_weight(incular_text::FontWeight::BOLD);
    for (before, after, expected) in [
        (
            TextSpan::new("👩‍🚀"),
            TextSpan::new(format!(" {words}")).style(words_style.clone()),
            first,
        ),
        (
            TextSpan::new(format!("{words} ")).style(words_style),
            TextSpan::new("👩‍🚀"),
            second,
        ),
    ] {
        let mut painter = painter(
            TextSpan::empty()
                .style(style.clone())
                .children([InlineSpan::Text(before), InlineSpan::Text(after)]),
            direction,
        );
        painter
            .layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        assert_eq!(
            painter.plain_text().encode_utf16().count() + 1,
            expected.len()
        );
        let positions: Vec<_> = (0..expected.len())
            .map(|i| {
                painter
                    .caret_offset_utf16(i, incular_text::TextAffinity::Downstream, 0.)
                    .unwrap()
                    .x
            })
            .collect();
        assert_eq!(positions, expected);
        let terminal = if direction == TextDirection::Ltr {
            positions.last()
        } else {
            positions.first()
        };
        assert_eq!(terminal.copied(), Some(painter.width().unwrap()));
    }
}

#[test]
fn ltr_emoji_and_bold_letters_share_correct_caret_boundaries() {
    assert_emoji_carets_across_style_boundary(
        TextDirection::Ltr,
        "words",
        &[0., 28., 28., 28., 28., 28., 42., 56., 70., 84., 98., 112.],
        &[
            0., 14., 28., 42., 56., 70., 84., 112., 112., 112., 112., 112.,
        ],
    );
}

#[test]
fn rtl_emoji_and_bold_letters_share_correct_caret_boundaries() {
    assert_emoji_carets_across_style_boundary(
        TextDirection::Rtl,
        "מילים",
        &[112., 84., 84., 84., 84., 84., 70., 56., 42., 28., 14., 0.],
        &[112., 98., 84., 70., 56., 42., 28., 0., 0., 0., 0., 0.],
    );
}

#[test]
fn forced_strut_survives_leading_trim_in_empty_and_nonempty_paragraphs() {
    use incular_text::{StrutStyle, TextHeightBehavior};
    let Some((mut engine, font)) = centered_width_ahem() else {
        return;
    };
    let style = font.font_size(7.).height(Some(11.));
    let strut = StrutStyle::from_text_style(&style).force_strut_height(true);
    let mut painter = painter(TextSpan::new("x").style(style.clone()), TextDirection::Ltr);
    painter.set_strut_style(Some(strut));
    painter.set_text_height_behavior(Some(TextHeightBehavior {
        apply_height_to_first_ascent: false,
        apply_height_to_last_descent: false,
        ..TextHeightBehavior::default()
    }));
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    let height = painter.size().unwrap().height;
    assert_eq!(height, 77.);
    for span in [TextSpan::new("x"), TextSpan::new(""), TextSpan::empty()] {
        painter.set_text(Some(span.style(style.clone())));
        painter
            .layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        assert_eq!(painter.size().unwrap().height, height);
        assert_eq!(painter.preferred_line_height(), Ok(height));
    }
}

#[test]
fn an_explicit_natural_height_does_not_inherit_the_root_multiplier() {
    let Some((mut engine, font)) = centered_width_ahem() else {
        return;
    };
    let span = TextSpan::empty()
        .style(font.font_size(10.).height(Some(1000.)))
        .children([InlineSpan::from(
            TextSpan::new("A").style(TextStyle::default().height(Some(0.))),
        )]);
    let mut painter = painter(span, TextDirection::Ltr);
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(painter.size().unwrap().height, 10.);
}

#[test]
#[cfg(debug_assertions)]
fn debug_boxes_outline_natural_text_bounds_before_painting_the_paragraph() {
    use incular_core::{Color, Offset, Rect};
    use incular_text::TextPainterPaint;
    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Flutter Debug Boxes", std::fs::read(path).unwrap()));
    let font = TextStyle::default().font_family("Flutter Debug Boxes");
    let span = TextSpan::new("M")
        .style(font.clone().font_size(128.))
        .children([InlineSpan::from(
            TextSpan::new("M").style(font.font_size(64.)),
        )]);
    let mut painter = painter(span, TextDirection::Ltr);
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    let shape = painter.shaped_layout().unwrap().clone();
    for enabled in [false, true, false] {
        painter.set_debug_paint_text_layout_boxes(enabled);
        let origin = Offset::new(5., 7.);
        let mut outlines = Vec::new();
        let mut paragraph_painted = false;
        painter
            .paint_operations(origin, |op| match op {
                TextPainterPaint::DebugBox {
                    rect,
                    color,
                    stroke_width,
                } => {
                    assert!(!paragraph_painted);
                    assert_eq!(color, Color::rgba(0, 255, 255, 255));
                    assert_eq!(stroke_width, 1.);
                    outlines.push(rect);
                }
                TextPainterPaint::Paragraph { layout, offset } => {
                    assert!(!paragraph_painted);
                    assert!(std::ptr::eq(layout, shape.as_ref()));
                    assert_eq!(offset, origin);
                    paragraph_painted = true;
                }
            })
            .unwrap();
        assert!(paragraph_painted);
        if enabled {
            assert_eq!(
                outlines,
                [
                    Rect::from_origin_size(origin, Size::new(128., 128.)),
                    Rect::from_origin_size(origin + Offset::new(128., 48.), Size::new(64., 64.)),
                ]
            );
        } else {
            assert!(outlines.is_empty());
        }
    }
}

#[test]
fn manual_soft_hyphens_are_the_painter_default() {
    let mut painter = TextPainter::new();
    assert_eq!(painter.hyphens(), incular_text::Hyphens::Manual);
    painter.set_hyphens(incular_text::Hyphens::Hidden);
    assert_eq!(painter.hyphens(), incular_text::Hyphens::Hidden);
}

#[test]
fn a_taken_soft_hyphen_changes_paint_width_but_not_line_or_caret_geometry() {
    use incular_text::{Hyphens, TextAffinity};
    let Some((mut engine, font)) = centered_width_ahem() else {
        return;
    };
    let style = font.font_size(10.);
    let span = TextSpan::new("abc\u{00ad}de").style(style.clone());
    let mut painter = painter(span, TextDirection::Ltr);
    painter.set_text_width_basis(TextWidthBasis::LongestLine);
    for (mode, count, full) in [(Hyphens::Hidden, 3, 30_f32), (Hyphens::Manual, 4, 40.)] {
        painter.set_hyphens(mode);
        for (max, expected) in [(30., 30.), (35., full.min(35.)), (49., full), (50., 50.)] {
            painter.layout_with_engine(&mut engine, 0., max).unwrap();
            assert_eq!(painter.width(), Ok(expected), "{mode:?} at {max}");
            let lines = painter.line_metrics().unwrap();
            assert_eq!(
                lines.iter().map(|m| m.width).collect::<Vec<_>>(),
                if max < 50. { vec![30., 20.] } else { vec![50.] }
            );
            let shape = painter.shaped_layout().unwrap();
            if max < 50. {
                assert_eq!(
                    shape.lines[0]
                        .glyphs
                        .iter()
                        .filter(|g| g.advance > 0.)
                        .count(),
                    count
                );
            }
            assert_eq!(
                painter
                    .caret_offset_utf16(4, TextAffinity::Upstream, 0.)
                    .unwrap()
                    .x,
                30.
            );
            assert_eq!(
                painter
                    .caret_offset_utf16(4, TextAffinity::Downstream, 0.)
                    .unwrap()
                    .x,
                if max < 50. { 0. } else { 30. }
            );
        }
    }
    for (source, expected) in [("abc\u{00ad}", 30.), ("abc\u{00ad}\nde", 40.)] {
        painter.set_text(Some(TextSpan::new(source).style(style.clone())));
        painter.layout_with_engine(&mut engine, 0., 49.).unwrap();
        assert_eq!(painter.width(), Ok(expected));
    }
}

#[test]
fn changing_soft_hyphens_invalidates_the_retained_paint_width() {
    use incular_text::Hyphens;
    let Some((mut engine, font)) = centered_width_ahem() else {
        return;
    };
    let mut painter = painter(
        TextSpan::new("abc\u{00ad}de").style(font.font_size(10.)),
        TextDirection::Ltr,
    );
    painter.set_text_width_basis(TextWidthBasis::LongestLine);
    painter.set_hyphens(Hyphens::Hidden);
    painter.layout_with_engine(&mut engine, 0., 49.).unwrap();
    assert_eq!(painter.width(), Ok(30.));
    let shape = painter.shaped_layout().unwrap().clone();
    painter.set_hyphens(Hyphens::Hidden);
    assert!(Arc::ptr_eq(&shape, painter.shaped_layout().unwrap()));
    painter.set_hyphens(Hyphens::Manual);
    assert_eq!(painter.width(), Err(TextPainterError::NeedsLayout));
    painter.layout_with_engine(&mut engine, 0., 49.).unwrap();
    assert_eq!(painter.width(), Ok(40.));
}

#[test]
fn position_queries_distinguish_unlaid_out_and_invalidated_geometry() {
    use incular_core::Offset;
    use incular_text::TextPainterPositionError;
    let mut painter = painter(TextSpan::new("TEXT"), TextDirection::Ltr);
    let error = painter.position_for_offset(Offset::ZERO).unwrap_err();
    assert!(matches!(error, TextPainterPositionError::NeverLaidOut));
    assert!(
        error
            .to_string()
            .contains("The TextPainter has never been laid out.")
    );
    painter.layout(0., f32::INFINITY).unwrap();
    assert_eq!(painter.position_for_offset(Offset::ZERO).unwrap().offset, 0);
    painter.mark_needs_layout();
    let error = painter.position_for_offset(Offset::ZERO).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("The calls that first invalidated the text layout were:")
    );
    let TextPainterPositionError::LayoutInvalidated { first_invalidation } = error else {
        panic!("expected invalidation");
    };
    painter.mark_needs_layout();
    let TextPainterPositionError::LayoutInvalidated {
        first_invalidation: second,
    } = painter.position_for_offset(Offset::ZERO).unwrap_err()
    else {
        panic!("expected invalidation");
    };
    if cfg!(debug_assertions) {
        assert!(Arc::ptr_eq(
            first_invalidation.as_ref().unwrap(),
            second.as_ref().unwrap()
        ));
    }
    painter.layout(0., f32::INFINITY).unwrap();
    assert_eq!(painter.position_for_offset(Offset::ZERO).unwrap().offset, 0);
    painter.dispose().unwrap();
    assert!(matches!(
        painter.position_for_offset(Offset::ZERO),
        Err(TextPainterPositionError::Disposed)
    ));
}

#[test]
fn positions_use_the_painted_x_offset_and_line_vertical_extent() {
    use incular_core::Offset;
    let Some((mut engine, font)) = centered_width_ahem() else {
        return;
    };
    let mut painter = painter(
        TextSpan::new("ABC\nDE").style(font.font_size(10.)),
        TextDirection::Ltr,
    );
    painter.set_text_align(TextAlign::Center);
    painter.layout_with_engine(&mut engine, 90., 90.).unwrap();
    for (x, y, offset) in [
        (30., 0., 0),
        (36., 0., 1),
        (56., 0., 3),
        (35., 11., 4),
        (51., 11., 6),
    ] {
        assert_eq!(
            painter
                .position_for_offset(Offset::new(x, y))
                .unwrap()
                .offset,
            offset
        );
    }
}

#[test]
#[cfg(debug_assertions)]
fn painter_creation_and_disposal_dispatch_the_same_stable_identity() {
    use incular_core::memory::allocations::{MemoryAllocations, ObjectEventKind};
    use std::{cell::RefCell, rc::Rc};

    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let _subscription = MemoryAllocations::current().add_listener(move |event| {
        if event.class_name == "TextPainter" {
            observed.borrow_mut().push(event);
        }
    });
    let mut explicit = TextPainter::new();
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].kind, ObjectEventKind::Created);
    explicit.dispose().unwrap();
    assert_eq!(explicit.dispose(), Err(TextPainterError::Disposed));
    drop(explicit);

    let implicit = TextPainter::default();
    assert_eq!(events.borrow().len(), 3);
    drop(implicit);

    let events = events.borrow();
    assert_eq!(events.len(), 4);
    for pair in events.chunks_exact(2) {
        assert_eq!(pair[0].library, "painting");
        assert_eq!(pair[1].library, "painting");
        assert_eq!(pair[0].id, pair[1].id);
        assert_eq!(pair[0].kind, ObjectEventKind::Created);
        assert_eq!(pair[1].kind, ObjectEventKind::Disposed);
    }
    assert_ne!(events[0].id, events[2].id);
}

#[test]
fn invalid_utf16_replaces_the_entire_run_and_reports_once_without_failing_layout() {
    use std::error::Error as _;
    let mut input: Vec<u16> = "Hello".encode_utf16().collect();
    input.push(0xd83d);
    input.extend("World".encode_utf16());
    let mut diagnostic = None;
    let span = TextSpan::from_utf16(&input, |error| diagnostic = Some(error));
    let diagnostic = diagnostic.expect("one invalid-run diagnostic");
    assert!(diagnostic.source().is_some());
    assert!(diagnostic.is_silent());
    assert_eq!(span.text, "\u{fffd}");
    let valid: Vec<_> = "Hello😀World".encode_utf16().collect();
    assert_eq!(
        TextSpan::from_utf16(&valid, |_| panic!("valid pair")).text,
        "Hello😀World"
    );

    let Ok(path) = std::env::var("INCULAR_FLUTTER_REPLACEMENT_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::new();
    assert!(engine.register_font_bytes_as("Flutter UTF16", std::fs::read(path).unwrap()));
    let style = TextStyle::default()
        .font_family("Flutter UTF16")
        .font_size(20.);
    let mut painter = painter(span.style(style), TextDirection::Ltr);
    painter
        .layout_with_engine(&mut engine, 0., f32::INFINITY)
        .unwrap();
    assert_eq!(painter.width(), Ok(20.));
    assert_eq!(painter.shaped_layout().unwrap().glyph_count(), 1);
}

#[test]
#[ignore = "Parley wraps non-ASCII Unicode breakable space separators instead of hanging them"]
fn text_caret_preserves_unicode_trailing_space_policies_at_tight_width() {
    let Some((mut engine, style)) = centered_width_ahem() else {
        return;
    };
    let style = style.font_size(14.);
    let segmenter = icu_segmenter::GraphemeClusterSegmenter::new();
    for source in ["A", "A\u{1f600}"] {
        assert_ltr_prefix_carets(&mut engine, &style, source, segmenter.segment_str(source));
    }
    for trailing in [
        '\u{0020}', '\u{3000}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
        '\u{2004}', '\u{2005}', '\u{2006}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{205f}',
    ] {
        let source = format!("A{trailing}");
        assert_ltr_prefix_carets(&mut engine, &style, &source, segmenter.segment_str(&source));
        let mut painter = painter(
            TextSpan::new(source).style(style.clone()),
            TextDirection::Ltr,
        );
        painter
            .layout_with_engine(&mut engine, 0., f32::INFINITY)
            .unwrap();
        use incular_text::TextAffinity::Downstream;
        for (unit, expected) in [(0, 0.), (1, 14.), (2, painter.width().unwrap())] {
            assert_eq!(
                painter.caret_offset_utf16(unit, Downstream, 0.).unwrap().x,
                expected
            );
        }
        painter.layout_with_engine(&mut engine, 0., 14.).unwrap();
        let lines = painter.line_metrics().unwrap();
        assert_eq!(lines.len(), 1, "trailing {trailing:?}");
        assert_eq!(lines[0].width, 14., "trailing {trailing:?}");
    }
}

#[test]
fn fixed_width_realignment_matches_fresh_geometry_without_changing_the_source_layout() {
    use incular_core::Color;
    use incular_text::{TextLayout, TextLayoutOptions};

    let mut engine = TextEngine::new();
    // Compare against new shaping rather than the width/rebreak pools.
    engine.set_cache_byte_budget(0);
    let style = TextStyle::new().background_color(Color::WHITE);
    for (source, direction) in [
        ("", TextDirection::Rtl),
        ("loose end  \nnext word", TextDirection::Ltr),
        ("אבג \u{200c}דה   \nXYZ", TextDirection::Rtl),
    ] {
        let options = TextLayoutOptions::new(Some(80.), TextAlign::Left)
            .text_direction(direction);
        let original = engine.layout_with_options(source, &style, options);
        let original_offsets: Vec<_> = original.lines.iter().map(|line| line.offset).collect();
        let _ = original.line_caret_positions(0);
        let mut edited = original.clone();
        for align in [TextAlign::Right, TextAlign::Center, TextAlign::Start, TextAlign::Left] {
            let fresh = engine.layout_with_options(
                source,
                &style,
                TextLayoutOptions { align, ..options },
            );
            TextLayout::realign_fixed_width(&mut edited, 80., align, direction);
            assert_eq!(edited.metrics, fresh.metrics);
            assert_eq!(edited.glyph_count(), fresh.glyph_count());
            assert_eq!(edited.lines.len(), fresh.lines.len());
            let near = |actual: f32, expected: f32| {
                assert!((actual - expected).abs() < 0.0001,
                    "{direction:?} {align:?} {source:?}: {actual} != {expected}");
            };
            for (index, (actual, expected)) in edited.lines.iter().zip(fresh.lines.iter()).enumerate() {
                near(actual.offset, expected.offset);
                assert_eq!(actual.width, expected.width);
                assert_eq!(actual.glyphs.len(), expected.glyphs.len());
                let original_glyphs = &original.lines[index].glyphs;
                for ((a, b), source_glyph) in actual.glyphs.iter().zip(expected.glyphs.iter()).zip(original_glyphs.iter()) {
                    assert_eq!(a.id, b.id, "{direction:?} {align:?} {source:?}, line {index}");
                    assert_eq!(a.cluster, source_glyph.cluster, "alignment changes only positions");
                    near(a.offset.x, b.offset.x);
                    near(a.offset.y, b.offset.y);
                }
                let a = edited.line_caret_positions(index);
                let b = fresh.line_caret_positions(index);
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b.iter()) {
                    assert_eq!((a.offset, a.affinity), (b.offset, b.affinity));
                    near(a.x, b.x);
                }
            }
            let a = edited.backgrounds.as_deref().unwrap_or(&[]);
            let b = fresh.backgrounds.as_deref().unwrap_or(&[]);
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b.iter()) {
                assert_eq!(a.line, b.line);
                near(a.rect.origin.x, b.rect.origin.x);
                near(a.rect.origin.y, b.rect.origin.y);
                near(a.rect.size.width, b.rect.size.width);
            }
        }
        assert!(original.lines.iter().map(|line| line.offset).eq(original_offsets));
    }
}

#[test]
fn selection_height_policies_keep_line_leading_distinct_from_strut_ink() {
    use incular_text::{BoxHeightStyle as H, BoxWidthStyle as W, StrutStyle};

    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let style = TextStyle::new().font_family("FlutterTest").font_size(10.);
    for (force, line_height, spacing, strut_box) in [
        (None, 20., 7.5, [0., 20.]),
        (Some(true), 34., 7.5, [2., 32.]),
        (Some(false), 34., 9.5, [2., 32.]),
    ] {
        let style = if force.is_none() { style.clone().height(Some(2.)) } else { style.clone() };
        let mut p = painter(TextSpan::new("ab\ncd\nef").style(style), TextDirection::Ltr);
        if let Some(force) = force {
            p.set_strut_style(Some(
                StrutStyle::new().font_family("FlutterTest").font_size(20.)
                    .height(1.5).leading(0.2).force_strut_height(force),
            ));
        }
        p.layout_with_engine(&mut engine, 0., 40.).unwrap();
        for (line, start) in [(0, 0), (1, 3), (2, 6)] {
            let origin = line as f32 * line_height;
            for (height, [top, bottom]) in [
                (H::Max, [0., line_height]),
                (H::IncludeLineSpacingTop, [if line == 0 { spacing } else { 0. }, line_height]),
                (H::IncludeLineSpacingMiddle, [
                    if line == 0 { spacing } else { spacing * 0.5 },
                    if line == 2 { line_height } else { line_height + spacing * 0.5 },
                ]),
                (H::IncludeLineSpacingBottom, [
                    spacing,
                    if line == 2 { line_height } else { line_height + spacing },
                ]),
                (H::Strut, strut_box),
            ] {
                let boxes = p.boxes_for_selection(TextRange::new(start, start + 2), height, W::Tight)
                    .unwrap();
                assert_eq!(boxes.len(), 1);
                let box_ = boxes[0].rect;
                assert_eq!(box_.origin.y, origin + top, "{force:?}, {line}, {height:?}");
                assert_eq!(box_.origin.y + box_.size.height, origin + bottom, "{force:?}, {line}, {height:?}");
            }
        }
    }
}

#[test]
fn strut_height_style_without_a_strut_uses_each_runs_line_height() {
    use incular_text::{BoxHeightStyle, BoxWidthStyle};

    let Ok(path) = std::env::var("INCULAR_FLUTTER_TEST_FONT_PATH") else {
        return;
    };
    let mut engine = TextEngine::with_system_fonts(false);
    assert!(engine.register_font_bytes_as("FlutterTest", std::fs::read(path).unwrap()));
    let span = TextSpan::new("a")
        .style(TextStyle::new().font_family("FlutterTest").font_size(10.).height(Some(2.)))
        .child(TextSpan::new("b").style(TextStyle::new().font_size(20.).height(Some(1.5))))
        .child(TextSpan::new("c").style(TextStyle::new().font_size(8.).height(Some(0.5))));
    let mut p = painter(span, TextDirection::Ltr);
    p.layout_with_engine(&mut engine, 0., 80.).unwrap();
    let boxes = p.boxes_for_selection(
        TextRange::new(0, 3), BoxHeightStyle::Strut, BoxWidthStyle::Tight,
    ).unwrap();
    let heights: Vec<_> = boxes.iter().map(|b| {
        (b.rect.origin.y, b.rect.origin.y + b.rect.size.height)
    }).collect();
    assert_eq!(heights, [(7.5, 27.5), (0., 30.), (19.5, 23.5)]);
}
