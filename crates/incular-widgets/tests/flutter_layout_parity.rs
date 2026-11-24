//! Flutter 3.50.0 box layout regressions, with the paired geometry oracles in
//! audits/flutter-3.50.0-parity/loose-layout-comparisons.md.

use incular_config::{Alignment, Constraints};
use incular_core::{Offset, Size};
use incular_widgets::{
    Align, Baseline, FittedBox, OverflowBox, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

fn loose_geometry(
    build: impl FnOnce(Widget) -> Widget,
    child_size: Size,
    max: Size,
) -> (Size, Offset) {
    let parent_key = Key::String("layout-parent".into());
    let child_key = Key::String("layout-child".into());
    let child = Widget::from(
        SizedBox::new()
            .width(child_size.width)
            .height(child_size.height),
    )
    .with_key(child_key.clone());
    let parent = build(child).with_key(parent_key.clone());

    // A tight outer box and an Align reproduce the loose child constraints of
    // the paired Flutter fixture, rather than laying the subject out as root.
    let mut tree = WidgetTree::new();
    tree.mount(Align::new(Alignment::TOP_LEFT, parent).into())
        .expect("mount layout fixture");
    tree.layout(Constraints::tight(max))
        .expect("layout fixture");
    let _ = tree.paint();

    let parent = tree
        .element_bounds(tree.element_with_key(&parent_key).expect("parent key"))
        .expect("parent bounds");
    let child = tree
        .element_bounds(tree.element_with_key(&child_key).expect("child key"))
        .expect("child bounds");
    (parent.size, child.origin - parent.origin)
}

#[test]
fn default_overflow_box_fills_a_loose_bounded_parent_and_centers_its_child() {
    assert_eq!(
        loose_geometry(
            |child| OverflowBox::new(child).into(),
            Size::new(20., 10.),
            Size::new(100., 80.),
        ),
        (Size::new(100., 80.), Offset::new(40., 35.)),
    );
}

#[test]
fn default_fitted_box_preserves_child_aspect_ratio_under_loose_constraints() {
    let (size, _) = loose_geometry(
        |child| FittedBox::new(child).into(),
        Size::new(200., 100.),
        Size::new(100., 100.),
    );
    assert_eq!(size, Size::new(100., 50.));
}

#[test]
fn baseline_can_place_a_child_without_a_baseline_above_its_parent() {
    assert_eq!(
        loose_geometry(
            |child| Baseline::new(5., child).into(),
            Size::new(20., 20.),
            Size::new(100., 80.),
        ),
        (Size::new(20., 5.), Offset::new(0., -15.)),
    );
}
