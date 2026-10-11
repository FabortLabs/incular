use incular::prelude::*;

#[test]
fn public_text_and_rich_text_overflow_configuration_mounts_retained_widgets() {
    let text: Widget = Text::new("one two three four five")
        .text_direction(incular_widgets::TextDirection::Ltr)
        .max_lines(Some(1))
        .overflow(TextOverflow::Ellipsis)
        .soft_wrap(true)
        .into();
    let rich: Widget = RichText::new(TextSpan::new("עברית mixed text"))
        .max_lines(Some(1))
        .overflow(TextOverflow::Clip)
        .into();

    let mut tree = incular::widgets::internal::WidgetTree::new();
    // RichText resolves its paragraph direction from the ambient
    // Directionality, just as Flutter's RichText does. Keep the explicit
    // direction on `text` above to exercise that property independently.
    tree.mount(
        Directionality::new(
            TextDirection::Ltr,
            Column::new(Vec::<Widget>::from([text, rich])),
        )
        .into(),
    )
    .unwrap();
    tree.layout(incular::config::Constraints::tight(Size::new(80., 60.)))
        .expect("layout");
}
