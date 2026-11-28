use std::sync::Arc;

use incular_gestures::{MouseCursor, RawPointerEvent};
use incular_text::{TextSpan, TextSpanMouse, TextStyle};

#[test]
fn text_spans_compare_values_recursively_and_keep_callback_identity() {
    let a1 = TextSpan::new("a");
    let a2 = TextSpan::new("a");
    let b1 = TextSpan::empty().child(a1.clone());
    let b2 = TextSpan::empty().child(a2.clone());
    let c1 = TextSpan::empty();
    let c2 = TextSpan::empty();
    assert_eq!(a1, a2);
    assert_eq!(b1, b2);
    assert_eq!(c1, c2);
    for (first, other) in [
        (&a1, &b2),
        (&b1, &c2),
        (&c1, &a2),
        (&a1, &c2),
        (&b1, &a2),
        (&c1, &b2),
    ] {
        assert_ne!(first, other);
    }

    let callback1: Arc<dyn Fn(RawPointerEvent) + Send + Sync> = Arc::new(|_| {});
    let callback2: Arc<dyn Fn(RawPointerEvent) + Send + Sync> = Arc::new(|_| {});
    let span = |cursor, callback| {
        let mut span = TextSpan::new("a");
        span.mouse = Some(Arc::new(TextSpanMouse {
            cursor,
            on_enter: Some(callback),
            on_exit: None,
        }));
        span
    };
    let d1 = span(MouseCursor::Defer, callback1.clone());
    let d2 = span(MouseCursor::Defer, callback1);
    let d3 = span(MouseCursor::Defer, callback2.clone());
    let e1 = span(MouseCursor::NotAllowed, callback2.clone());
    let e2 = span(MouseCursor::NotAllowed, callback2);
    assert_ne!(a1, d1);
    assert_eq!(d1, d2);
    assert_ne!(d2, d3);
    assert_ne!(d3, e1);
    assert_eq!(e1, e2);
}

#[test]
fn multiline_span_diagnostics_include_style_values_and_empty_children() {
    let span = TextSpan::new("a")
        .style(TextStyle::default().size(10.))
        .children([
            TextSpan::new("b").child(TextSpan::empty()),
            TextSpan::new("c"),
        ]);
    assert_eq!(
        span.to_string_deep(),
        concat!(
            "TextSpan:\n",
            "  inherit: true\n",
            "  size: 10.0\n",
            "  \"a\"\n",
            "  TextSpan:\n",
            "    \"b\"\n",
            "    TextSpan:\n",
            "      (empty)\n",
            "  TextSpan:\n",
            "    \"c\"\n",
        )
    );
}

#[test]
fn mouse_diagnostics_report_only_the_callbacks_and_cursor_enabled_on_the_span() {
    let plain = TextSpan::new("a");
    assert_eq!(plain.to_string_deep(), "TextSpan:\n  \"a\"\n");
    let mouse = plain
        .on_enter(|_| {})
        .on_exit(|_| {})
        .mouse_cursor(MouseCursor::NotAllowed);
    assert_eq!(
        mouse.to_string_deep(),
        concat!(
            "TextSpan:\n",
            "  \"a\"\n",
            "  callbacks: enter, exit\n",
            "  mouseCursor: SystemMouseCursor(forbidden)\n",
        )
    );
}
