//! Flutter 3.50.0 defaults for IndexedStack (W04).

use incular_config::{Alignment, Constraints};
use incular_core::{Offset, Size};
use incular_widgets::{
    Align, Directionality, IndexedStack, SizedBox, TextDirection, Widget,
    internal::{Key, WidgetTree},
};

fn selected_child_origin(direction: TextDirection) -> Offset {
    let stack_key = Key::String("stack".into());
    let selected_key = Key::String("selected".into());
    let narrower =
        Widget::from(SizedBox::new().width(20.).height(10.)).with_key(selected_key.clone());
    let wider = Widget::from(SizedBox::new().width(80.).height(30.));
    let stack =
        Widget::from(IndexedStack::new([wider, narrower]).index(1)).with_key(stack_key.clone());
    let scene = Directionality::new(direction, Align::new(Alignment::TOP_LEFT, stack));
    let mut tree = WidgetTree::new();
    tree.mount(scene.into()).expect("mount fixture");
    tree.layout(Constraints::tight(Size::new(100., 80.)))
        .expect("layout fixture");

    let bounds = |key| {
        tree.element_bounds(tree.element_with_key(key).expect("fixture key"))
            .expect("fixture bounds")
    };
    let stack = bounds(&stack_key);
    let selected = bounds(&selected_key);
    // The unselected wider child still determines the stack's loose size.
    assert_eq!(stack.size, Size::new(80., 30.));
    assert_eq!(selected.size, Size::new(20., 10.));
    selected.origin - stack.origin
}

#[test]
fn indexed_stack_default_aligns_selected_child_at_top_start() {
    assert_eq!(selected_child_origin(TextDirection::Ltr), Offset::ZERO);
    // Flutter's default AlignmentDirectional.topStart resolves to top-right.
    assert_eq!(
        selected_child_origin(TextDirection::Rtl),
        Offset::new(60., 0.),
    );
}
