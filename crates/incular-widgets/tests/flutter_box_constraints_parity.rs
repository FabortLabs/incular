//! Paired Flutter 3.50.0 rendering box constraint and geometry cases.

use incular_config::{Axis, Constraints};
use incular_core::{Lerp, Size};
use incular_widgets::{
    Column, Expanded, Row, SizedBox, TextDirection, UnconstrainedBox, Widget, internal::WidgetTree,
};

// rendering/box_constraints_test.dart:17. Compare finite bounds and value
// equality; Dart's optional named copy API and hash are not Rust contracts.
#[test]
fn replacing_all_finite_bounds_leaves_original_constraints_unchanged() {
    let original = Constraints::new(3., 7., 11., 17.);
    let copied = original
        .with_max_width(17.)
        .with_min_width(13.)
        .with_max_height(117.)
        .with_min_height(111.);
    assert_eq!(copied, Constraints::new(13., 17., 111., 117.));
    assert_eq!(original, Constraints::new(3., 7., 11., 17.));
    assert_ne!(copied, original);
}

// rendering/box_constraints_test.dart:60.
#[test]
fn lerp_interpolates_all_four_finite_bounds() {
    let start = Constraints::new(13., 17., 111., 117.);
    let end = Constraints::new(3., 7., 11., 17.);
    assert_eq!(start.lerp(&end, 0.2), Constraints::new(11., 15., 91., 97.));
    let half = Some(Constraints::new(1.5, 3.5, 5.5, 8.5));
    assert_eq!(Constraints::lerp_optional(None, Some(end), 0.5), half);
    assert_eq!(Constraints::lerp_optional(Some(end), None, 0.5), half);
    assert_eq!(Constraints::lerp_optional(None, None, 0.5), None);
}

// rendering/box_constraints_test.dart:134. Equalities also guarantee
// normalized bounds because the Rust type cannot hold a contradictory box.
#[test]
fn lerp_overshoot_normalizes_interpolated_bounds_instead_of_using_an_endpoint() {
    let start = Constraints::tight_for(None, Some(25.));
    let end = Constraints::tight_for(None, Some(0.));
    assert_eq!(start.lerp(&end, 1.2), end);

    let start = Constraints::tight(Size::new(10., 10.));
    let end = Constraints::loose(Size::new(100., 100.));
    assert_eq!(
        start.lerp(&end, -0.1),
        Constraints::tight(Size::new(11., 11.)),
    );
    let empty = Some(Constraints::tight(Size::ZERO));
    assert_eq!(Constraints::lerp_optional(None, Some(start), -0.5), empty);
    assert_eq!(Constraints::lerp_optional(Some(start), None, 1.5), empty);
}

#[test]
fn matched_unbounded_maxima_survive_normalization() {
    let a = Constraints::tight_for(Some(10.), None);
    let b = Constraints::tight_for(Some(20.), None);
    assert_eq!(a.lerp(&b, 0.5), Constraints::tight_for(Some(15.), None));
    assert_eq!(a.lerp(&b, -2.), Constraints::tight_for(Some(0.), None));
    let bounded = Constraints::tight(Size::new(10., 10.));
    assert_eq!(a.lerp(&bounded, 0.25), a);
    assert_eq!(a.lerp(&bounded, 0.75), bounded);
    assert_eq!(bounded.lerp(&a, 0.25), bounded);
    assert_eq!(bounded.lerp(&a, 0.75), a);
}

fn laid_out_root_size(widget: impl Into<Widget>, constraints: Constraints) -> Size {
    let mut tree = WidgetTree::new();
    let root = tree.mount(widget.into()).expect("mount");
    tree.layout(constraints).expect("layout");
    tree.element_bounds(root).expect("root bounds").size
}

// rendering/box_test.dart:755. Keep finite width for the horizontal flex
// while its child is free to use 200 on the vertical axis.
#[test]
fn unconstrained_box_honors_horizontal_constrained_axis() {
    let flex = Row::new([Expanded::new(
        SizedBox::new().width(f32::INFINITY).height(200.),
    )])
    .text_direction(TextDirection::Ltr);
    let widget = UnconstrainedBox::new(flex).constrained_axis(Axis::Horizontal);
    assert_eq!(
        laid_out_root_size(widget, Constraints::at_most(100., f32::INFINITY)),
        Size::new(100., 200.),
    );
}

// rendering/box_test.dart:776. The transpose uses a vertical tight flex
// child with unbounded width on the parent constraint.
#[test]
fn unconstrained_box_honors_vertical_constrained_axis() {
    let flex = Column::new([Expanded::new(
        SizedBox::new().width(200.).height(f32::INFINITY),
    )])
    .text_direction(TextDirection::Ltr);
    let widget = UnconstrainedBox::new(flex).constrained_axis(Axis::Vertical);
    assert_eq!(
        laid_out_root_size(widget, Constraints::at_most(f32::INFINITY, 100.)),
        Size::new(200., 100.),
    );
}
