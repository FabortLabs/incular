use incular_core::{BuildContext, Color};
use incular_text::{TextScaler, TextStyle};
use incular_widgets::internal::{RenderKind, render_kind};
use incular_widgets::{Text, Widget};
use std::sync::Arc;

#[test]
fn plain_labels_share_their_style_and_overrides_keep_their_values() {
    let context = BuildContext::new();
    let resolve = |label: Text, context: &BuildContext| {
        let widget: Widget = label.into();
        let RenderKind::Text { style, .. } = render_kind(&widget, context) else {
            panic!("expected text");
        };
        style
    };
    let plain = Text::new("plain");
    let first = resolve(plain.clone(), &context);
    for i in 0..100 {
        let next = resolve(Text::new(format!("row {i}")), &context);
        assert!(Arc::ptr_eq(&first, &next));
    }
    let scaled = resolve(plain.clone().text_scaler(TextScaler::linear(2.)), &context);
    assert_eq!(*scaled, TextStyle::default().scaled(TextScaler::linear(2.)));
    let ambient = TextStyle::default().font_size(30.).color(Color::BLACK);
    let inherited = resolve(plain.clone(), &context.provide(ambient.clone()));
    assert_eq!(*inherited, ambient.merge(&TextStyle::default()));
    let colored = resolve(plain.color(Color::BLACK), &context);
    assert_eq!(colored.color, Color::BLACK);
    assert_eq!(*first, TextStyle::default());
}
