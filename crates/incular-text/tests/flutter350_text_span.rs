use incular_core::Size;
use incular_text::{
    InlineSpan, InlineSpanRef, InlineSpanSemanticsInformation, PlainTextOptions, TextAffinity,
    TextSpan, WidgetSpan,
};

#[test]
fn presentation_text_respects_placeholder_and_semantics_options_independently() {
    let defaults = PlainTextOptions::default();
    let abc = TextSpan::with_children("a", [TextSpan::new("b"), TextSpan::new("c")]);
    assert_eq!(abc.to_plain_text(defaults), "abc");
    let widget = || WidgetSpan::sized(10, Size::new(10., 10.));
    let with_widget = TextSpan::new("a")
        .child(TextSpan::new("b"))
        .child(widget())
        .child(TextSpan::new("c"));
    assert_eq!(with_widget.to_plain_text(defaults), "ab\u{fffc}c");
    let with_label = TextSpan::new("a")
        .child(TextSpan::new("b").semantics_label("foo"))
        .child(TextSpan::new("c"));
    assert_eq!(with_label.to_plain_text(defaults), "afooc");
    assert_eq!(
        with_label.to_plain_text(PlainTextOptions {
            include_semantics_labels: false,
            ..defaults
        }),
        "abc"
    );
    assert_eq!(with_label.plain_text(), "abc");
    assert_eq!(with_label.semantics_text().as_deref(), Some("afooc"));

    let combined = with_label.child(widget());
    assert_eq!(
        combined.to_plain_text(PlainTextOptions {
            include_placeholders: false,
            ..defaults
        }),
        "afooc"
    );
    assert_eq!(
        combined.to_plain_text(PlainTextOptions {
            include_semantics_labels: false,
            include_placeholders: false
        }),
        "abc"
    );
    assert_eq!(combined.plain_text(), "abc\u{fffc}");
}

fn positional_span(span: &TextSpan, offset: usize, affinity: TextAffinity) -> Option<&str> {
    match span.span_for_utf16_position(offset, affinity) {
        Some(InlineSpanRef::Text(text)) => Some(text.text.as_str()),
        Some(InlineSpanRef::Widget(_)) => Some("\u{fffc}"),
        None => None,
    }
}

#[test]
fn positions_skip_empty_wrappers_and_count_an_inline_widget_once() {
    use TextAffinity::Downstream as Down;
    let text = TextSpan::empty()
        .child(TextSpan::empty().child(TextSpan::new("a")))
        .child(TextSpan::new("b"))
        .child(TextSpan::new("c"));
    for (offset, expected) in [Some("a"), Some("b"), Some("c"), None]
        .into_iter()
        .enumerate()
    {
        assert_eq!(positional_span(&text, offset, Down), expected);
    }
    let with_widget = TextSpan::new("a")
        .child(TextSpan::new("b"))
        .child(WidgetSpan::new(42))
        .child(TextSpan::new("c"));
    for (offset, expected) in ["a", "b", "\u{fffc}", "c"].into_iter().enumerate() {
        assert_eq!(positional_span(&with_widget, offset, Down), Some(expected));
    }
    assert!(matches!(
        with_widget.span_for_utf16_position(2, Down),
        Some(InlineSpanRef::Widget(span)) if span.id == 42
    ));
}

#[test]
fn utf16_affinity_crosses_text_and_widget_boundaries_without_counting_utf8_bytes() {
    use TextAffinity::{Downstream as Down, Upstream as Up};
    let text = TextSpan::new("a🦀")
        .child(WidgetSpan::new(12))
        .child(TextSpan::new("c"));
    for (position, affinity, expected) in [
        (0, Down, Some("a🦀")),
        (0, Up, None),
        (2, Down, Some("a🦀")),
        (2, Up, Some("a🦀")),
        (3, Down, Some("\u{fffc}")),
        (3, Up, Some("a🦀")),
        (4, Down, Some("c")),
        (4, Up, None),
        (5, Down, None),
        (5, Up, Some("c")),
    ] {
        assert_eq!(positional_span(&text, position, affinity), expected);
    }
}

#[test]
fn semantics_records_preserve_literal_text_label_identifier_and_placeholder_order() {
    let span = TextSpan::new("aaa")
        .semantics_label("bbb")
        .semantics_identifier("ccc");
    let item = InlineSpanSemanticsInformation {
        text: "aaa",
        semantics_label: Some("bbb"),
        semantics_identifier: Some("ccc"),
        is_placeholder: false,
    };
    assert_eq!(span.semantics_information(), [item]);
    let combined = TextSpan::empty()
        .child(span)
        .child(WidgetSpan::new(1))
        .child(TextSpan::new("d"));
    assert_eq!(
        combined.semantics_information(),
        [
            item,
            InlineSpanSemanticsInformation {
                text: "\u{fffc}",
                semantics_label: None,
                semantics_identifier: None,
                is_placeholder: true,
            },
            InlineSpanSemanticsInformation {
                text: "d",
                semantics_label: None,
                semantics_identifier: None,
                is_placeholder: false,
            },
        ]
    );
}

fn child_names(root: &TextSpan) -> Vec<&str> {
    let mut texts = Vec::new();
    assert!(root.visit_direct_children(|span| {
        if let InlineSpan::Text(span) = span {
            texts.push(span.text.as_str());
        }
        true
    }));
    texts
}

fn preorder_index(root: &TextSpan, target: &TextSpan, cursor: &mut usize) -> bool {
    if std::ptr::eq(root, target) {
        return false;
    }
    *cursor += 1;
    root.visit_direct_children(|span| match span {
        InlineSpan::Text(span) => preorder_index(span, target, cursor),
        InlineSpan::Widget(_) => true,
    })
}

#[test]
fn direct_visitation_preserves_order_and_can_stop_a_recursive_walk() {
    let root = TextSpan::empty()
        .child(
            TextSpan::empty()
                .child(TextSpan::new("leaf1"))
                .child(TextSpan::new("leaf2")),
        )
        .child(TextSpan::new("branch2"));
    let [InlineSpan::Text(branch1), InlineSpan::Text(branch2)] = &root.children[..] else {
        panic!("expected two children")
    };
    let [InlineSpan::Text(leaf1), InlineSpan::Text(leaf2)] = &branch1.children[..] else {
        panic!("expected two grandchildren")
    };
    assert_eq!(child_names(&root), ["", "branch2"]);
    assert_eq!(child_names(branch1), ["leaf1", "leaf2"]);
    for span in [branch2, leaf1, leaf2] {
        assert!(child_names(span).is_empty());
    }
    let find = |target| {
        let mut cursor = 0;
        (!preorder_index(&root, target, &mut cursor)).then_some(cursor)
    };
    assert_eq!(
        [&root, branch1, leaf1, leaf2, branch2].map(find),
        [Some(0), Some(1), Some(2), Some(3), Some(4)]
    );
    assert_eq!(find(&TextSpan::new("foobar")), None);
}
