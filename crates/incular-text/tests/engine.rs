use incular_core::Color;
use incular_text::{
    FontFamily, FontVariation, TextAffinity, TextAlign, TextEditingController, TextEngine,
    TextLayoutOptions, TextOverflow, TextSelection, TextStyle,
};
use std::sync::Arc;

#[test]
fn cache_ignores_color_but_not_size() {
    let mut engine = TextEngine::new();
    let mut style = TextStyle::default();
    let a = engine.layout("hello", &style, Some(200.), TextAlign::Start);
    style.color = Color::BLACK;
    let b = engine.layout("hello", &style, Some(200.), TextAlign::Start);
    assert!(Arc::ptr_eq(&a, &b));
    style.size = 20.;
    let c = engine.layout("hello", &style, Some(200.), TextAlign::Start);
    assert!(!Arc::ptr_eq(&a, &c));
}

#[test]
fn one_run_line_shares_glyph_positions_with_paint_run() {
    let mut engine = TextEngine::new();
    let layout = engine.layout(
        "A single font line",
        &TextStyle::default(),
        None,
        TextAlign::Start,
    );
    assert_eq!(layout.line_count(), 1);
    let line = layout.line(0);
    assert_eq!(line.runs().len(), 1);
    assert!(Arc::ptr_eq(
        &line.paragraph_line().glyphs,
        &line.runs()[0].glyphs
    ));

    let mixed = engine.layout("abc אבג xyz", &TextStyle::default(), None, TextAlign::Start);
    let line = mixed.line(0);
    assert!(line.runs().len() > 1);
    let paint_glyphs: Vec<_> = line
        .runs()
        .iter()
        .flat_map(|run| run.glyphs.iter().copied())
        .collect();
    assert_eq!(line.glyphs().collect::<Vec<_>>(), paint_glyphs);
}

#[test]
fn cache_eviction_keeps_recent_entries_and_external_layouts_alive() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    // Centered lines never take the cross-width path, so this exercises the
    // primary cache alone.
    let oldest = engine.layout("oldest entry", &style, None, TextAlign::Center);
    let entry = engine.layout("entry 0", &style, None, TextAlign::Center);
    for index in 1..2047 {
        engine.layout(&format!("entry {index}"), &style, None, TextAlign::Center);
    }

    // A hit makes the oldest insertion the most recently used entry.
    let hit = engine.layout("oldest entry", &style, None, TextAlign::Center);
    assert!(Arc::ptr_eq(&oldest, &hit));
    engine.layout("one beyond capacity", &style, None, TextAlign::Center);

    let retained = engine.layout("oldest entry", &style, None, TextAlign::Center);
    let rebuilt = engine.layout("entry 0", &style, None, TextAlign::Center);
    assert!(Arc::ptr_eq(&oldest, &retained));
    assert!(!Arc::ptr_eq(&entry, &rebuilt));
    assert!(entry.glyph_count() > 0, "external owners survive eviction");
}

#[test]
fn layout_pool_grows_until_a_cyclic_working_set_fits() {
    // 3000 labels exceed the 2048-entry base pool. Each pass used to evict
    // every label just before its reuse; the pool must grow until a pass
    // is served entirely from cache.
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let labels: Vec<String> = (0..3000).map(|index| format!("label {index}")).collect();
    let mut misses = Vec::new();
    for _ in 0..3 {
        let before = engine.diagnostics().cache_misses;
        for label in &labels {
            // Centered lines bypass cross-width reuse, isolating the pool.
            engine.layout(label, &style, Some(400.), TextAlign::Center);
        }
        misses.push(engine.diagnostics().cache_misses - before);
    }
    assert_eq!(misses[0], 3000);
    assert_eq!(misses[2], 0, "passes after growth hit: {misses:?}");
    assert!(engine.diagnostics().cache_growths > 0);
}

#[test]
fn single_line_layout_is_reused_across_widths_it_fits() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let wide = engine.layout("a short label", &style, Some(300.), TextAlign::Start);
    let shaped = engine.diagnostics().shaping_runs;
    let resized = engine.layout("a short label", &style, Some(310.), TextAlign::Start);
    let unbounded = engine.layout("a short label", &style, None, TextAlign::Start);
    assert!(Arc::ptr_eq(&wide, &resized));
    assert!(Arc::ptr_eq(&wide, &unbounded));
    assert_eq!(engine.diagnostics().shaping_runs, shaped, "no reshaping");
    assert_eq!(engine.diagnostics().width_reuses, 2);

    // A width the line no longer fits wraps, so it must reshape.
    let narrow = engine.layout(
        "a short label",
        &style,
        Some(wide.metrics.size.width / 2.),
        TextAlign::Start,
    );
    assert!(narrow.line_count() > 1);
}

#[test]
fn rewrapping_a_shaped_paragraph_matches_a_fresh_layout() {
    let style = TextStyle::default();
    let paragraph = "A paragraph long enough to wrap over several lines at each of the widths used here, mixing words, numbers 12345 and punctuation.";
    let mut engine = TextEngine::new();
    for (index, width) in [300., 220., 410., 150., 300.].into_iter().enumerate() {
        for align in [TextAlign::Start, TextAlign::Center, TextAlign::End] {
            let rewrapped = engine.layout(paragraph, &style, Some(width), align);
            let fresh = TextEngine::new().layout(paragraph, &style, Some(width), align);
            assert!(rewrapped.line_count() > 1, "{width} wraps");
            assert_eq!(rewrapped.line_count(), fresh.line_count());
            assert_eq!(rewrapped.metrics.size, fresh.metrics.size);
            for (left, right) in rewrapped.lines().zip(fresh.lines()) {
                assert_eq!((left.start(), left.end()), (right.start(), right.end()));
                assert_eq!(
                    (left.offset(), left.width()),
                    (right.offset(), right.width())
                );
                assert_eq!(
                    left.glyphs().collect::<Vec<_>>(),
                    right.glyphs().collect::<Vec<_>>()
                );
            }
        }
        if index > 0 {
            assert!(
                engine.diagnostics().shapes_reused > 0,
                "{width} reused shaping"
            );
        }
    }
}

#[test]
fn width_dependent_layouts_are_not_reused_across_widths() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    for align in [TextAlign::Center, TextAlign::End, TextAlign::Right] {
        let first = engine.layout("label", &style, Some(300.), align);
        let second = engine.layout("label", &style, Some(400.), align);
        assert!(!Arc::ptr_eq(&first, &second), "{align:?}");
        assert_ne!(first.line(0).offset(), second.line(0).offset(), "{align:?}");
    }
    let rtl = engine.layout("אבג", &style, Some(300.), TextAlign::Start);
    let wider = engine.layout("אבג", &style, Some(400.), TextAlign::Start);
    assert!(!Arc::ptr_eq(&rtl, &wider));
    assert_eq!(engine.diagnostics().width_reuses, 0);
}

#[test]
fn shared_engine_keeps_layouts_across_calls_and_tolerates_nesting() {
    let style = TextStyle::default();
    let first = TextEngine::with_shared(|engine| {
        engine.layout("shared measurement", &style, None, TextAlign::Start)
    });
    let (second, nested) = TextEngine::with_shared(|engine| {
        let nested = TextEngine::with_shared(|inner| {
            inner.layout("nested measurement", &style, None, TextAlign::Start)
        });
        (
            engine.layout("shared measurement", &style, None, TextAlign::Start),
            nested,
        )
    });
    assert!(Arc::ptr_eq(&first, &second));
    assert!(nested.glyph_count() > 0);
}

#[test]
fn composed_document_keeps_one_font_record_per_shaped_run() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let paragraph = "This paragraph wraps over several visual lines so a font run must not be copied for every line in the composed document.";
    let width = Some(100.);
    let shaped = engine.layout(paragraph, &style, width, TextAlign::Start);
    assert!(shaped.line_count() > 1);

    let document = engine.layout(
        &format!("{paragraph}\n{paragraph}"),
        &style,
        width,
        TextAlign::Start,
    );
    assert_eq!(
        document.font_runs().count(),
        2 * shaped.font_runs().count(),
        "composition must rebase each run once, independent of visual line count"
    );
    for (shaped_run, rebased) in shaped.font_runs().zip(document.font_runs()) {
        assert_eq!(rebased.range, shaped_run.range);
    }
    let second_start = paragraph.len() + 1;
    for (shaped_run, rebased) in shaped
        .font_runs()
        .zip(document.font_runs().skip(shaped.font_runs().count()))
    {
        assert_eq!(rebased.range.start, shaped_run.range.start + second_start);
        assert_eq!(rebased.range.end, shaped_run.range.end + second_start);
    }
}

#[test]
fn unicode_clusters_are_never_split_before_resolution() {
    let mut engine = TextEngine::new();
    for text in ["e\u{301}", "नमस्ते", "مرحبا", "👩\u{200d}💻", "✈\u{fe0f}"]
    {
        let layout = engine.layout(text, &TextStyle::default(), None, TextAlign::Start);
        assert!(layout.metrics.size.width.is_finite());
        assert!(
            layout
                .font_runs()
                .all(|run| run.range.start <= run.range.end)
        );
    }
}

#[test]
fn bidi_layout_exposes_visual_caret_stops_and_affinity() {
    let mut engine = TextEngine::new();
    let text = "abc אבג xyz";
    let layout = engine.layout(text, &TextStyle::default(), None, TextAlign::Start);
    let stops = layout.line_caret_positions(0);
    assert!(!stops.is_empty());
    assert!(stops.windows(2).all(|pair| pair[0].x <= pair[1].x));
    assert!(
        stops
            .iter()
            .any(|stop| stop.affinity == TextAffinity::Upstream)
    );

    let controller = TextEditingController::with_text(text);
    let first = stops[0];
    controller.set_selection_with_affinity(TextSelection::collapsed(first.offset), first.affinity);
    controller.move_right_visual(&layout, false);
    assert_eq!(controller.selection().extent, stops[1].offset);
    assert_eq!(controller.caret_affinity(), stops[1].affinity);
}

#[test]
fn caret_stops_of_one_line_do_not_depend_on_query_order() {
    let mut engine = TextEngine::new();
    let text = "first line\nsecond\nthird one";
    let layout = engine.layout(text, &TextStyle::default(), None, TextAlign::Start);
    let twin = engine.layout(
        &format!("{text}\n"),
        &TextStyle::default(),
        None,
        TextAlign::Start,
    );
    // Query the last line before any other line has been derived.
    let third = layout.line_caret_positions(2).to_vec();
    assert!(third.iter().all(|stop| stop.line == 2));
    assert_eq!(third.first().map(|stop| stop.offset), Some(18));
    assert_eq!(third.last().map(|stop| stop.offset), Some(text.len()));
    assert!(layout.line_caret_positions(3).is_empty());
    for line in 0..3 {
        let left = layout.line_caret_positions(line);
        let right = twin.line_caret_positions(line);
        assert_eq!(left.len(), right.len());
        assert!(
            left.iter()
                .zip(right)
                .all(|(a, b)| a.offset == b.offset && a.x == b.x && a.affinity == b.affinity)
        );
    }
}

#[test]
fn registered_fonts_advance_generation_and_invalidate_layouts() {
    let mut engine = TextEngine::new();
    let first = engine.layout("hello", &TextStyle::default(), None, TextAlign::Start);
    let generation = engine.font_database_generation();
    engine.register_font_bytes(Vec::new());
    assert!(engine.font_database_generation() > generation);
    let second = engine.layout("hello", &TextStyle::default(), None, TextAlign::Start);
    assert!(!Arc::ptr_eq(&first, &second));
}

#[test]
fn soft_wrapping_and_max_lines_bound_retained_layout() {
    let mut engine = TextEngine::new();
    let layout = engine.layout_with_options(
        "one two three four five six seven eight",
        &TextStyle::default().font_size(18.),
        TextLayoutOptions::new(Some(70.), TextAlign::Start)
            .max_lines(Some(2))
            .overflow(TextOverflow::Clip),
    );
    assert_eq!(layout.line_count(), 2);
    assert!(layout.overflowed);

    let unwrapped = engine.layout_with_options(
        "one two three four",
        &TextStyle::default().font_size(18.),
        TextLayoutOptions::new(Some(20.), TextAlign::Start).soft_wrap(false),
    );
    assert_eq!(unwrapped.line_count(), 1);
    assert!(unwrapped.overflowed);
}

#[test]
fn icon_data_resolves_font_glyph_through_text_engine() {
    let icon = incular_text::IconData::new('A' as u32)
        .family(FontFamily::SystemUi)
        .variations([FontVariation::weight(600.)]);
    let glyph = icon.glyph_text().expect("valid icon code point");
    let mut engine = TextEngine::new();
    let layout = engine.layout(&glyph, &icon.text_style(24.), None, TextAlign::Start);
    assert!(layout.glyph_count() > 0);
    assert_eq!(icon.font_variations(), &[FontVariation::weight(600.)]);
}

#[test]
fn ellipsis_is_shaped_and_never_splits_bidi_graphemes() {
    let mut engine = TextEngine::new();
    let source = "עברית café 👩\u{200d}💻 mixed English words";
    let clipped = engine.layout_with_options(
        source,
        &TextStyle::default().font_size(20.),
        TextLayoutOptions::new(Some(95.), TextAlign::Start)
            .max_lines(Some(1))
            .overflow(TextOverflow::Clip),
    );
    let ellipsized = engine.layout_with_options(
        source,
        &TextStyle::default().font_size(20.),
        TextLayoutOptions::new(Some(95.), TextAlign::Start)
            .max_lines(Some(1))
            .overflow(TextOverflow::Ellipsis),
    );
    assert_eq!(ellipsized.line_count(), 1);
    assert!(ellipsized.overflowed);
    // The final source is re-shaped with U+2026, so it has at least one
    // renderer glyph even when the visible source prefix is empty.
    assert!(ellipsized.glyph_count() > 0);
    assert!(ellipsized.glyph_count() >= clipped.glyph_count());
    assert!(ellipsized.font_runs().any(|run| run.direction == "rtl"));
}

mod document_contracts {
    use incular_core::Color;
    use incular_text::{TextAlign, TextEngine, TextLayoutOptions, TextStyle};
    use std::sync::Arc;

    fn paragraph(index: usize) -> String {
        format!("Paragraph {index}: the retained per-paragraph cache keeps warm documents cheap.")
    }

    fn document(paragraphs: usize) -> String {
        (0..paragraphs)
            .map(paragraph)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn warm_document_reshapes_nothing() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = document(300);
        let first = engine.layout(&doc, &style, None, TextAlign::Start);
        let cold = engine.diagnostics();
        assert_eq!(cold.paragraphs_reshaped, 300);

        let second = engine.layout(&doc, &style, None, TextAlign::Start);
        let warm = engine.diagnostics();
        assert_eq!(
            warm.paragraphs_reshaped - cold.paragraphs_reshaped,
            0,
            "warm document must not reshape"
        );
        assert_eq!(
            warm.documents_reused - cold.documents_reused,
            1,
            "the composed document is served whole"
        );
        assert_eq!(warm.documents_composed, cold.documents_composed);
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn document_longer_than_the_base_pool_keeps_its_paragraphs() {
        // The base pool holds 2048 layouts. A longer file must still reuse
        // every unchanged paragraph when one paragraph is edited.
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let mut lines: Vec<String> = (0..3000).map(paragraph).collect();
        engine.layout(&lines.join("\n"), &style, None, TextAlign::Start);
        let before = engine.diagnostics();

        lines[1500] = format!("EDITED {}", paragraph(1500));
        engine.layout(&lines.join("\n"), &style, None, TextAlign::Start);
        let after = engine.diagnostics();
        assert_eq!(after.paragraphs_reshaped - before.paragraphs_reshaped, 1);
        assert_eq!(
            after.parley_layouts_reused - before.parley_layouts_reused
                + (after.paragraphs_carried - before.paragraphs_carried),
            2999
        );
    }

    fn unwrapped(width: f32) -> TextLayoutOptions {
        TextLayoutOptions::new(Some(width), TextAlign::Start).soft_wrap(false)
    }

    #[test]
    fn unwrapped_clipped_document_composes_like_the_monolithic_layout() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = document(40);
        let composed = engine.layout_with_options(&doc, &style, unwrapped(4000.));
        // A line limit routes the same text through whole-string shaping.
        let monolithic =
            engine.layout_with_options(&doc, &style, unwrapped(4000.).max_lines(Some(40)));
        assert_eq!(engine.diagnostics().documents_composed, 1);
        assert_eq!(composed.line_count(), monolithic.line_count());
        assert!(!composed.overflowed);
        assert!((composed.metrics.size.width - monolithic.metrics.size.width).abs() < 0.01);
        assert!((composed.metrics.size.height - monolithic.metrics.size.height).abs() < 0.01);
        for (left, right) in composed.lines().zip(monolithic.lines()) {
            assert_eq!(left.start(), right.start());
            assert_eq!(left.glyphs().len(), right.glyphs().len());
            assert!((left.width() - right.width()).abs() < 0.01);
            assert!((left.baseline() - right.baseline()).abs() < 0.01);
        }
    }

    #[test]
    fn unwrapped_clipped_document_keeps_the_first_line_on_horizontal_overflow() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = document(40);
        let clipped = engine.layout_with_options(&doc, &style, unwrapped(60.));
        let full = engine.layout_with_options(&doc, &style, unwrapped(4000.));
        assert!(clipped.overflowed);
        assert_eq!(clipped.line_count(), 1);
        assert_eq!(clipped.line(0).end(), full.line(0).end());
        assert_eq!(clipped.metrics.size.height, clipped.metrics.line_height);
    }

    #[test]
    fn unwrapped_clipped_edit_reshapes_exactly_one_paragraph() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let mut lines: Vec<String> = (0..300).map(paragraph).collect();
        engine.layout_with_options(&lines.join("\n"), &style, unwrapped(4000.));
        let before = engine.diagnostics();

        lines[150] = format!("EDITED {}", paragraph(150));
        engine.layout_with_options(&lines.join("\n"), &style, unwrapped(4000.));
        let after = engine.diagnostics();
        assert_eq!(after.paragraphs_reshaped - before.paragraphs_reshaped, 1);
    }

    fn assert_same_composition(
        carried: &incular_text::TextLayout,
        fresh: &incular_text::TextLayout,
    ) {
        assert_eq!(carried.line_count(), fresh.line_count());
        assert_eq!(carried.metrics.size, fresh.metrics.size);
        assert_eq!(carried.metrics.baseline, fresh.metrics.baseline);
        assert_eq!(
            carried.font_runs().map(|run| run.range).collect::<Vec<_>>(),
            fresh.font_runs().map(|run| run.range).collect::<Vec<_>>()
        );
        for (left, right) in carried.lines().zip(fresh.lines()) {
            assert_eq!(left.index(), right.index());
            let painted = |line: incular_text::TextLineRef<'_>| {
                line.runs()
                    .iter()
                    .map(|run| line.paint_origin().y + run.origin.y)
                    .collect::<Vec<_>>()
            };
            assert_eq!(painted(left), painted(right));
            assert_eq!(
                carried.line_caret_positions(left.index()),
                fresh.line_caret_positions(right.index())
            );
            assert_eq!(
                (left.start(), left.caret_end(), left.end()),
                (right.start(), right.caret_end(), right.end())
            );
            assert_eq!(left.baseline(), right.baseline());
            assert_eq!(left.width(), right.width());
            assert_eq!(
                left.glyphs().collect::<Vec<_>>(),
                right.glyphs().collect::<Vec<_>>()
            );
            assert_eq!(left.clusters().len(), right.clusters().len());
            assert!(left.clusters().zip(right.clusters()).all(|(a, b)| (
                a.start, a.end, a.left, a.right
            ) == (
                b.start, b.end, b.left, b.right
            )));
        }
    }

    #[test]
    fn carried_paragraphs_compose_exactly_like_a_fresh_layout() {
        let style = TextStyle::default();
        let base = document(120);
        let removed = base.find("\nParagraph 90:").expect("paragraph 90");
        let first_break = base.find('\n').expect("a document");
        let edits = [
            format!("X{base}"),
            format!("{base}X"),
            format!("{base}\ntrailing"),
            format!("\n{base}"),
            format!("Leading line\n{base}"),
            base[first_break + 1..].to_string(),
            base[..base.len() - 10].to_string(),
            base.replacen("Paragraph 60:", "Paragraph sixty:", 1),
            base.replacen("Paragraph 60:", "Paragraph\n60:", 1),
            format!("{}{}", &base[..removed], &base[removed + 1..]),
            // Repeated paragraphs let the shared prefix and suffix overlap.
            "same\nsame\nsame\nsame".to_owned(),
            "same\nsame\nsame\nsame\nsame".to_owned(),
            "same\nsame\nsame".to_owned(),
            "same\n\nsame\nsame".to_owned(),
            "\nsame\nsame\nsame".to_owned(),
            "same\nsame\nsame\n".to_owned(),
        ];
        for width in [None, Some(240.)] {
            let mut engine = TextEngine::new();
            engine.layout(&base, &style, width, TextAlign::Start);
            for edited in &edits {
                let carried = engine.layout(edited, &style, width, TextAlign::Start);
                let fresh = TextEngine::new().layout(edited, &style, width, TextAlign::Start);
                assert_same_composition(&carried, &fresh);
            }
            assert!(engine.diagnostics().paragraphs_carried > 0);
        }
    }

    #[test]
    fn edits_near_the_top_carry_the_paragraphs_after_them() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = document(301);
        engine.layout(&doc, &style, Some(400.), TextAlign::Start);
        for edited in [
            format!("X{doc}"),
            format!("\nX{doc}"),
            format!("\n\nX{doc}"),
        ] {
            let before = engine.diagnostics();
            engine.layout(&edited, &style, Some(400.), TextAlign::Start);
            let after = engine.diagnostics();
            assert!(
                after.paragraphs_considered - before.paragraphs_considered <= 2,
                "an edit at the top lays out at most the edited paragraphs"
            );
            assert!(after.paragraphs_carried - before.paragraphs_carried >= 300);
        }
    }

    #[test]
    fn lines_are_found_by_index_and_offset_across_paragraphs() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = "first\nsecond\n\nfourth";
        let layout = engine.layout(doc, &style, None, TextAlign::Start);
        assert_eq!(layout.line_count(), 4);
        assert_eq!(layout.paragraphs().len(), 4);
        let starts: Vec<_> = layout.lines().map(|line| line.start()).collect();
        assert_eq!(starts, [0, 6, 13, 14]);
        for (index, line) in layout.lines().enumerate() {
            assert_eq!(line.index(), index);
            assert_eq!(layout.line(index).start(), line.start());
            assert_eq!(layout.line_for_offset(line.start()), index);
        }
        assert_eq!(layout.line_for_offset(3), 0);
        // A hard break belongs to no line, like Flutter's getLineBoundary.
        assert_eq!(layout.line_for_offset(5), 3);
        assert_eq!(layout.line_for_offset(doc.len()), 3);
        assert!(layout.get_line(4).is_none());
        let fourth = layout.line(3);
        assert_eq!(fourth.glyphs().next().map(|glyph| glyph.cluster), Some(14));
        assert!(fourth.baseline() > layout.line(2).baseline());
        assert_eq!(fourth.paragraph_line().start, 0);
    }

    #[test]
    fn single_paragraph_edit_reshapes_exactly_one_paragraph() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let mut doc = document(301);
        engine.layout(&doc, &style, None, TextAlign::Start);
        let before = engine.diagnostics();

        // Edit the middle paragraph only.
        let middle = format!("EDITED {}", paragraph(150));
        let mut lines: Vec<String> = doc.split('\n').map(str::to_owned).collect();
        lines[150] = middle;
        doc = lines.join("\n");

        engine.layout(&doc, &style, None, TextAlign::Start);
        let after = engine.diagnostics();
        assert_eq!(
            after.paragraphs_reshaped - before.paragraphs_reshaped,
            1,
            "only the edited paragraph reshapes"
        );
        assert_eq!(
            after.parley_layouts_reused - before.parley_layouts_reused
                + (after.paragraphs_carried - before.paragraphs_carried),
            300,
            "all other paragraphs reuse their layouts"
        );
        assert_eq!(
            after.paragraphs_carried - before.paragraphs_carried,
            300,
            "paragraphs before and after the edit are carried without lookup"
        );
    }

    #[test]
    fn color_only_change_is_a_layout_cache_hit() {
        // LayoutKey deliberately excludes color: recoloring must not shape.
        let mut engine = TextEngine::new();
        let plain = TextStyle::default();
        engine.layout("color contract", &plain, None, TextAlign::Start);
        let before = engine.diagnostics();

        let recolored = TextStyle {
            color: Color::rgba(255, 0, 0, 255),
            ..plain.clone()
        };
        let first = engine.layout("color contract", &recolored, None, TextAlign::Start);
        let second = engine.layout("color contract", &plain, None, TextAlign::Start);
        let after = engine.diagnostics();
        assert_eq!(after.shaping_runs - before.shaping_runs, 0, "no reshaping");
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn selection_and_caret_geometry_do_not_touch_the_engine() {
        // Selection math consumes the already-retained TextLayout; this test
        // pins that invariant at the engine boundary: repeated layout calls
        // for identical input stay cache hits even while callers mutate
        // selection state externally.
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = "selectable body\nsecond line\nthird".to_string();
        engine.layout(&doc, &style, Some(320.), TextAlign::Start);
        let before = engine.diagnostics();

        for _ in 0..25 {
            engine.layout(&doc, &style, Some(320.), TextAlign::Start);
        }
        let after = engine.diagnostics();
        assert_eq!(after.cache_misses - before.cache_misses, 0);
        assert_eq!(after.shaping_runs - before.shaping_runs, 0);
        assert_eq!(after.paragraphs_reshaped - before.paragraphs_reshaped, 0);
    }
}

mod probe_compare {
    use incular_text::{TextAlign, TextEngine, TextStyle};

    #[test]
    fn multiline_document_runs_have_increasing_vertical_origins() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = "Line 1\nLine 2\nLine 3";
        let layout = engine.layout(doc, &style, Some(200.0), TextAlign::Start);
        assert_eq!(layout.line_count(), 3);
        // Runs are positioned within their paragraph; the line's paint
        // origin places the paragraph in the layout.
        let origin = |index: usize| {
            let line = layout.line(index);
            line.paint_origin().y + line.runs()[0].origin.y
        };
        assert_eq!(origin(0), 0.0);
        assert!(origin(1) > 0.0);
        assert!(origin(2) > origin(1));
    }

    #[test]
    fn probe_mono_vs_composed() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let doc = "abcdef\nxy\n123456";
        let mono = engine.layout(doc, &style, Some(120.), TextAlign::Start);
        let d1 = engine.diagnostics();
        println!(
            "mono: lh={} h={} lines={}",
            mono.metrics.line_height,
            mono.metrics.size.height,
            mono.line_count()
        );
        for (i, l) in mono.lines().enumerate() {
            println!(
                "  mono[{i}]: start={} end={} caret_end={} baseline={}",
                l.start(),
                l.end(),
                l.caret_end(),
                l.baseline()
            );
        }
        // Force composed by clearing cache? Routing keys off contains('\n') only.
        let _ = d1;
    }
}

mod probe_entry_size {
    use incular_text::{TextAlign, TextEngine, TextStyle};

    #[test]
    fn probe_single_layout_deep_size() {
        let mut engine = TextEngine::new();
        let style = TextStyle::default();
        let live0 = probe_live();
        let layout = engine.layout("warm", &style, None, TextAlign::Start);
        let live1 = probe_live();
        // VmRSS is sampled around an allocator/cache warm-up and can move in
        // either direction when the OS reclaims pages. Keep this diagnostic
        // probe observational instead of allowing a harmless decrease to
        // panic in debug builds.
        println!("delta_live_bytes={}", live1.saturating_sub(live0));
        println!(
            "lines={} glyphs={} runs={}",
            layout.line_count(),
            layout.glyph_count(),
            layout.font_runs().count()
        );
        println!("cache_misses={}", engine.diagnostics().cache_misses);
    }

    fn probe_live() -> usize {
        // Reads this process's VmRSS as a stable proxy (pages, KB→B).
        let Ok(stat) = std::fs::read_to_string("/proc/self/status") else {
            // VmRSS is Linux-specific; keep the diagnostic probe harmless on
            // Windows and other supported hosts.
            return 0;
        };
        stat.split('\n')
            .find(|line| line.starts_with("VmRSS"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<usize>().ok())
            .unwrap_or(0)
            * 1024
    }
}
