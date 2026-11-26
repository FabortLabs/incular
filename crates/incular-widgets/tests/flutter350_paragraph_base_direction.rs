use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_text::TextAlign;
use incular_widgets::{Directionality, Text, Widget, internal::WidgetTree};

const MIXED: &str = "אבג xyz";

fn scene(ambient: TextDirection, explicit: Option<TextDirection>) -> Widget {
    let mut label = Text::new(MIXED).align(TextAlign::Left);
    if let Some(direction) = explicit {
        label = label.text_direction(direction);
    }
    Directionality::new(ambient, Widget::from(label).with_key("mixed paragraph")).into()
}

fn visual_order(tree: &WidgetTree) -> (f32, f32) {
    let id = tree.element_with_key(&"mixed paragraph".into()).unwrap();
    let layout = tree.text_layout(tree.render_id(id).unwrap()).unwrap();
    let line = &layout.lines[0];
    // Hebrew uses two UTF-8 bytes per scalar; Flutter uses UTF-16 offsets.
    (
        line.selection_spans(0, 6)[0].0,
        line.selection_spans(7, 10)[0].0,
    )
}

#[test]
fn changing_the_ambient_base_direction_reshapes_a_retained_mixed_paragraph() {
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(TextDirection::Rtl, None)).unwrap();
    let constraints = Constraints::loose(Size::new(300., 100.));
    tree.layout(constraints).unwrap();
    let label = tree.element_with_key(&"mixed paragraph".into()).unwrap();
    let render = tree.render_id(label);
    let (hebrew, latin) = visual_order(&tree);
    assert!(latin < hebrew);

    tree.update(root, scene(TextDirection::Ltr, None)).unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(tree.element_with_key(&"mixed paragraph".into()), Some(label));
    assert_eq!(tree.render_id(label), render);
    let (hebrew, latin) = visual_order(&tree);
    assert!(hebrew < latin, "RTL shaping was retained in the LTR paragraph");

    tree.update(root, scene(TextDirection::Ltr, Some(TextDirection::Rtl))).unwrap();
    tree.layout(constraints).unwrap();
    let (hebrew, latin) = visual_order(&tree);
    assert!(latin < hebrew, "explicit direction must override the ambient one");
}
