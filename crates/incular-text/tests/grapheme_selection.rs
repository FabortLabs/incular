use incular_text::{TextAlign, TextEngine, TextLayout, TextLayoutOptions, TextStyle};

fn layout_options() -> TextLayoutOptions {
    TextLayoutOptions::new(Some(300.0), TextAlign::Start)
}

fn caret_offsets(layout: &TextLayout, line: usize) -> Vec<usize> {
    layout
        .line_caret_positions(line)
        .iter()
        .map(|caret| caret.offset)
        .collect()
}

#[test]
fn joiner_only_selection_is_empty_and_grapheme_carets_remain_complete() {
    let mut engine = TextEngine::new();

    let emoji = "👩\u{200d}💻";
    let emoji_text = format!("a{emoji}b");
    let emoji_layout = engine.layout_with_options(
        &emoji_text,
        &TextStyle::default(),
        layout_options().soft_wrap(false),
    );
    assert!(emoji_layout.line(0).selection_spans(5, 8).is_empty());

    let emoji_carets = caret_offsets(&emoji_layout, 0);
    for expected in [0, 1, 1 + emoji.len(), emoji_text.len()] {
        assert!(
            emoji_carets.contains(&expected),
            "emoji caret stops should include byte offset {expected}: {emoji_carets:?}"
        );
    }
    assert!(
        !emoji_carets
            .iter()
            .any(|offset| (1 + 1..1 + emoji.len()).contains(offset)),
        "an extended emoji grapheme must not expose interior caret offsets: {emoji_carets:?}"
    );

    let combining = "e\u{301}";
    let combining_layout = engine.layout_with_options(
        combining,
        &TextStyle::default(),
        layout_options().soft_wrap(false),
    );
    assert!(
        !combining_layout
            .line(0)
            .selection_spans(1, combining.len())
            .is_empty()
    );

    let combining_carets = caret_offsets(&combining_layout, 0);
    assert!(combining_carets.contains(&0));
    assert!(combining_carets.contains(&combining.len()));
    assert!(!combining_carets.contains(&1));

    let joiner_combining = "e\u{200d}\u{301}";
    let joiner_combining_layout = engine.layout_with_options(
        joiner_combining,
        &TextStyle::default(),
        layout_options().soft_wrap(false),
    );
    let line = joiner_combining_layout.line(0);
    let visible_cell = line.selection_spans(0, 1);
    assert_eq!(visible_cell.len(), 1);
    assert!(line.selection_spans(1, 4).is_empty());
    assert_eq!(
        line.selection_spans(4, joiner_combining.len()),
        visible_cell,
        "the combining component remains selectable across the same visible cell"
    );
    let joiner_combining_carets = caret_offsets(&joiner_combining_layout, 0);
    assert!(joiner_combining_carets.contains(&0));
    assert!(joiner_combining_carets.contains(&joiner_combining.len()));
    assert!(
        !joiner_combining_carets
            .iter()
            .any(|offset| (1..joiner_combining.len()).contains(offset))
    );
}

#[test]
fn hard_break_document_rebases_joiner_carets_and_selection_ranges() {
    let mut engine = TextEngine::new();
    let first = "header";
    let second = "👩\u{200d}💻 e\u{301}";
    let source = format!("{first}\n{second}");
    let style = TextStyle::default();
    let options = layout_options();

    let document = engine.layout_with_options(&source, &style, options);
    let standalone = engine.layout_with_options(second, &style, options);
    let second_start = first.len() + 1;

    assert_eq!(document.line_count(), 2);
    assert_eq!(document.line(1).start(), second_start);
    assert!(
        document
            .line(1)
            .selection_spans(second_start + 4, second_start + 7)
            .is_empty()
    );

    let document_carets = document.line_caret_positions(1);
    assert!(!document_carets.is_empty());
    for caret in standalone.line_caret_positions(0) {
        assert!(
            document_carets.iter().any(|document_caret| {
                document_caret.offset == caret.offset + second_start
                    && document_caret.affinity == caret.affinity
                    && document_caret.x.to_bits() == caret.x.to_bits()
                    && document_caret.line == 1
            }),
            "document caret {:?} was not rebased from standalone caret {:?}",
            document_carets,
            caret
        );
    }
}

#[test]
fn max_line_clipping_rebases_and_discards_cached_carets_for_removed_lines() {
    let mut engine = TextEngine::new();
    let source = "👩\u{200d}💻\nsecond";
    let options = layout_options()
        .max_lines(Some(1))
        .overflow(incular_text::TextOverflow::Clip);
    let clipped = engine.layout_with_options(source, &TextStyle::default(), options);

    assert_eq!(clipped.line_count(), 1);
    assert!(clipped.overflowed);
    assert!(clipped.line(0).selection_spans(4, 7).is_empty());

    let carets = clipped.line_caret_positions(0);
    assert!(carets.iter().any(|caret| caret.offset == 0));
    assert!(
        carets
            .iter()
            .any(|caret| caret.offset == "👩\u{200d}💻".len())
    );
    assert!(carets.iter().all(|caret| caret.line == 0));
    assert!(
        clipped.line_caret_positions(1).is_empty(),
        "cached caret positions for clipped lines must not remain addressable"
    );

    let empty =
        engine.layout_with_options(source, &TextStyle::default(), options.max_lines(Some(0)));
    assert!((empty.line_count() == 0));
    assert!(
        empty.line_caret_positions(0).is_empty(),
        "a zero-line clip must not expose cached caret positions"
    );
}
