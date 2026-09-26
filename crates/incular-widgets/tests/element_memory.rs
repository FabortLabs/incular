use std::mem::size_of;

use incular_widgets::internal::{Element, WidgetTree};
use incular_widgets::{Column, Text, Widget};

#[test]
fn ordinary_elements_do_not_need_scrolling_state() {
    let children = (0..1_000)
        .map(|index| Widget::from(Text::new(format!("item {index}"))))
        .collect::<Vec<_>>();
    let mut tree = WidgetTree::new();
    tree.mount(Column::new(children).into())
        .expect("mount ordinary elements");

    assert_eq!(tree.element_count(), 1_001);
    #[cfg(all(target_pointer_width = "64", not(feature = "devtools")))]
    assert!(size_of::<Element>() <= 320);
    eprintln!(
        "element_size={} common_element_bytes={}",
        size_of::<Element>(),
        tree.element_count() * size_of::<Element>()
    );
}
