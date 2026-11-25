//! Natural-extent lists fill the first paint window without building offscreen rows.

use std::{cell::Cell, rc::Rc};

use incular_config::{Constraints, TextDirection};
use incular_core::Size;
use incular_widgets::{
    Directionality, ListView, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

#[test]
fn short_natural_rows_fill_the_first_frame_without_building_past_its_edge() {
    let built = Rc::new(Cell::new(0));
    let list = ListView::builder(1_000_000, {
        let built = built.clone();
        move |i| {
            built.set(built.get() + 1);
            Widget::from(SizedBox::new().height(12.3)).with_key(Key::from(i as u64))
        }
    })
    .cache_extent(0.);
    let mut tree = WidgetTree::new();
    tree.mount(Directionality::new(TextDirection::Ltr, list).into())
        .unwrap();
    tree.layout(Constraints::tight(Size::new(800., 600.)))
        .unwrap();
    for index in 0..49 {
        let id = tree.element_with_key(&Key::from(index as u64)).unwrap();
        let rect = tree.element_bounds(id).unwrap();
        assert!((rect.origin.y - index as f32 * 12.3).abs() < 0.001);
        assert!(rect.origin.y < 600.);
        if index == 48 {
            assert!(rect.origin.y + rect.size.height > 600.);
        }
    }
    assert!(tree.element_with_key(&Key::from(49_u64)).is_none());
    assert_eq!(built.get(), 49, "only the intersecting rows are built");
}
