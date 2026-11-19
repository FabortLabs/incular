use std::mem::size_of;

use incular_widgets::internal::{Element, WidgetTree};
use incular_widgets::{Column, Text, Widget};

#[test]
fn ordinary_elements_keep_cold_state_out_of_common_record() {
    let children = (0..1_000)
        .map(|index| {
            Widget::from(
                Text::new(format!("item {index}"))
                    .text_direction(incular_widgets::TextDirection::Ltr),
            )
        })
        .collect::<Vec<_>>();
    let mut tree = WidgetTree::new();
    tree.mount(Column::new(children).into())
        .expect("mount ordinary elements");

    assert_eq!(tree.element_count(), 1_001);
    #[cfg(all(target_pointer_width = "64", not(feature = "devtools")))]
    assert!(size_of::<Element>() <= 160);
    eprintln!(
        "element_size={} common_element_bytes={}",
        size_of::<Element>(),
        tree.element_count() * size_of::<Element>()
    );
}

#[test]
fn render_descriptors_keep_the_general_backdrop_filter_out_of_line() {
    assert!(size_of::<incular_widgets::internal::RenderKind>() <= 128);
}
