//! Flutter 3.50.0 neutral FractionallySizedBox and OverflowBox geometry.

use incular_config::{Alignment, Constraints};
use incular_core::{Offset, Rect, Size};
use incular_runtime::Runtime;
use incular_widgets::{
    Align, Center, Container, Directionality, FractionallySizedBox, OverflowBox, Placeholder,
    SizedBox, TextDirection, Widget, internal::Key,
};

const SUBJECT: u64 = 1;

fn bounds(scene: impl Into<Widget>, viewport: Size) -> Rect {
    let mut runtime = Runtime::new(scene.into()).expect("mount");
    runtime
        .run_frame(Constraints::tight(viewport))
        .expect("frame");
    let tree = runtime.tree();
    let subject = tree
        .element_with_key(&Key::from(SUBJECT))
        .expect("fixture key");
    tree.element_bounds(subject).expect("subject bounds")
}

fn viewport() -> Size {
    Size::new(800., 600.)
}

// fractionally_sized_box_test.dart:9, "FractionallySizedBox".
#[test]
fn fractional_child_centers_inside_the_overflow_box_limits() {
    let child = Widget::from(Container::new()).with_key(SUBJECT);
    let fractional = FractionallySizedBox::new(child)
        .width_factor(0.5)
        .height_factor(0.25);
    let scene = OverflowBox::new(Center::new(fractional))
        .min_width(0.)
        .max_width(100.)
        .min_height(0.)
        .max_height(100.)
        .alignment(Alignment::TOP_LEFT);
    assert_eq!(
        bounds(scene, viewport()),
        Rect::from_origin_size(Offset::new(25., 37.5), Size::new(50., 25.)),
    );
}

// fractionally_sized_box_test.dart:32, "FractionallySizedBox alignment".
#[test]
fn fractional_top_right_alignment_is_absolute_in_rtl() {
    let child = Widget::from(Placeholder::new()).with_key(SUBJECT);
    let scene = Directionality::new(
        TextDirection::Rtl,
        FractionallySizedBox::new(child)
            .width_factor(0.5)
            .height_factor(0.5)
            .alignment(Alignment::TOP_RIGHT),
    );
    let child = bounds(scene, viewport());
    assert_eq!(child.size, Size::new(400., 300.));
    assert_eq!(
        child.origin + Offset::new(200., 150.),
        Offset::new(600., 150.)
    );
}

// fractionally_sized_box_test.dart:100, "FractionallySizedBox does not crash at zero area".
#[test]
fn fractional_box_without_factors_completes_a_zero_area_frame() {
    let fractional = Widget::from(FractionallySizedBox::new(Placeholder::new())).with_key(SUBJECT);
    let scene = Directionality::new(TextDirection::Ltr, Center::new(fractional));
    assert_eq!(bounds(scene, Size::ZERO).size, Size::ZERO);
}

// overflow_box_test.dart:10, "OverflowBox control test".
#[test]
fn tight_small_overflow_box_centers_its_larger_child() {
    let child = Widget::from(Container::new()).with_key(SUBJECT);
    let overflow = OverflowBox::new(child)
        .min_width(0.)
        .max_width(100.)
        .min_height(0.)
        .max_height(50.);
    let scene = Align::new(
        Alignment::BOTTOM_RIGHT,
        SizedBox::new().width(10.).height(20.).child(overflow),
    );
    assert_eq!(
        bounds(scene, viewport()),
        Rect::from_origin_size(Offset::new(745., 565.), Size::new(100., 50.)),
    );
}
