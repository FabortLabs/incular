//! With one constrained axis, FittedBox lays out before applying its paint fit.

use incular_config::Constraints;
use incular_core::{Offset, Rect, Size};
use incular_widgets::{
    BoxFit, Center, FittedBox, SizedBox, Widget,
    internal::{Key, WidgetTree},
};

const FIT: Key = Key::Value(9180);
const LEAF: Key = Key::Value(9181);
const VIEW: Size = Size {
    width: 800.,
    height: 600.,
};

fn scene(fit: BoxFit, size: Size) -> Widget {
    let leaf = Widget::from(SizedBox::from_size(size)).with_key(LEAF);
    let fitted = Widget::from(FittedBox::new(leaf).fit(fit)).with_key(FIT);
    Center::new(SizedBox::new().width(200.).child(fitted)).into()
}

fn size(tree: &WidgetTree, key: &Key) -> Size {
    let element = tree.element_with_key(key).unwrap();
    tree.render_size(tree.render_id(element).unwrap()).unwrap()
}

fn world_box(tree: &WidgetTree, key: &Key) -> Rect {
    tree.element_bounds(tree.element_with_key(key).unwrap())
        .unwrap()
}

/// widgets/fitted_box_test.dart:10.
#[test]
fn fitted_box_sizes_a_single_constrained_axis_without_distorting_the_ratio() {
    let mut tree = WidgetTree::new();
    tree.mount(scene(BoxFit::Contain, Size::new(100., 50.)))
        .unwrap();
    tree.layout(Constraints::tight(VIEW)).unwrap();
    assert_eq!(size(&tree, &FIT), Size::new(200., 100.));
    assert_eq!(size(&tree, &LEAF), Size::new(100., 50.));
    let fitted = world_box(&tree, &FIT);
    let leaf = world_box(&tree, &LEAF);
    let bottom_right = |rect: Rect| rect.origin + Offset::new(rect.size.width, rect.size.height);
    assert_eq!(bottom_right(fitted), Offset::new(500., 350.));
    assert_eq!(bottom_right(leaf), bottom_right(fitted));
}

/// widgets/fitted_box_test.dart:434.
#[test]
fn scale_down_uses_intrinsic_size_or_preserves_ratio_under_the_maximum() {
    let mut tree = WidgetTree::new();
    let root = tree
        .mount(scene(BoxFit::ScaleDown, Size::new(100., 50.)))
        .unwrap();
    for (leaf, fit, offset) in [
        (
            Size::new(100., 50.),
            Size::new(200., 50.),
            Offset::new(50., 0.),
        ),
        (Size::new(400., 200.), Size::new(200., 100.), Offset::ZERO),
    ] {
        tree.update(root, scene(BoxFit::ScaleDown, leaf)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(size(&tree, &FIT), fit);
        assert_eq!(
            world_box(&tree, &LEAF).origin - world_box(&tree, &FIT).origin,
            offset
        );
    }
}

/// widgets/fitted_box_test.dart:487.
#[test]
fn changing_scale_down_relayouts_in_place() {
    let mut tree = WidgetTree::new();
    let child = Size::new(100., 50.);
    let root = tree.mount(scene(BoxFit::ScaleDown, child)).unwrap();
    let fit_id = tree
        .render_id(tree.element_with_key(&FIT).unwrap())
        .unwrap();
    for (fit, height) in [
        (BoxFit::ScaleDown, 50.),
        (BoxFit::Contain, 100.),
        (BoxFit::ScaleDown, 50.),
    ] {
        tree.update(root, scene(fit, child)).unwrap();
        tree.layout(Constraints::tight(VIEW)).unwrap();
        assert_eq!(
            tree.render_id(tree.element_with_key(&FIT).unwrap()),
            Some(fit_id)
        );
        assert_eq!(size(&tree, &FIT).height, height);
    }
}
