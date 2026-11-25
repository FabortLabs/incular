//! Retained baseline contract of Flutter 3.50.0 RenderFittedBox.

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{
    FittedBox, SizedBox, Text, TextStyle, Widget,
    internal::{Key, WidgetTree},
};

#[test]
fn fitted_box_reports_and_clears_the_real_untransformed_child_baseline() {
    let leaf_key = Key::from("fitted baseline leaf");
    let scene = |has_baseline| {
        let leaf = if has_baseline {
            Widget::from(
                Text::new("Ag")
                    .text_direction(TextDirection::Ltr)
                    .style(TextStyle::default().font_size(16.)),
            )
        } else {
            Widget::from(SizedBox::new().width(30.).height(20.))
        };
        Widget::from(FittedBox::new(leaf.with_key(leaf_key.clone())))
    };
    let mut tree = WidgetTree::new();
    let root = tree.mount(scene(true)).unwrap();
    let retained = tree.render_id(root).unwrap();
    let bounds = Constraints::tight(Size::new(200., 200.));
    for has_baseline in [true, false, true] {
        tree.update(root, scene(has_baseline)).unwrap();
        tree.layout(bounds).unwrap();
        assert_eq!(tree.render_id(root), Some(retained));
        let leaf = tree
            .render_id(tree.element_with_key(&leaf_key).unwrap())
            .unwrap();
        let baseline = tree.baseline(leaf);
        assert_eq!(baseline.is_some(), has_baseline);
        assert!(!tree.content_transform(retained).unwrap().is_translation());
        assert_eq!(tree.baseline(retained), baseline);
    }
}
