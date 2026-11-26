//! Basic single-child widgets in a zero-area Flutter 3.50 view.

use incular_config::Constraints;
use incular_core::Size;
use incular_runtime::Runtime;
use incular_widgets::{
    Builder, Center, Directionality, LimitedBox, Offstage, Padding, Placeholder, RawImage,
    TextDirection, Widget, internal::Key,
};

fn zero_area(child: Widget) {
    let key = Key::Value(350);
    let child = child.with_key(key.clone());
    let view = Directionality::new(TextDirection::Ltr, Center::new(child));
    let mut runtime = Runtime::new(view.into()).unwrap();
    runtime.run_frame(Constraints::tight(Size::ZERO)).unwrap();
    let tree = runtime.tree();
    let element = tree
        .element_with_key(&key)
        .expect("first frame mounts widget");
    assert_eq!(tree.element_bounds(element).unwrap().size, Size::ZERO);
}

#[test]
fn limited_box_under_zero_area_center() {
    zero_area(LimitedBox::new(Placeholder::new()).into());
}

#[test]
fn padding_larger_than_the_view_does_not_force_a_positive_size() {
    zero_area(Padding::all(5., Placeholder::new()).into());
}

#[test]
fn offstage_under_zero_area_center() {
    zero_area(Offstage::new(Placeholder::new()).into());
}

#[test]
fn raw_image_without_backing_image_under_zero_area_center() {
    zero_area(RawImage::new().into());
}

#[test]
fn builder_mounts_its_placeholder_in_a_zero_area_view() {
    zero_area(Builder::new(|_| Placeholder::new().into()).into());
}
