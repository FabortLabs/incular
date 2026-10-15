//! `TextDocumentLayout` matches a whole-text `TextLayout` while reusing the
//! paragraphs around each edit.

use incular_text::{
    TextAlign, TextEditingController, TextEngine, TextGeometry, TextLayoutOptions, TextLineRef,
    TextOverflow, TextSelection, TextStyle,
};

fn paragraph(index: usize) -> String {
    format!("Paragraph {index}: the retained per-paragraph cache keeps warm documents cheap.")
}

fn document(paragraphs: usize) -> String {
    (0..paragraphs)
        .map(paragraph)
        .collect::<Vec<_>>()
        .join("\n")
}

fn options(width: Option<f32>) -> TextLayoutOptions {
    TextLayoutOptions::new(width, TextAlign::Start).overflow(TextOverflow::Visible)
}

/// Every line, caret stop and painted run of `document` matches `whole`.
fn assert_same_geometry(document: &impl TextGeometry, whole: &impl TextGeometry) {
    assert_eq!(document.line_count(), whole.line_count());
    assert_eq!(document.metrics(), whole.metrics());
    for (left, right) in document.lines().zip(whole.lines()) {
        assert_eq!(left.index(), right.index());
        assert_eq!(
            (left.start(), left.caret_end(), left.end()),
            (right.start(), right.caret_end(), right.end())
        );
        assert_eq!(left.baseline(), right.baseline());
        assert_eq!(
            (left.offset(), left.width()),
            (right.offset(), right.width())
        );
        assert_eq!(
            left.glyphs().collect::<Vec<_>>(),
            right.glyphs().collect::<Vec<_>>()
        );
        let spans = |line: TextLineRef<'_>| {
            line.clusters()
                .map(|span| (span.start, span.end, span.left, span.right))
                .collect::<Vec<_>>()
        };
        assert_eq!(spans(left), spans(right));
        let painted = |line: TextLineRef<'_>| {
            line.runs()
                .iter()
                .map(|run| line.paint_origin().y + run.origin.y)
                .collect::<Vec<_>>()
        };
        assert_eq!(painted(left), painted(right));
        assert_eq!(
            document.line_caret_positions(left.index()),
            whole.line_caret_positions(right.index())
        );
    }
}

#[test]
fn edited_documents_match_a_whole_text_layout() {
    let style = TextStyle::default();
    let base = document(120);
    let removed = base.find("\nParagraph 90:").expect("paragraph 90");
    let first_break = base.find('\n').expect("a document");
    let edits = [
        base.clone(),
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
        String::new(),
        "one paragraph".to_owned(),
        "mixed אבג direction\nשלום world".to_owned(),
    ];
    for width in [None, Some(240.)] {
        let mut engine = TextEngine::new();
        engine.layout_paragraphs(&base, &style, options(width));
        for edited in &edits {
            let document = engine.layout_paragraphs(edited, &style, options(width));
            let whole = TextEngine::new().layout_with_options(edited, &style, options(width));
            assert_same_geometry(&document, &whole);
        }
        assert!(engine.diagnostics().paragraphs_carried > 0);
    }
}

#[test]
fn an_edit_near_the_top_lays_out_only_the_edited_paragraph() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let doc = document(301);
    engine.layout_paragraphs(&doc, &style, options(Some(400.)));
    for edited in [
        format!("X{doc}"),
        format!("\nX{doc}"),
        format!("\n\nX{doc}"),
    ] {
        let before = engine.diagnostics();
        engine.layout_paragraphs(&edited, &style, options(Some(400.)));
        let after = engine.diagnostics();
        assert!(
            after.paragraphs_considered - before.paragraphs_considered <= 2,
            "an edit at the top lays out at most the edited paragraphs"
        );
        assert!(after.paragraphs_carried - before.paragraphs_carried >= 300);
    }
}

#[test]
fn an_unchanged_document_is_returned_whole() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let doc = document(50);
    let first = engine.layout_paragraphs(&doc, &style, options(None));
    let before = engine.diagnostics();
    let second = engine.layout_paragraphs(&doc, &style, options(None));
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert_eq!(
        engine.diagnostics().paragraphs_considered,
        before.paragraphs_considered
    );
    assert_eq!(first.paragraph_count(), 50);
}

#[test]
fn lines_are_found_by_index_offset_and_position() {
    let mut engine = TextEngine::new();
    let style = TextStyle::default();
    let doc = "first\nsecond\n\nfourth";
    let layout = engine.layout_paragraphs(doc, &style, options(None));
    assert_eq!(layout.line_count(), 4);
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
    assert!(layout.get_line(4).is_none());
    let fourth = layout.line(3);
    assert_eq!(fourth.glyphs().next().map(|glyph| glyph.cluster), Some(14));
    assert_eq!(fourth.shaped().start, 0);
    let height = layout.metrics.line_height;
    let visible: Vec<_> = layout
        .lines_between(height * 2.5, height * 2.6)
        .map(|line| line.index())
        .collect();
    assert!(visible.contains(&2) && visible.len() <= 3, "{visible:?}");
}

#[test]
fn arrow_keys_cross_paragraphs_in_a_document_layout() {
    let mut engine = TextEngine::new();
    let doc = "ab\ncd";
    let layout = engine.layout_paragraphs(doc, &TextStyle::default(), options(None));
    let controller = TextEditingController::with_text(doc);
    controller.set_selection(TextSelection::collapsed(2));
    controller.move_right_visual(&layout, false);
    assert_eq!(controller.value().selection.extent, 3);
    controller.move_left_visual(&layout, false);
    assert_eq!(controller.value().selection.extent, 2);
}
