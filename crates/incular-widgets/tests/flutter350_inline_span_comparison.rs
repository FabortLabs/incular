use incular_core::{Color, Size};
use incular_text::{
    InlineSpanComparison as Impact, RichText, TextSpan, TextStyle, WidgetSpanAlignment,
};
use incular_widgets::{RichParagraph, SizedBox, Text, Widget, WidgetSpan};

fn paragraph(child: impl Into<Widget>, alignment: WidgetSpanAlignment) -> RichParagraph {
    let span = TextSpan::new("a")
        .child(TextSpan::new("b"))
        .child(WidgetSpan::new(20).alignment(alignment))
        .child(TextSpan::new("c"));
    RichParagraph::new(RichText::new(span)).child(20, child)
}

#[test]
fn inline_widget_descriptor_replacement_and_alignment_changes_require_layout() {
    let first = paragraph(
        SizedBox::from_size(Size::new(10., 10.)),
        WidgetSpanAlignment::Baseline,
    );
    let equal = paragraph(
        SizedBox::from_size(Size::new(10., 10.)),
        WidgetSpanAlignment::Baseline,
    );
    let resized = paragraph(
        SizedBox::from_size(Size::new(11., 10.)),
        WidgetSpanAlignment::Baseline,
    );
    let literal = paragraph(Text::new("test"), WidgetSpanAlignment::Baseline);
    let different = paragraph(Text::new("different!"), WidgetSpanAlignment::Baseline);
    let top = paragraph(
        SizedBox::from_size(Size::new(10., 10.)),
        WidgetSpanAlignment::TextTop,
    );
    for span in [&first, &equal, &resized] {
        assert_eq!(span.compare_spans(span), Impact::Identical);
    }
    assert_eq!(first.compare_spans(&equal), Impact::Identical);
    for (a, b) in [
        (&first, &resized),
        (&first, &literal),
        (&equal, &resized),
        (&literal, &different),
        (&resized, &different),
        (&equal, &different),
        (&first, &different),
        (&first, &top),
    ] {
        assert_eq!(a.compare_spans(b), Impact::Layout);
    }
}

#[test]
fn changing_a_widget_grandchild_does_not_reuse_stale_outer_span_layout() {
    let nested = |height| {
        let inner = TextSpan::empty()
            .child(WidgetSpan::new(19))
            .child(TextSpan::new("The sky is falling :)"));
        paragraph(
            RichParagraph::new(RichText::new(inner))
                .child(19, SizedBox::from_size(Size::new(10., height))),
            WidgetSpanAlignment::Baseline,
        )
    };
    let a = nested(10.);
    let b = nested(11.);
    assert_eq!(a.compare_spans(&b), Impact::Layout);
    assert_eq!(a.compare_spans(&a), Impact::Identical);
    assert_eq!(b.compare_spans(&b), Impact::Identical);
}

#[test]
fn span_comparison_propagates_the_strongest_child_impact_without_shaping() {
    let text = |span| TextSpan::new("a").child(span);
    let base = text(TextSpan::new("b").style(TextStyle::new().font_size(14.).color(Color::BLACK)));
    let recolor =
        text(TextSpan::new("b").style(TextStyle::new().font_size(14.).color(Color::WHITE)));
    let resize =
        text(TextSpan::new("b").style(TextStyle::new().font_size(15.).color(Color::BLACK)));
    let metadata = text(
        TextSpan::new("b")
            .style(TextStyle::new().font_size(14.).color(Color::BLACK))
            .semantics_identifier("new id"),
    );
    assert_eq!(base.compare_to(&recolor), Impact::Paint);
    assert_eq!(base.compare_to(&resize), Impact::Layout);
    assert_eq!(base.compare_to(&metadata), Impact::Metadata);
    assert_eq!(base.compare_to(&base.clone()), Impact::Identical);
}
