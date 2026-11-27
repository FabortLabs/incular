use incular_config::Constraints;
use incular_core::Size;
use incular_rendering::DisplayList;
use incular_widgets::{
    Alignment, Center, CustomPaint, Directionality, OverflowBox, Text, TextDirection, Widget,
    internal::{Key, WidgetTree},
};
use std::time::Instant;

fn measure(max_height: Option<f32>) -> (f32, f32) {
    let paragraph_key = Key::from("paragraph");
    let measured_key = Key::from("baseline source");
    let paragraph = Widget::from(Text::new("Hello World")).with_key(paragraph_key.clone());
    let child = match max_height {
        Some(height) => Widget::from(
            OverflowBox::new(paragraph)
                .alignment(Alignment::TOP_LEFT)
                .max_height(height),
        )
        .with_key(measured_key.clone()),
        None => paragraph,
    };
    let root = Directionality::new(
        TextDirection::Ltr,
        Center::new(CustomPaint::new(Size::ZERO, DisplayList::default()).child(child)),
    );
    let mut tree = WidgetTree::new();
    tree.mount(root.into()).unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    tree.update_compositor(Instant::now()).unwrap();
    let _ = tree.paint();

    let render = |key| tree.render_id(tree.element_with_key(key).unwrap()).unwrap();
    let paragraph = render(&paragraph_key);
    let from = if max_height.is_some() {
        render(&measured_key)
    } else {
        paragraph
    };
    (
        tree.baseline(from).unwrap(),
        tree.render_size(paragraph).unwrap().height,
    )
}

#[test]
fn reducing_the_child_height_does_not_clamp_its_text_baseline() {
    let (normal_baseline, normal_height) = measure(None);
    let (overflow_baseline, child_height) = measure(Some(normal_height / 2.));
    assert!(normal_baseline < normal_height);
    assert_eq!(child_height, normal_height / 2.);
    assert_eq!(overflow_baseline, normal_baseline);
    assert!(overflow_baseline > child_height);
}
