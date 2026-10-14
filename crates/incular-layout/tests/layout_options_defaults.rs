use incular_layout::{LayoutOptions, TextDirection, VerticalDirection};

#[test]
fn layout_options_default_preserves_directions() {
    let options = LayoutOptions::default();

    assert_eq!(options.text_direction, TextDirection::Ltr);
    assert_eq!(options.vertical_direction, VerticalDirection::Down);
}
