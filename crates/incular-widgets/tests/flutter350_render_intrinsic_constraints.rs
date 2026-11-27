use incular_config::{Axis, Constraints};
use incular_core::Size;
use incular_widgets::internal::{Key, WidgetTree};
use incular_widgets::{IntrinsicHeight, IntrinsicWidth, MaxIntrinsicSize, SizedBox, Widget};

const PARENT: Key = Key::Value(1);
const PROXY: Key = Key::Value(2);
const CHILD: Key = Key::Value(3);
const MAXIMUM: Size = Size {
    width: 100.,
    height: 200.,
};
const ORDINARY: Size = Size {
    width: 55.,
    height: 110.,
};

fn keyed(widget: impl Into<Widget>, key: Key) -> Widget {
    widget.into().with_key(key)
}

fn leaf() -> Widget {
    keyed(
        SizedBox::builder()
            .width(ORDINARY.width)
            .height(ORDINARY.height)
            .build(),
        CHILD,
    )
}

fn measured_widget(axis: Axis, maximum: Size) -> Widget {
    let child = keyed(MaxIntrinsicSize::new(maximum, leaf()), PROXY);
    match axis {
        Axis::Horizontal => keyed(IntrinsicWidth::new(child), PARENT),
        Axis::Vertical => keyed(IntrinsicHeight::new(child), PARENT),
    }
}

fn size(tree: &WidgetTree, key: &Key) -> Size {
    let element = tree.element_with_key(key).unwrap();
    tree.element_bounds(element).unwrap().size
}

fn expect_sizes(axis: Axis, constraints: Constraints, expected: Size) {
    let mut tree = WidgetTree::new();
    tree.mount(measured_widget(axis, MAXIMUM)).unwrap();
    tree.layout(constraints).unwrap();
    for key in [&PARENT, &PROXY, &CHILD] {
        assert_eq!(size(&tree, key), expected);
    }
}

#[test]
fn intrinsic_width_clamps_child_maximum_under_loose_constraints() {
    expect_sizes(
        Axis::Horizontal,
        Constraints::new(50., 70., 8., 800.),
        Size::new(70., 110.),
    );
}

#[test]
fn intrinsic_width_keeps_tight_larger_constraint() {
    expect_sizes(
        Axis::Horizontal,
        Constraints::new(500., 500., 8., 800.),
        Size::new(500., 110.),
    );
}

#[test]
fn intrinsic_width_keeps_tight_smaller_constraint() {
    expect_sizes(
        Axis::Horizontal,
        Constraints::new(50., 50., 8., 800.),
        Size::new(50., 110.),
    );
}

#[test]
fn intrinsic_height_clamps_child_maximum_under_loose_constraints() {
    expect_sizes(
        Axis::Vertical,
        Constraints::new(5., 500., 8., 80.),
        Size::new(55., 80.),
    );
}

#[test]
fn intrinsic_height_keeps_tight_larger_constraint() {
    expect_sizes(
        Axis::Vertical,
        Constraints::new(5., 500., 400., 400.),
        Size::new(55., 400.),
    );
}

#[test]
fn intrinsic_height_keeps_tight_smaller_constraint() {
    expect_sizes(
        Axis::Vertical,
        Constraints::new(5., 500., 80., 80.),
        Size::new(55., 80.),
    );
}

#[test]
fn override_does_not_change_ordinary_layout() {
    let mut tree = WidgetTree::new();
    tree.mount(keyed(MaxIntrinsicSize::new(MAXIMUM, leaf()), PROXY))
        .unwrap();
    tree.layout(Constraints::at_most(500., 500.)).unwrap();
    assert_eq!(size(&tree, &PROXY), ORDINARY);
    assert_eq!(size(&tree, &CHILD), ORDINARY);
}

#[test]
fn updating_the_maximum_recomputes_the_parent_without_remounting() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(measured_widget(Axis::Horizontal, MAXIMUM))
        .unwrap();
    let constraints = Constraints::at_most(500., 500.);
    tree.layout(constraints).unwrap();
    assert_eq!(size(&tree, &PARENT), Size::new(100., 110.));
    let child = tree.element_with_key(&CHILD).unwrap();
    let render = tree.render_id(child).unwrap();

    tree.update(
        root,
        measured_widget(Axis::Horizontal, Size::new(80., 200.)),
    )
    .unwrap();
    tree.layout(constraints).unwrap();
    assert_eq!(size(&tree, &PARENT), Size::new(80., 110.));
    assert_eq!(size(&tree, &CHILD), Size::new(80., 110.));
    assert_eq!(tree.element_with_key(&CHILD), Some(child));
    assert_eq!(tree.render_id(child), Some(render));
}
